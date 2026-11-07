//! Bounded interpreter for TRION TIR v1.
//!
//! The runtime executes the explicitly supported TIR subset on the host CPU.
//! It is a user-space interpreter, not native ternary hardware or a kernel loader.
#![forbid(unsafe_code)]

use core::fmt;
use std::collections::BTreeMap;
use trion_ternary::Ternary;

const TIR_HEADER: &str = "TRION-TIR 1 x86_64";
const MAX_TIR_BYTES: usize = 1_048_576;
const MAX_TIR_INSTRUCTIONS: usize = 65_536;
const STACK_REGISTER: usize = 4;

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
    DuplicateAddress { line: usize },
    InvalidRegister { line: usize },
    InvalidImmediate { line: usize },
    InvalidTarget { line: usize },
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
            Self::DuplicateAddress { line } => write!(f, "duplicate TIR address on line {line}"),
            Self::InvalidRegister { line } => write!(f, "invalid TIR register on line {line}"),
            Self::InvalidImmediate { line } => write!(f, "invalid TIR immediate on line {line}"),
            Self::InvalidTarget { line } => write!(
                f,
                "branch target does not name an instruction on line {line}"
            ),
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

#[derive(Debug)]
pub struct MachineState<'a> {
    pub registers: [u64; 16],
    pub memory_base: u64,
    pub memory: &'a [u8],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Returned {
        target: u64,
        instructions_executed: usize,
    },
    Completed {
        instructions_executed: usize,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Nop,
    MovImm64 { dst: usize, value: u64 },
    Mov { dst: usize, src: usize },
    Add { dst: usize, src: usize },
    Sub { dst: usize, src: usize },
    Cmp { lhs: usize, rhs: usize },
    Jmp { target: u64 },
    Jz { target: u64 },
    Jnz { target: u64 },
    Jl { target: u64 },
    Jle { target: u64 },
    Jg { target: u64 },
    Jge { target: u64 },
    Call { target: u64 },
    Ret,
}
#[derive(Clone, Copy, Debug)]
struct Instruction {
    address: u64,
    operation: Operation,
    line: usize,
}
#[derive(Clone, Copy, Debug, Default)]
struct Flags {
    zero: bool,
    sign: bool,
    overflow: bool,
}

fn parse_register(text: &str, line: usize) -> Result<usize, RuntimeError> {
    let value = text
        .strip_prefix('r')
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|v| *v < 16)
        .ok_or(RuntimeError::InvalidRegister { line })?;
    if text != format!("r{value}") {
        return Err(RuntimeError::InvalidRegister { line });
    }
    Ok(value)
}
fn parse_address(text: &str, line: usize) -> Result<u64, RuntimeError> {
    if text.len() != 16
        || !text
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(RuntimeError::InvalidAddress { line });
    }
    u64::from_str_radix(text, 16).map_err(|_| RuntimeError::InvalidAddress { line })
}
fn parse_target(text: &str, line: usize) -> Result<u64, RuntimeError> {
    if text.len() != 16
        || !text
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(RuntimeError::InvalidTarget { line });
    }
    u64::from_str_radix(text, 16).map_err(|_| RuntimeError::InvalidTarget { line })
}
fn parse_program(bytes: &[u8]) -> Result<Vec<Instruction>, RuntimeError> {
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
    let mut program = Vec::new();
    let mut seen = BTreeMap::new();
    for (index, line_text) in lines.enumerate() {
        let line = index + 2;
        if program.len() >= MAX_TIR_INSTRUCTIONS {
            return Err(RuntimeError::TooManyInstructions);
        }
        let fields: Vec<&str> = line_text.split(' ').collect();
        if fields.iter().any(|f| f.is_empty()) || fields.len() < 2 {
            return Err(RuntimeError::InvalidRecord { line });
        }
        let address = parse_address(fields[0], line)?;
        if seen.insert(address, program.len()).is_some() {
            return Err(RuntimeError::DuplicateAddress { line });
        }
        let op = match fields.as_slice() {
            [_, "NOP"] => Operation::Nop,
            [_, "RET"] => Operation::Ret,
            [_, "MOV_IMM64", dst, imm] => {
                let dst = parse_register(dst, line)?;
                let value =
                    Ternary::parse(imm).map_err(|_| RuntimeError::InvalidImmediate { line })?;
                if value.to_string() != *imm {
                    return Err(RuntimeError::InvalidImmediate { line });
                }
                let n = value
                    .to_u128()
                    .map_err(|_| RuntimeError::InvalidImmediate { line })?;
                let value =
                    u64::try_from(n).map_err(|_| RuntimeError::InvalidImmediate { line })?;
                Operation::MovImm64 { dst, value }
            }
            [_, "MOV", dst, src] => Operation::Mov {
                dst: parse_register(dst, line)?,
                src: parse_register(src, line)?,
            },
            [_, "ADD", dst, src] => Operation::Add {
                dst: parse_register(dst, line)?,
                src: parse_register(src, line)?,
            },
            [_, "SUB", dst, src] => Operation::Sub {
                dst: parse_register(dst, line)?,
                src: parse_register(src, line)?,
            },
            [_, "CMP", lhs, rhs] => Operation::Cmp {
                lhs: parse_register(lhs, line)?,
                rhs: parse_register(rhs, line)?,
            },
            [_, "JMP", target] => Operation::Jmp {
                target: parse_target(target, line)?,
            },
            [_, "JZ", target] => Operation::Jz {
                target: parse_target(target, line)?,
            },
            [_, "JNZ", target] => Operation::Jnz {
                target: parse_target(target, line)?,
            },
            [_, "JL", target] => Operation::Jl {
                target: parse_target(target, line)?,
            },
            [_, "JLE", target] => Operation::Jle {
                target: parse_target(target, line)?,
            },
            [_, "JG", target] => Operation::Jg {
                target: parse_target(target, line)?,
            },
            [_, "JGE", target] => Operation::Jge {
                target: parse_target(target, line)?,
            },
            [_, "CALL", target] => Operation::Call {
                target: parse_target(target, line)?,
            },
            _ => return Err(RuntimeError::UnsupportedOperation { line }),
        };
        program
            .try_reserve(1)
            .map_err(|_| RuntimeError::AllocationFailure)?;
        program.push(Instruction {
            address,
            operation: op,
            line,
        });
    }
    if program.is_empty() {
        return Err(RuntimeError::EmptyProgram);
    }
    for instruction in &program {
        let target = match instruction.operation {
            Operation::Jmp { target }
            | Operation::Jz { target }
            | Operation::Jnz { target }
            | Operation::Jl { target }
            | Operation::Jle { target }
            | Operation::Jg { target }
            | Operation::Jge { target }
            | Operation::Call { target } => Some(target),
            _ => None,
        };
        if let Some(target) = target
            && !seen.contains_key(&target)
        {
            return Err(RuntimeError::InvalidTarget {
                line: instruction.line,
            });
        }
    }
    Ok(program)
}

pub fn validate_tir(bytes: &[u8]) -> Result<usize, RuntimeError> {
    Ok(parse_program(bytes)?.len())
}

pub fn execute_tir(
    bytes: &[u8],
    state: &mut MachineState<'_>,
    max_instructions: usize,
) -> Result<ExecutionOutcome, RuntimeError> {
    let program = parse_program(bytes)?;
    let addresses: BTreeMap<u64, usize> = program
        .iter()
        .enumerate()
        .map(|(i, x)| (x.address, i))
        .collect();
    let mut regs = state.registers;
    let mut flags = Flags::default();
    let mut calls: Vec<usize> = Vec::new();
    let mut pc = 0usize;
    let mut executed = 0usize;
    while pc < program.len() {
        if executed >= max_instructions {
            return Err(RuntimeError::StepLimitExceeded);
        }
        executed += 1;
        let instruction = program[pc];
        let next = pc + 1;
        let mut next_pc = next;
        match instruction.operation {
            Operation::Nop => {}
            Operation::MovImm64 { dst, value } => regs[dst] = value,
            Operation::Mov { dst, src } => regs[dst] = regs[src],
            Operation::Add { dst, src } => {
                let a = regs[dst];
                let b = regs[src];
                let (v, carry) = a.overflowing_add(b);
                flags = Flags {
                    zero: v == 0,
                    sign: (v >> 63) != 0,
                    overflow: ((!(a ^ b) & (a ^ v)) >> 63) != 0,
                };
                regs[dst] = v;
                let _ = carry;
            }
            Operation::Sub { dst, src } => {
                let a = regs[dst];
                let b = regs[src];
                let (v, _borrow) = a.overflowing_sub(b);
                flags = Flags {
                    zero: v == 0,
                    sign: (v >> 63) != 0,
                    overflow: (((a ^ b) & (a ^ v)) >> 63) != 0,
                };
                regs[dst] = v;
            }
            Operation::Cmp { lhs, rhs } => {
                let a = regs[lhs];
                let b = regs[rhs];
                let v = a.wrapping_sub(b);
                flags = Flags {
                    zero: a == b,
                    sign: (v >> 63) != 0,
                    overflow: (((a ^ b) & (a ^ v)) >> 63) != 0,
                };
            }
            Operation::Jmp { target } => next_pc = addresses[&target],
            Operation::Jz { target } if flags.zero => next_pc = addresses[&target],
            Operation::Jnz { target } if !flags.zero => next_pc = addresses[&target],
            Operation::Jl { target } if flags.sign != flags.overflow => {
                next_pc = addresses[&target]
            }
            Operation::Jle { target } if flags.zero || flags.sign != flags.overflow => {
                next_pc = addresses[&target]
            }
            Operation::Jg { target } if !flags.zero && flags.sign == flags.overflow => {
                next_pc = addresses[&target]
            }
            Operation::Jge { target } if flags.sign == flags.overflow => {
                next_pc = addresses[&target]
            }
            Operation::Jz { .. }
            | Operation::Jnz { .. }
            | Operation::Jl { .. }
            | Operation::Jle { .. }
            | Operation::Jg { .. }
            | Operation::Jge { .. } => {}
            Operation::Call { target } => {
                calls
                    .try_reserve(1)
                    .map_err(|_| RuntimeError::AllocationFailure)?;
                calls.push(next);
                next_pc = addresses[&target];
            }
            Operation::Ret if !calls.is_empty() => next_pc = calls.pop().expect("checked nonempty"),
            Operation::Ret => {
                let rsp = regs[STACK_REGISTER];
                let offset = rsp
                    .checked_sub(state.memory_base)
                    .ok_or(RuntimeError::StackAddressBeforeMemory)?;
                let offset =
                    usize::try_from(offset).map_err(|_| RuntimeError::StackAddressOutOfBounds)?;
                let end = offset
                    .checked_add(8)
                    .ok_or(RuntimeError::StackAddressOutOfBounds)?;
                let slice = state
                    .memory
                    .get(offset..end)
                    .ok_or(RuntimeError::StackAddressOutOfBounds)?;
                let mut raw = [0u8; 8];
                raw.copy_from_slice(slice);
                let target = u64::from_le_bytes(raw);
                regs[STACK_REGISTER] = rsp
                    .checked_add(8)
                    .ok_or(RuntimeError::StackPointerOverflow)?;
                state.registers = regs;
                return Ok(ExecutionOutcome::Returned {
                    target,
                    instructions_executed: executed,
                });
            }
        }
        pc = next_pc;
    }
    state.registers = regs;
    Ok(ExecutionOutcome::Completed {
        instructions_executed: executed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn machine<'a>(memory: &'a [u8]) -> MachineState<'a> {
        let mut registers = [0u64; 16];
        registers[STACK_REGISTER] = 0x8000;
        MachineState {
            registers,
            memory_base: 0x8000,
            memory,
        }
    }
    #[test]
    fn arithmetic_and_conditional_branch_execute() {
        let tir=b"TRION-TIR 1 x86_64\n0000000000000000 MOV_IMM64 r0 1TTT0\n0000000000000001 MOV_IMM64 r1 1TTT0\n0000000000000002 CMP r0 r1\n0000000000000003 JZ 0000000000000005\n0000000000000004 MOV_IMM64 r2 1\n0000000000000005 ADD r0 r1\n";
        let mut s = machine(&[]);
        let out = execute_tir(tir, &mut s, 20).unwrap();
        assert_eq!(
            out,
            ExecutionOutcome::Completed {
                instructions_executed: 5
            }
        );
        assert_eq!(s.registers[0], 84);
        assert_eq!(s.registers[2], 0);
    }
    #[test]
    fn call_and_return_execute() {
        let tir=b"TRION-TIR 1 x86_64\n0000000000000000 CALL 0000000000000002\n0000000000000001 JMP 0000000000000004\n0000000000000002 MOV_IMM64 r0 1TTT0\n0000000000000003 RET\n0000000000000004 NOP\n";
        let mut s = machine(&[]);
        let out = execute_tir(tir, &mut s, 20).unwrap();
        assert_eq!(
            out,
            ExecutionOutcome::Completed {
                instructions_executed: 5
            }
        );
        assert_eq!(s.registers[0], 42);
    }
    #[test]
    fn rejects_missing_branch_target_and_duplicate_addresses() {
        let mut s = machine(&[]);
        assert!(matches!(
            execute_tir(
                b"TRION-TIR 1 x86_64\n0000000000000000 JMP 0000000000000001\n",
                &mut s,
                4
            ),
            Err(RuntimeError::InvalidTarget { .. })
        ));
        assert!(matches!(
            execute_tir(
                b"TRION-TIR 1 x86_64\n0000000000000000 NOP\n0000000000000000 RET\n",
                &mut s,
                4
            ),
            Err(RuntimeError::DuplicateAddress { .. })
        ));
    }
    #[test]
    fn step_limit_stops_infinite_loop() {
        let tir = b"TRION-TIR 1 x86_64\n0000000000000000 JMP 0000000000000000\n";
        let mut s = machine(&[]);
        assert_eq!(
            execute_tir(tir, &mut s, 3),
            Err(RuntimeError::StepLimitExceeded)
        );
    }
    #[test]
    fn malformed_input_does_not_mutate_registers() {
        let mut s = machine(&[]);
        let before = s.registers;
        assert!(execute_tir(b"bad", &mut s, 2).is_err());
        assert_eq!(s.registers, before);
    }
}
