//! Format-neutral direct image preparation. This produces a checked mapping plan;
//! it does not map pages into the kernel or enter guest code.

use crate::{Error, Format, InertImage, Description};
use crate::guest::Permissions;

pub const DEFAULT_STACK_SIZE: u64 = 8 * 1024 * 1024;
pub const DEFAULT_GUEST_BUDGET: u64 = 10_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedRegion {
    pub address: u64,
    pub memory_size: u64,
    pub permissions: Permissions,
    /// File-backed bytes; the remainder of memory_size is zero-filled by the backend.
    pub initial_bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadPlan {
    pub format: Format,
    pub architecture: crate::Architecture,
    pub image_base: u64,
    pub entry_point: u64,
    pub stack_size: u64,
    pub instruction_budget: u64,
    pub dependencies: Vec<String>,
    pub regions: Vec<PlannedRegion>,
}

fn validate_region(
    address: u64,
    memory_size: u64,
    file_size: usize,
    permissions: Permissions,
) -> Result<(), Error> {
    permissions.validate()?;
    if memory_size == 0 || file_size as u64 > memory_size {
        return Err(Error::Invalid("image region size"));
    }
    address.checked_add(memory_size).ok_or(Error::Overflow)?;
    Ok(())
}

fn check_entry(regions: &[PlannedRegion], entry: u64) -> Result<(), Error> {
    if regions.iter().any(|r| {
        r.permissions.execute
            && entry >= r.address
            && entry < r.address.saturating_add(r.memory_size)
    }) {
        Ok(())
    } else {
        Err(Error::Invalid("entry point is not inside an executable image region"))
    }
}

/// Prepare a PE32+ image without translating it. If it imports symbols, every
/// import must resolve against the caller-supplied, policy-controlled export table.
pub fn prepare_pe32_plus(
    input: &[u8],
    load_base: Option<u64>,
    exports: Option<&crate::windows::ExportTable>,
) -> Result<LoadPlan, Error> {
    let mut image = crate::pe::map_preferred(input)?;
    let preferred = image.description.preferred_base;
    let base = load_base.unwrap_or(preferred);
    if base != preferred {
        crate::loader::rebase_pe32_plus(&mut image, base)?;
    }

    let imports = if image.description.dependencies.is_empty() {
        Vec::new()
    } else {
        let table = exports.ok_or(Error::Unsupported("PE imports require an explicit trusted export table"))?;
        crate::loader::resolve_pe32_plus_imports(&mut image, table)?
    };
    let _ = imports;

    let mut regions = Vec::new();
    let pe_offset = u32::from_le_bytes(input.get(0x3c..0x40)
        .ok_or(Error::Truncated("PE e_lfanew"))?.try_into()
        .map_err(|_| Error::Truncated("PE e_lfanew"))?) as usize;
    let optional = pe_offset.checked_add(24).ok_or(Error::Overflow)?;
    let headers_size = u32::from_le_bytes(input.get(optional + 60..optional + 64)
        .ok_or(Error::Truncated("PE SizeOfHeaders"))?.try_into()
        .map_err(|_| Error::Truncated("PE SizeOfHeaders"))?) as usize;
    let header_bytes = image.bytes.get(..headers_size)
        .ok_or(Error::Truncated("mapped PE headers"))?.to_vec();
    if headers_size > 0 {
        regions.push(PlannedRegion {
            address: base,
            memory_size: headers_size as u64,
            permissions: Permissions::READ,
            initial_bytes: header_bytes,
        });
    }
    for section in &image.description.sections {
        if section.memory_size == 0 { continue; }
        let permissions = Permissions {
            read: section.readable,
            write: section.writable,
            execute: section.executable,
        };
        permissions.validate()?;
        let start = usize::try_from(section.address).map_err(|_| Error::Overflow)?;
        let file_len = usize::try_from(section.file_size).map_err(|_| Error::Overflow)?;
        let end = start.checked_add(file_len).ok_or(Error::Overflow)?;
        let initial_bytes = image.bytes.get(start..end)
            .ok_or(Error::Truncated("mapped PE section"))?.to_vec();
        let address = base.checked_add(section.address).ok_or(Error::Overflow)?;
        validate_region(address, section.memory_size, initial_bytes.len(), permissions)?;
        regions.push(PlannedRegion { address, memory_size: section.memory_size, permissions, initial_bytes });
    }
    let entry = image.description.entry_point;
    check_entry(&regions, entry)?;
    Ok(LoadPlan {
        format: Format::Pe32Plus,
        architecture: image.description.architecture,
        image_base: base,
        entry_point: entry,
        stack_size: DEFAULT_STACK_SIZE,
        instruction_budget: DEFAULT_GUEST_BUDGET,
        dependencies: image.description.dependencies,
        regions,
    })
}

/// Prepare a dependency-free Mach-O 64 image. dyld-dependent images are refused
/// until a versioned, brokered dependency/fixup runtime is supplied.
fn validate_dependency_free_macho(input: &[u8]) -> Result<(), Error> {
    let read_u32 = |offset: usize| -> Result<u32, Error> {
        let end = offset.checked_add(4).ok_or(Error::Overflow)?;
        let bytes = input.get(offset..end).ok_or(Error::Truncated("Mach-O load command"))?;
        Ok(u32::from_le_bytes(bytes.try_into().map_err(|_| Error::Truncated("Mach-O load command"))?))
    };
    if input.len() < 32 { return Err(Error::Truncated("Mach-O header")); }
    let count = read_u32(16)? as usize;
    let command_bytes = read_u32(20)? as usize;
    if count > crate::MAX_COMMANDS { return Err(Error::Limit("Mach-O load command count")); }
    let end = 32usize.checked_add(command_bytes).ok_or(Error::Overflow)?;
    if end > input.len() { return Err(Error::Truncated("Mach-O load commands")); }
    let mut at = 32usize;
    for _ in 0..count {
        let command = read_u32(at)?;
        let size = read_u32(at.checked_add(4).ok_or(Error::Overflow)?)? as usize;
        if size < 8 || at.checked_add(size).ok_or(Error::Overflow)? > end {
            return Err(Error::Invalid("Mach-O load command bounds"));
        }
        match command {
            0x19 | 0x8000_0028 | 0x1b | 0x32 | 0x24 | 0x2a | 0x26 | 0x29 => {},
            0x0c | 0x8000_0018 | 0x8000_001f | 0x8000_0023 =>
                return Err(Error::Unsupported("Mach-O dylib dependency requires dyld")),
            0x22 | 0x8000_0022 | 0x8000_0034 | 0x8000_0033 =>
                return Err(Error::Unsupported("Mach-O dyld rebase/bind/fixup data is unsupported")),
            0x1d => return Err(Error::Unsupported("Mach-O code signature requires explicit verification policy")),
            _ => return Err(Error::Unsupported("Mach-O load command outside dependency-free profile")),
        }
        at = at.checked_add(size).ok_or(Error::Overflow)?;
    }
    if at != end { return Err(Error::Invalid("Mach-O sizeofcmds mismatch")); }
    Ok(())
}

pub fn prepare_macho64(input: &[u8], load_base: Option<u64>) -> Result<LoadPlan, Error> {
    validate_dependency_free_macho(input)?;
    let description = crate::macho::inspect(input)?;
    if !description.dependencies.is_empty() {
        return Err(Error::Unsupported("Mach-O dependencies require the dyld/framework runtime"));
    }
    let base = load_base.unwrap_or(description.preferred_base);
    let slide = base as i128 - description.preferred_base as i128;
    let mut regions = Vec::new();
    for segment in &description.sections {
        if segment.memory_size == 0 { continue; }
        let permissions = Permissions {
            read: segment.readable,
            write: segment.writable,
            execute: segment.executable,
        };
        permissions.validate()?;
        let start = usize::try_from(segment.file_offset).map_err(|_| Error::Overflow)?;
        let len = usize::try_from(segment.file_size).map_err(|_| Error::Overflow)?;
        let end = start.checked_add(len).ok_or(Error::Overflow)?;
        let initial_bytes = input.get(start..end)
            .ok_or(Error::Truncated("Mach-O segment bytes"))?.to_vec();
        let address = if slide >= 0 {
            segment.address.checked_add(slide as u64)
        } else {
            segment.address.checked_sub((-slide) as u64)
        }.ok_or(Error::Overflow)?;
        validate_region(address, segment.memory_size, initial_bytes.len(), permissions)?;
        regions.push(PlannedRegion {
            address,
            memory_size: segment.memory_size,
            permissions,
            initial_bytes,
        });
    }
    let entry = if slide >= 0 {
        description.entry_point.checked_add(slide as u64)
    } else {
        description.entry_point.checked_sub((-slide) as u64)
    }.ok_or(Error::Overflow)?;
    check_entry(&regions, entry)?;
    Ok(LoadPlan {
        format: Format::MachO64,
        architecture: description.architecture,
        image_base: base,
        entry_point: entry,
        stack_size: DEFAULT_STACK_SIZE,
        instruction_budget: DEFAULT_GUEST_BUDGET,
        dependencies: description.dependencies,
        regions,
    })
}

pub fn prepare(input: &[u8]) -> Result<LoadPlan, Error> {
    let description: Description = crate::inspect(input)?;
    match description.format {
        Format::Pe32Plus => prepare_pe32_plus(input, None, None),
        Format::MachO64 => prepare_macho64(input, None),
    }
}

/// Materialize a checked load plan into the bounded guest-memory model.
/// This remains an isolated model until the kernel execution backend is wired.
pub fn materialize(plan: &LoadPlan, process_id: u64) -> Result<crate::guest::GuestProcess, Error> {
    let mut process = crate::guest::GuestProcess::new(process_id, plan.instruction_budget)?;
    for region in &plan.regions {
        let rounded = region.memory_size.checked_add(crate::guest::PAGE_SIZE - 1)
            .ok_or(Error::Overflow)? / crate::guest::PAGE_SIZE * crate::guest::PAGE_SIZE;
        let base = region.address / crate::guest::PAGE_SIZE * crate::guest::PAGE_SIZE;
        if base != region.address {
            return Err(Error::Invalid("load-plan region base must be page aligned"));
        }
        process.address_space.map(base, rounded, Permissions::READ_WRITE)?;
        if !region.initial_bytes.is_empty() {
            process.address_space.write(base, &region.initial_bytes)?;
        }
        process.address_space.protect(base, region.permissions)?;
    }
    let stack_size = plan.stack_size.checked_add(crate::guest::PAGE_SIZE - 1)
        .ok_or(Error::Overflow)? / crate::guest::PAGE_SIZE * crate::guest::PAGE_SIZE;
    let stack_base = 0x0000_7000_0000_0000u64;
    process.address_space.map(stack_base, stack_size, Permissions::READ_WRITE)?;
    let stack_top = stack_base.checked_add(stack_size).ok_or(Error::Overflow)?;
    process.set_entry(plan.entry_point, stack_top)?;
    Ok(process)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materializes_a_plan_with_wx_permissions_and_runs_original_bytes() {
        let plan = LoadPlan {
            format: Format::Pe32Plus,
            architecture: crate::Architecture::X86_64,
            image_base: 0x400000,
            entry_point: 0x400000,
            stack_size: crate::guest::PAGE_SIZE,
            instruction_budget: 16,
            dependencies: Vec::new(),
            regions: vec![PlannedRegion {
                address: 0x400000,
                memory_size: crate::guest::PAGE_SIZE,
                permissions: Permissions::READ_EXECUTE,
                initial_bytes: vec![0xf4],
            }],
        };
        let mut process = materialize(&plan, 10).unwrap();
        assert_eq!(process.run().unwrap(), crate::guest::ExecutionEvent::Halted);
    }

    #[test]
    fn rejects_writable_executable_image_regions() {
        let result = validate_region(
            0x400000, 4096, 1,
            Permissions { read: true, write: true, execute: true },
        );
        assert!(result.is_err());
    }
}


/// Host-side policy boundary for interpreted guest syscalls. Implementations
/// must authorize every operation; guest pointers are never dereferenced here.
pub trait SyscallBroker {
    fn dispatch(&mut self, number: u64, arguments: [u64; 6]) -> Result<i64, Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Exited(i32),
    Breakpoint { rip: u64 },
    BudgetExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionSummary {
    pub process_id: u64,
    pub outcome: ExecutionOutcome,
    pub syscalls_dispatched: u64,
    pub instructions_remaining: u64,
}

/// Execute a prepared image through the bounded direct x86-64 interpreter.
/// All syscall effects are delegated to the explicit broker; no guest syscall
/// number is forwarded to the host kernel directly.
pub fn execute_with_broker(
    plan: &LoadPlan,
    process_id: u64,
    broker: &mut impl SyscallBroker,
) -> Result<ExecutionSummary, Error> {
    let mut process = materialize(plan, process_id)?;
    let mut syscalls = 0u64;
    loop {
        match process.run()? {
            crate::guest::ExecutionEvent::Syscall { number, arguments, .. } => {
                let result = broker.dispatch(number, arguments)?;
                process.resume_with_syscall_result(result)?;
                syscalls = syscalls.checked_add(1).ok_or(Error::Overflow)?;
            }
            crate::guest::ExecutionEvent::Halted => {
                let code = match process.state {
                    crate::guest::ProcessState::Exited(code) => code,
                    _ => 0,
                };
                return Ok(ExecutionSummary {
                    process_id,
                    outcome: ExecutionOutcome::Exited(code),
                    syscalls_dispatched: syscalls,
                    instructions_remaining: process.instruction_budget,
                });
            }
            crate::guest::ExecutionEvent::Breakpoint { rip } => {
                return Ok(ExecutionSummary {
                    process_id,
                    outcome: ExecutionOutcome::Breakpoint { rip },
                    syscalls_dispatched: syscalls,
                    instructions_remaining: process.instruction_budget,
                });
            }
            crate::guest::ExecutionEvent::BudgetExhausted => {
                return Ok(ExecutionSummary {
                    process_id,
                    outcome: ExecutionOutcome::BudgetExhausted,
                    syscalls_dispatched: syscalls,
                    instructions_remaining: 0,
                });
            }
            crate::guest::ExecutionEvent::Continue => unreachable!("run consumes continue events"),
        }
    }
}


/// Exact current instruction/format envelope for the in-crate development
/// backend. This is intentionally not advertised as Windows or macOS ABI
/// compatibility.
pub const INTERPRETED_PROFILE_ID: &str = "TRION-X64-INTERPRETED-CORE-0";
pub const INTERPRETED_PROFILE_INSTRUCTIONS: &[&str] = &[
    "NOP", "RET", "INT3", "HLT", "SYSCALL", "PUSH r64", "POP r64",
    "MOV r64, imm64", "CALL rel32", "JMP rel32", "Jcc rel32",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionProfile {
    pub id: &'static str,
    pub architecture: crate::Architecture,
    pub max_image_size: usize,
    pub max_guest_memory: u64,
    pub instruction_set: &'static [&'static str],
    pub supports_pe32_plus: bool,
    pub supports_macho64: bool,
    pub supports_dynamic_linking: bool,
    pub native_kernel_backend: bool,
}

pub const INTERPRETED_CORE_PROFILE: ExecutionProfile = ExecutionProfile {
    id: INTERPRETED_PROFILE_ID,
    architecture: crate::Architecture::X86_64,
    max_image_size: crate::MAX_IMAGE_SIZE,
    max_guest_memory: crate::guest::MAX_GUEST_MEMORY,
    instruction_set: INTERPRETED_PROFILE_INSTRUCTIONS,
    supports_pe32_plus: true,
    supports_macho64: true,
    supports_dynamic_linking: false,
    native_kernel_backend: false,
};
