//! Bounded interpreter for the supported TRION TIR v1 subset.
//!
//! This interpreter executes TIR records on the host CPU. It is not a native
//! ternary processor, a native executable loader, or a kernel execution path.

#![forbid(unsafe_code)]

use core::fmt;
use trion_ternary::Ternary;

const TIR_HEADER: &str = "TRION-TIR 1 x86_64";
const MAX_TIR_BYTES: usize = 1_048_576;
const MAX_TIR_INSTRUCTIONS: usize = 65_536;
const STACK_REGISTER: usize = 4;

/// Errors from TIR validation or bounded execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    InputTooLarge,
    InvalidUtf8,
    MissingFinalNewline,
    InvalidHeader,
    EmptyProgram,
    TooManyInstructions,
    AllocationFailure,
    InvalidRecord { line: usize },
    InvalidAddress { line: usize },
    InvalidRegister { line: usize },
    InvalidImmediate { line: usize },
    UnsupportedOperation { line: usize },
    StepLimitExceeded,
    StackAddressBeforeMemory,
    StackAddressOutOfBounds,
    StackPointerOverflow,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTooLarge => f.write_str("TIR input exceeds the one-megabyte limit"),
            Self::InvalidUtf8 => f.write_str("TIR input is not valid UTF-8"),
            Self::MissingFinalNewline => f.write_str("TIR must end with a newline"),
            Self::InvalidHeader => f.write_str("unsupported TIR header or version"),
            Self::EmptyProgram => f.write_str("TIR contains no instructions"),
            Self::TooManyInstructions => f.write_str("TIR exceeds the instruction-count limit"),
            Self::AllocationFailure => f.write_str("unable to allocate TIR instruction buffer"),
            Self::InvalidRecord { line } => write!(f, "invalid TIR record on line {line}"),
            Self::InvalidAddress { line } => write!(f, "invalid TIR address on line {line}"),
            Self::InvalidRegister { line } => write!(f, "invalid TIR register on line {line}"),
            Self::InvalidImmediate { line } => {
                write!(f, "invalid TIR immediate on line {line}")
            }
            Self::UnsupportedOperation { line } => {
                write!(f, "unsupported TIR operation on line {line}")
            }
            Self::StepLimitExceeded => f.write_str("TIR execution instruction limit exceeded"),
            Self::StackAddressBeforeMemory => {
                f.write_str("RSP points before the caller-provided memory region")
            }
            Self::StackAddressOutOfBounds => {
                f.write_str("RET return-address read is outside caller-provided memory")
            }
            Self::StackPointerOverflow => f.write_str("RSP overflow while executing RET"),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Mutable architectural register state and the read-only memory region used by RET.
#[derive(Debug)]
pub struct MachineState<'a> {
    /// Modeled x86-64 general-purpose registers in the translator's register order.
    pub registers: [u64; 16],
    /// Guest virtual address corresponding to memory[0].
    pub memory_base: u64,
    /// Read-only guest memory. RET reads its return address from this region.
    pub memory: &'a [u8],
}

/// How the linear TIR stream stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionOutcome {
    /// RET completed and returned the address read from the modeled stack.
    Returned {
        target: u64,
        instructions_executed: usize,
    },
    /// The stream ended without a RET instruction.
    Completed { instructions_executed: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Nop,
    MovImm64 { register: usize, value: u64 },
    Ret,
}

/// Validate and execute TIR v1 with a strict instruction budget.
///
/// The complete stream is parsed before any register changes are committed.
/// On any error, `state.registers` remains unchanged.
pub fn execute_tir(
    bytes: &[u8],
    state: &mut MachineState<'_>,
    max_instructions: usize,
) -> Result<ExecutionOutcome, RuntimeError> {
    let program = parse_tir(bytes)?;
    let mut registers = state.registers;
    let mut executed = 0usize;

    for operation in program {
        if executed >= max_instructions {
            return Err(RuntimeError::StepLimitExceeded);
        }

        executed = executed
            .checked_add(1)
            .ok_or(RuntimeError::TooManyInstructions)?;

        match operation {
            Operation::Nop => {}
            Operation::MovImm64 { register, value } => {
                let destination = registers
                    .get_mut(register)
                    .ok_or(RuntimeError::InvalidRegister { line: 0 })?;
                *destination = value;
            }
            Operation::Ret => {
                let rsp = registers[STACK_REGISTER];
                let offset = rsp
                    .checked_sub(state.memory_base)
                    .ok_or(RuntimeError::StackAddressBeforeMemory)?;
                let offset =
                    usize::try_from(offset).map_err(|_| RuntimeError::StackAddressOutOfBounds)?;
                let end = offset
                    .checked_add(8)
                    .ok_or(RuntimeError::StackAddressOutOfBounds)?;
                let return_bytes = state
                    .memory
                    .get(offset..end)
                    .ok_or(RuntimeError::StackAddressOutOfBounds)?;
                let mut encoded = [0u8; 8];
                encoded.copy_from_slice(return_bytes);
                let target = u64::from_le_bytes(encoded);
                registers[STACK_REGISTER] = rsp
                    .checked_add(8)
                    .ok_or(RuntimeError::StackPointerOverflow)?;
                state.registers = registers;
                return Ok(ExecutionOutcome::Returned {
                    target,
                    instructions_executed: executed,
                });
            }
        }
    }

    state.registers = registers;
    Ok(ExecutionOutcome::Completed {
        instructions_executed: executed,
    })
}

/// Validate a complete TIR v1 stream without executing it.
///
/// Returns the number of validated instructions. The same parser and limits
/// are used by the interpreter and executable-container loader.
pub fn validate_tir(bytes: &[u8]) -> Result<usize, RuntimeError> {
    Ok(parse_tir(bytes)?.len())
}

fn parse_tir(bytes: &[u8]) -> Result<Vec<Operation>, RuntimeError> {
    if bytes.len() > MAX_TIR_BYTES {
        return Err(RuntimeError::InputTooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| RuntimeError::InvalidUtf8)?;
    if !text.ends_with('\n') {
        return Err(RuntimeError::MissingFinalNewline);
    }

    let mut lines = text.split_terminator('\n');
    if lines.next() != Some(TIR_HEADER) {
        return Err(RuntimeError::InvalidHeader);
    }

    let mut operations = Vec::new();
    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        if operations.len() >= MAX_TIR_INSTRUCTIONS {
            return Err(RuntimeError::TooManyInstructions);
        }

        let mut fields = line.split(' ');
        let address = fields
            .next()
            .ok_or(RuntimeError::InvalidRecord { line: line_number })?;
        if address.len() != 16
            || !address
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            || u64::from_str_radix(address, 16).is_err()
        {
            return Err(RuntimeError::InvalidAddress { line: line_number });
        }

        let mnemonic = fields
            .next()
            .ok_or(RuntimeError::InvalidRecord { line: line_number })?;
        let operation = match mnemonic {
            "NOP" => {
                if fields.next().is_some() {
                    return Err(RuntimeError::InvalidRecord { line: line_number });
                }
                Operation::Nop
            }
            "RET" => {
                if fields.next().is_some() {
                    return Err(RuntimeError::InvalidRecord { line: line_number });
                }
                Operation::Ret
            }
            "MOV_IMM64" => {
                let register_text = fields
                    .next()
                    .ok_or(RuntimeError::InvalidRecord { line: line_number })?;
                let immediate_text = fields
                    .next()
                    .ok_or(RuntimeError::InvalidRecord { line: line_number })?;
                if fields.next().is_some() {
                    return Err(RuntimeError::InvalidRecord { line: line_number });
                }

                let register = register_text
                    .strip_prefix('r')
                    .and_then(|value| value.parse::<usize>().ok())
                    .filter(|value| *value < 16)
                    .ok_or(RuntimeError::InvalidRegister { line: line_number })?;
                if register_text != format!("r{register}") {
                    return Err(RuntimeError::InvalidRegister { line: line_number });
                }

                let immediate = Ternary::parse(immediate_text)
                    .map_err(|_| RuntimeError::InvalidImmediate { line: line_number })?;
                let unsigned = immediate
                    .to_u128()
                    .map_err(|_| RuntimeError::InvalidImmediate { line: line_number })?;
                let value = u64::try_from(unsigned)
                    .map_err(|_| RuntimeError::InvalidImmediate { line: line_number })?;
                if immediate.to_string() != immediate_text {
                    return Err(RuntimeError::InvalidImmediate { line: line_number });
                }
                Operation::MovImm64 { register, value }
            }
            _ => return Err(RuntimeError::UnsupportedOperation { line: line_number }),
        };

        if operations.try_reserve(1).is_err() {
            return Err(RuntimeError::AllocationFailure);
        }
        operations.push(operation);
    }

    if operations.is_empty() {
        return Err(RuntimeError::EmptyProgram);
    }
    Ok(operations)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RETURN_TARGET: u64 = 0x1234_5678_9abc_def0;

    fn stack() -> [u8; 8] {
        RETURN_TARGET.to_le_bytes()
    }

    fn state<'a>(memory: &'a [u8]) -> MachineState<'a> {
        let mut registers = [0u64; 16];
        registers[STACK_REGISTER] = 0x8000;
        MachineState {
            registers,
            memory_base: 0x8000,
            memory,
        }
    }

    #[test]
    fn executes_nop_mov_and_ret() {
        let input = b"TRION-TIR 1 x86_64\n0000000000400000 NOP\n0000000000400001 MOV_IMM64 r0 1TTT0\n000000000040000b RET\n";
        let memory = stack();
        let mut machine = state(&memory);

        let result = execute_tir(input, &mut machine, 10).unwrap();

        assert_eq!(
            result,
            ExecutionOutcome::Returned {
                target: RETURN_TARGET,
                instructions_executed: 3
            }
        );
        assert_eq!(machine.registers[0], 42);
        assert_eq!(machine.registers[STACK_REGISTER], 0x8008);
    }

    #[test]
    fn mov_to_rsp_changes_where_ret_reads() {
        let input = b"TRION-TIR 1 x86_64\n0000000000400000 MOV_IMM64 r4 10\n0000000000400001 RET\n";
        let memory = [0u8, 0, 0, 0x78, 0x56, 0x34, 0x12, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut machine = state(&memory);
        machine.memory_base = 0;

        let result = execute_tir(input, &mut machine, 10).unwrap();

        assert_eq!(
            result,
            ExecutionOutcome::Returned {
                target: 0x1234_5678,
                instructions_executed: 2
            }
        );
        assert_eq!(machine.registers[STACK_REGISTER], 11);
    }

    #[test]
    fn rejects_unsupported_or_malformed_tir_before_mutating_registers() {
        let input = b"TRION-TIR 1 x86_64\n0000000000400000 ADD r0 r1\n";
        let memory = stack();
        let mut machine = state(&memory);
        machine.registers[0] = 99;
        let before = machine.registers;

        assert!(matches!(
            execute_tir(input, &mut machine, 10),
            Err(RuntimeError::UnsupportedOperation { .. })
        ));
        assert_eq!(machine.registers, before);
    }

    #[test]
    fn instruction_limit_is_enforced_without_partial_commit() {
        let input =
            b"TRION-TIR 1 x86_64\n0000000000400000 MOV_IMM64 r0 1TTT0\n0000000000400001 RET\n";
        let memory = stack();
        let mut machine = state(&memory);
        let before = machine.registers;

        assert_eq!(
            execute_tir(input, &mut machine, 1),
            Err(RuntimeError::StepLimitExceeded)
        );
        assert_eq!(machine.registers, before);
    }

    #[test]
    fn rejects_stack_out_of_bounds_without_partial_commit() {
        let input =
            b"TRION-TIR 1 x86_64\n0000000000400000 MOV_IMM64 r0 1TTT0\n0000000000400001 RET\n";
        let memory = [0u8; 4];
        let mut machine = state(&memory);
        let before = machine.registers;

        assert_eq!(
            execute_tir(input, &mut machine, 10),
            Err(RuntimeError::StackAddressOutOfBounds)
        );
        assert_eq!(machine.registers, before);
    }

    #[test]
    fn rejects_rsp_underflow_and_overflow() {
        let input = b"TRION-TIR 1 x86_64\n0000000000400000 RET\n";
        let memory = stack();
        let mut machine = state(&memory);
        machine.registers[STACK_REGISTER] = 0x7fff;
        assert_eq!(
            execute_tir(input, &mut machine, 1),
            Err(RuntimeError::StackAddressBeforeMemory)
        );

        machine.registers[STACK_REGISTER] = u64::MAX - 3;
        machine.memory_base = u64::MAX - 3;
        assert_eq!(
            execute_tir(input, &mut machine, 1),
            Err(RuntimeError::StackPointerOverflow)
        );
    }

    #[test]
    fn rejects_bad_headers_noncanonical_immediates_and_missing_newline() {
        let memory = stack();
        let mut machine = state(&memory);

        assert_eq!(
            execute_tir(
                b"TRION-TIR 2 x86_64\n0000000000000000 NOP\n",
                &mut machine,
                2
            ),
            Err(RuntimeError::InvalidHeader)
        );
        assert_eq!(
            execute_tir(b"TRION-TIR 1 x86_64\n0000000000000000 NOP", &mut machine, 2),
            Err(RuntimeError::MissingFinalNewline)
        );
        assert!(matches!(
            execute_tir(
                b"TRION-TIR 1 x86_64\n0000000000000000 MOV_IMM64 r0 T\n",
                &mut machine,
                2
            ),
            Err(RuntimeError::InvalidImmediate { .. })
        ));
    }

    #[test]
    fn completes_a_program_without_ret() {
        let input = b"TRION-TIR 1 x86_64\n0000000000400000 NOP\n";
        let memory = stack();
        let mut machine = state(&memory);
        let result = execute_tir(input, &mut machine, 1).unwrap();
        assert_eq!(
            result,
            ExecutionOutcome::Completed {
                instructions_executed: 1
            }
        );
    }
    #[test]
    fn parser_rejects_deterministic_arbitrary_bytes_without_panicking() {
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        for length in 0..256usize {
            let mut bytes = Vec::with_capacity(length);
            for _ in 0..length {
                seed ^= seed << 7;
                seed ^= seed >> 9;
                seed ^= seed << 8;
                bytes.push(seed as u8);
            }
            let result = std::panic::catch_unwind(|| validate_tir(&bytes));
            assert!(result.is_ok(), "parser panicked for {length} input bytes");
        }
    }
}
