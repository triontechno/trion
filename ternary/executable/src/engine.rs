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
    for section in &image.description.sections {
        if section.memory_size == 0 { continue; }
        let writable = section.writable;
        let executable = section.executable;
        let permissions = Permissions { read: section.readable, write: writable, execute: executable };
        permissions.validate()?;
        let start = usize::try_from(section.address).map_err(|_| Error::Overflow)?;
        let file_len = usize::try_from(section.file_size).map_err(|_| Error::Overflow)?;
        let end = start.checked_add(file_len).ok_or(Error::Overflow)?;
        let initial_bytes = image.bytes.get(start..end)
            .ok_or(Error::Truncated("mapped PE section"))?.to_vec();
        let address = base.checked_add(section.address).ok_or(Error::Overflow)?;
        validate_region(address, section.memory_size, initial_bytes.len(), permissions)?;
        regions.push(PlannedRegion {
            address,
            memory_size: section.memory_size,
            permissions,
            initial_bytes,
        });
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
pub fn prepare_macho64(input: &[u8], load_base: Option<u64>) -> Result<LoadPlan, Error> {
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
            read: true,
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
