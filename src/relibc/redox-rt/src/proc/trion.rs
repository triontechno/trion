//! Restricted native PE32+ entry path for the Redox process model.
//!
//! This loader maps a restricted freestanding PE32+ image into a fresh Redox
//! address space. It applies DIR64 base relocations and resolves only the
//! explicitly supported ExitProcess import; broader Windows runtime support is separate.

use super::*;
use alloc::{collections::BTreeSet, vec::Vec};

const MAX_PE_IMAGE: usize = 256 * 1024 * 1024;
const MAX_PE_HEADERS: usize = 16 * 1024 * 1024;
const MAX_PE_SECTIONS: usize = 96;

#[derive(Clone, Copy)]
struct Section {
    virtual_address: usize,
    image_size: usize,
    memory_size: usize,
    raw_offset: u64,
    raw_size: usize,
    protection: MapFlags,
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16> {
    let end = offset.checked_add(2).ok_or(Error::new(EOVERFLOW))?;
    let value = bytes.get(offset..end).ok_or(Error::new(ENOEXEC))?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset.checked_add(4).ok_or(Error::new(EOVERFLOW))?;
    let value = bytes.get(offset..end).ok_or(Error::new(ENOEXEC))?;
    Ok(u32::from_le_bytes(value.try_into().map_err(|_| Error::new(ENOEXEC))?))
}

fn u64_at(bytes: &[u8], offset: usize) -> Result<u64> {
    let end = offset.checked_add(8).ok_or(Error::new(EOVERFLOW))?;
    let value = bytes.get(offset..end).ok_or(Error::new(ENOEXEC))?;
    Ok(u64::from_le_bytes(value.try_into().map_err(|_| Error::new(ENOEXEC))?))
}

fn read_vec(fd: &FdGuardUpper, offset: u64, size: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(|_| Error::new(ENOMEM))?;
    bytes.resize(size, 0);
    pread_all(fd, offset, &mut bytes)?;
    Ok(bytes)
}

fn page_align(value: usize) -> Result<usize> {
    value
        .checked_add(PAGE_SIZE - 1)
        .map(|value| value / PAGE_SIZE * PAGE_SIZE)
        .ok_or(Error::new(EOVERFLOW))
}

fn map_writable(addrspace: &FdGuardUpper, address: usize, size: usize) -> Result<()> {
    if address % PAGE_SIZE != 0 || size == 0 || size % PAGE_SIZE != 0 {
        return Err(Error::new(ENOEXEC));
    }
    let flags = MapFlags::PROT_READ | MapFlags::PROT_WRITE | MapFlags::MAP_FIXED_NOREPLACE;
    let mapped = mmap_anon_remote(addrspace, 0, address, size, flags)?;
    if mapped != address {
        return Err(Error::new(ENOEXEC));
    }
    Ok(())
}

fn map_writable_fixed(addrspace: &FdGuardUpper, address: usize, size: usize) -> Result<()> {
    if address % PAGE_SIZE != 0 || size == 0 || size % PAGE_SIZE != 0 {
        return Err(Error::new(ENOEXEC));
    }
    let flags = MapFlags::PROT_READ | MapFlags::PROT_WRITE | MapFlags::MAP_FIXED;
    let mapped = mmap_anon_remote(addrspace, 0, address, size, flags)?;
    if mapped != address {
        return Err(Error::new(ENOEXEC));
    }
    Ok(())
}

fn apply_pe_relocations(
    image: &mut [u8],
    sections: &[Section],
    reloc_rva: usize,
    reloc_size: usize,
    preferred_base: usize,
    actual_base: usize,
) -> Result<()> {
    if actual_base == preferred_base {
        return Ok(());
    }
    if reloc_rva == 0 || reloc_size == 0 {
        return Err(Error::new(ENOEXEC));
    }
    let reloc_end = reloc_rva
        .checked_add(reloc_size)
        .filter(|end| *end <= image.len())
        .ok_or(Error::new(ENOEXEC))?;
    let delta = actual_base as i128 - preferred_base as i128;
    let mut at = reloc_rva;
    let mut relocation_count = 0usize;
    let mut seen_targets = BTreeSet::new();
    while at < reloc_end {
        if reloc_end - at < 8 {
            return Err(Error::new(ENOEXEC));
        }
        let page_rva = u32_at(image, at)? as usize;
        let block_size = u32_at(image, at + 4)? as usize;
        let block_end = at
            .checked_add(block_size)
            .filter(|end| block_size >= 8 && block_size % 2 == 0 && *end <= reloc_end)
            .ok_or(Error::new(ENOEXEC))?;
        if page_rva % PAGE_SIZE != 0 || page_rva >= image.len() {
            return Err(Error::new(ENOEXEC));
        }
        let entries = (block_size - 8) / 2;
        for index in 0..entries {
            relocation_count = relocation_count
                .checked_add(1)
                .filter(|count| *count <= 1_000_000)
                .ok_or(Error::new(ENOEXEC))?;
            let entry = u16_at(image, at + 8 + index * 2)?;
            match entry >> 12 {
                0 => {}
                10 => {
                    let target = page_rva
                        .checked_add(usize::from(entry & 0x0fff))
                        .ok_or(Error::new(EOVERFLOW))?;
                    let target_end = target.checked_add(8).ok_or(Error::new(EOVERFLOW))?;
                    let in_section = sections.iter().any(|section| {
                        target >= section.virtual_address
                            && target_end
                                <= section.virtual_address.saturating_add(section.image_size)
                    });
                    if !in_section {
                        return Err(Error::new(ENOEXEC));
                    }
                    if !seen_targets.insert(target) {
                        return Err(Error::new(ENOEXEC));
                    }
                    let value = (u64_at(image, target)? as i128)
                        .checked_add(delta)
                        .filter(|value| (0..=u64::MAX as i128).contains(value))
                        .ok_or(Error::new(ENOEXEC))? as u64;
                    image[target..target_end].copy_from_slice(&value.to_le_bytes());
                }
                _ => return Err(Error::new(ENOEXEC)),
            }
        }
        at = block_end;
    }
    if at != reloc_end {
        return Err(Error::new(ENOEXEC));
    }
    Ok(())
}

fn resolve_pe_imports(
    image: &mut [u8],
    sections: &[Section],
    headers_size: usize,
    import_rva: usize,
    import_size: usize,
    addrspace: &FdGuardUpper,
    stack_base: usize,
) -> Result<()> {
    if import_rva == 0 && import_size == 0 {
        return Ok(());
    }
    if import_rva == 0 || import_size < 20 {
        return Err(Error::new(ENOEXEC));
    }
    let rva_is_mapped = |rva: usize, size: usize| {
        rva.checked_add(size).is_some_and(|end| {
            end <= headers_size
                || sections.iter().any(|section| {
                    rva >= section.virtual_address
                        && end <= section.virtual_address.saturating_add(section.image_size)
                })
        })
    };
    let import_end = import_rva
        .checked_add(import_size)
        .filter(|end| *end <= image.len() && rva_is_mapped(import_rva, import_size))
        .ok_or(Error::new(ENOEXEC))?;
    let mut iat_slots = Vec::new();
    let mut seen_slots = BTreeSet::new();
    let mut descriptor_at = import_rva;
    let mut descriptor_count = 0usize;
    let mut terminated = false;
    while descriptor_at.checked_add(20).is_some_and(|end| end <= import_end) {
        descriptor_count += 1;
        if descriptor_count > 4096 {
            return Err(Error::new(ENOEXEC));
        }
        let original_thunk = u32_at(image, descriptor_at)? as usize;
        let name_rva = u32_at(image, descriptor_at + 12)? as usize;
        let first_thunk = u32_at(image, descriptor_at + 16)? as usize;
        if original_thunk == 0 && name_rva == 0 && first_thunk == 0 {
            if image[descriptor_at..import_end].iter().any(|byte| *byte != 0) {
                return Err(Error::new(ENOEXEC));
            }
            terminated = true;
            break;
        }
        if name_rva == 0 || first_thunk == 0 {
            return Err(Error::new(ENOEXEC));
        }

        let library_tail = image.get(name_rva..).ok_or(Error::new(ENOEXEC))?;
        let library_len = library_tail
            .iter()
            .take(4096)
            .position(|byte| *byte == 0)
            .ok_or(Error::new(ENOEXEC))?;
        let library = &library_tail[..library_len];
        if !rva_is_mapped(name_rva, library_len + 1) {
            return Err(Error::new(ENOEXEC));
        }
        if !library.eq_ignore_ascii_case(b"kernel32.dll")
            && !library.eq_ignore_ascii_case(b"kernelbase.dll")
        {
            return Err(Error::new(ENOEXEC));
        }

        let lookup = if original_thunk == 0 {
            first_thunk
        } else {
            original_thunk
        };
        let mut thunk_index = 0usize;
        let mut thunk_terminated = false;
        loop {
            if thunk_index >= 16_384 {
                return Err(Error::new(ENOEXEC));
            }
            let offset = thunk_index.checked_mul(8).ok_or(Error::new(EOVERFLOW))?;
            let lookup_at = lookup.checked_add(offset).ok_or(Error::new(EOVERFLOW))?;
            let iat_at = first_thunk.checked_add(offset).ok_or(Error::new(EOVERFLOW))?;
            if !rva_is_mapped(lookup_at, 8) || !rva_is_mapped(iat_at, 8) {
                return Err(Error::new(ENOEXEC));
            }
            let value = u64_at(image, lookup_at)?;
            if value == 0 {
                thunk_terminated = true;
                break;
            }
            if value & (1u64 << 63) != 0 {
                return Err(Error::new(ENOEXEC));
            }
            let name_at = usize::try_from(value).map_err(|_| Error::new(EOVERFLOW))?;
            let symbol_at = name_at.checked_add(2).ok_or(Error::new(EOVERFLOW))?;
            let symbol_tail = image.get(symbol_at..).ok_or(Error::new(ENOEXEC))?;
            let symbol_len = symbol_tail
                .iter()
                .take(4096)
                .position(|byte| *byte == 0)
                .ok_or(Error::new(ENOEXEC))?;
            if !rva_is_mapped(name_at, symbol_len + 3) {
                return Err(Error::new(ENOEXEC));
            }
            if &symbol_tail[..symbol_len] != b"ExitProcess" {
                return Err(Error::new(ENOEXEC));
            }
            let iat_end = iat_at.checked_add(8).ok_or(Error::new(EOVERFLOW))?;
            if iat_end > image.len() || !seen_slots.insert(iat_at) {
                return Err(Error::new(ENOEXEC));
            }
            if iat_slots.len() >= 16_384 {
                return Err(Error::new(ENOEXEC));
            }
            iat_slots.try_reserve(1).map_err(|_| Error::new(ENOMEM))?;
            iat_slots.push(iat_at);
            thunk_index += 1;
        }
        if !thunk_terminated {
            return Err(Error::new(ENOEXEC));
        }
        descriptor_at = descriptor_at.checked_add(20).ok_or(Error::new(EOVERFLOW))?;
    }
    if !terminated {
        return Err(Error::new(ENOEXEC));
    }
    if iat_slots.is_empty() {
        return Ok(());
    }

    let thunk = exit_process_thunk()?;
    let thunk_page = mmap_anon_remote(
        addrspace,
        0,
        0,
        PAGE_SIZE,
        MapFlags::PROT_READ | MapFlags::PROT_WRITE,
    )?;
    let thunk_end = thunk_page
        .checked_add(PAGE_SIZE)
        .ok_or(Error::new(EOVERFLOW))?;
    if thunk_page < 0x1_0000 || thunk_page % PAGE_SIZE != 0 || thunk_end > stack_base {
        let _ = munmap_remote(addrspace, thunk_page, PAGE_SIZE);
        return Err(Error::new(ENOEXEC));
    }
    if let Err(error) = write_remote(addrspace, thunk_page, &thunk) {
        let _ = munmap_remote(addrspace, thunk_page, PAGE_SIZE);
        return Err(error);
    }
    if let Err(error) = mprotect_remote(
        addrspace,
        thunk_page,
        PAGE_SIZE,
        MapFlags::PROT_READ | MapFlags::PROT_EXEC,
    ) {
        let _ = munmap_remote(addrspace, thunk_page, PAGE_SIZE);
        return Err(error);
    }
    for slot in iat_slots {
        image[slot..slot + 8].copy_from_slice(&(thunk_page as u64).to_le_bytes());
    }
    Ok(())
}

fn write_remote(addrspace: &FdGuardUpper, address: usize, bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }
    let page_base = address / PAGE_SIZE * PAGE_SIZE;
    let displacement = address - page_base;
    let size = page_align(displacement.checked_add(bytes.len()).ok_or(Error::new(EOVERFLOW))?)?;
    let (_guard, target) =
        unsafe { MmapGuard::map_mut_anywhere(addrspace, page_base, size)? };
    let end = displacement.checked_add(bytes.len()).ok_or(Error::new(EOVERFLOW))?;
    target
        .get_mut(displacement..end)
        .ok_or(Error::new(ENOEXEC))?
        .copy_from_slice(bytes);
    Ok(())
}

fn exit_trampoline() -> Result<Vec<u8>> {
    guest_exit_thunk([0x89, 0xC7])
}

fn exit_process_thunk() -> Result<Vec<u8>> {
    guest_exit_thunk([0x89, 0xCF])
}

fn guest_exit_thunk(move_status: [u8; 2]) -> Result<Vec<u8>> {
    let syscall_number: u32 = syscall::SYS_GUEST_EXIT
        .try_into()
        .map_err(|_| Error::new(ENOEXEC))?;
    let mut code = alloc::vec![
        move_status[0], move_status[1],
        0x83, 0xE7, 0xFF,             // and edi, 0xff (Redox process exit status)
        0x31, 0xF6,                   // xor esi, esi
        0x31, 0xD2,                   // xor edx, edx
        0x45, 0x31, 0xD2,             // xor r10d, r10d
        0x45, 0x31, 0xC0,             // xor r8d, r8d
        0x45, 0x31, 0xC9,             // xor r9d, r9d
        0xB8, 0, 0, 0, 0,             // mov eax, SYS_GUEST_EXIT
        0x0F, 0x05,                   // syscall
        0x0F, 0x0B,                   // ud2 if the non-returning syscall unexpectedly returns
    ];
    code[19..23].copy_from_slice(&syscall_number.to_le_bytes());
    Ok(code)
}

fn write_guest_policy(guest_policy_fd: &FdGuardUpper, grants_fd: &FdGuardUpper) -> Result<()> {
    let policy = [0, grants_fd.as_raw_fd() as usize];
    let policy = unsafe {
        core::slice::from_raw_parts(
            policy.as_ptr().cast::<u8>(),
            core::mem::size_of_val(&policy),
        )
    };
    guest_policy_fd.write(policy)?;
    Ok(())
}

fn preflight_guest_policy(
    thread_fd: &FdGuardUpper,
    grants_fd: &FdGuardUpper,
) -> Result<FdGuardUpper> {
    let guest_policy_fd = thread_fd.dup_into_upper(b"guest-policy")?;
    guest_policy_fd.fcntl(syscall::F_SETFD, O_CLOEXEC)?;
    write_guest_policy(&guest_policy_fd, grants_fd)?;
    Ok(guest_policy_fd)
}

fn clear_guest_filetable(thread_fd: &FdGuardUpper) -> Result<()> {
    let filetable_fd = thread_fd.dup_into_upper(b"filetable-binary")?;
    filetable_fd.fcntl(syscall::F_SETFD, O_CLOEXEC)?;
    filetable_fd.call_wo(
        &[],
        CallFlags::empty(),
        &[syscall::FileTableVerb::Refresh as u64],
    )?;
    let bytes_len = filetable_fd.lseek(0, syscall::flag::SEEK_END)?;
    if bytes_len % size_of::<u64>() != 0 {
        return Err(Error::new(ENOEXEC));
    }
    filetable_fd.lseek(0, SEEK_SET)?;

    let mut buffer = [0u8; 256 * size_of::<u64>()];
    let mut remaining = bytes_len;
    while remaining > 0 {
        let chunk_size = remaining.min(buffer.len());
        let mut read = 0;
        while read < chunk_size {
            let count = filetable_fd.read(&mut buffer[read..chunk_size])?;
            if count == 0 {
                return Err(Error::new(ENOEXEC));
            }
            read += count;
        }
        filetable_fd.call_wo(
            &buffer[..chunk_size],
            CallFlags::empty(),
            &[syscall::FileTableVerb::Close as u64],
        )?;
        remaining -= chunk_size;
    }
    Ok(())
}

fn finish_native_exec(
    image_file: FdGuardUpper,
    thread_fd: &FdGuardUpper,
    proc_fd: &FdGuardUpper,
    path: &[u8],
    extrainfo: &ExtraInfo,
    grants_fd: FdGuardUpper,
    entry: usize,
    stack_return_offset: usize,
) -> Result<Option<FexecResult>> {
    let stack_base = super::STACK_TOP
        .checked_sub(super::STACK_SIZE)
        .ok_or(Error::new(EOVERFLOW))?;
    map_writable(&grants_fd, stack_base, super::STACK_SIZE)?;

    let trampoline = exit_trampoline()?;
    let trampoline_size = page_align(trampoline.len())?;
    let trampoline_address = mmap_anon_remote(
        &grants_fd,
        0,
        0,
        trampoline_size,
        MapFlags::PROT_READ | MapFlags::PROT_WRITE,
    )?;
    if trampoline_address == 0 || trampoline_address % PAGE_SIZE != 0 {
        return Err(Error::new(ENOEXEC));
    }
    if let Err(error) = write_remote(&grants_fd, trampoline_address, &trampoline) {
        let _ = munmap_remote(&grants_fd, trampoline_address, trampoline_size);
        return Err(error);
    }
    if let Err(error) = mprotect_remote(
        &grants_fd,
        trampoline_address,
        trampoline_size,
        MapFlags::PROT_READ | MapFlags::PROT_EXEC,
    ) {
        let _ = munmap_remote(&grants_fd, trampoline_address, trampoline_size);
        return Err(error);
    }

    // Both supported raw entry ABIs enter with RSP == 8 (mod 16). PE reserves
    // the Windows 32-byte home area; the freestanding Mach-O profile does not.
    let stack_pointer = super::STACK_TOP
        .checked_sub(stack_return_offset)
        .ok_or(Error::new(EOVERFLOW))?;
    if stack_pointer % 16 != 8 {
        return Err(Error::new(ENOEXEC));
    }
    write_remote(
        &grants_fd,
        stack_pointer,
        &trampoline_address.to_ne_bytes(),
    )?;

    let addrspace_selection_fd = thread_fd.dup_into_upper(b"current-addrspace")?;

    let mut name = [0u8; 32];
    let name_len = path.len().min(name.len());
    name[..name_len].copy_from_slice(&path[..name_len]);
    sys_call_wo(
        proc_fd.as_raw_fd(),
        &name,
        CallFlags::empty(),
        &[ProcCall::Rename as u64],
    )?;
    sys_call(
        proc_fd.as_raw_fd(),
        CallFlags::empty(),
        &[ProcCall::DisableSetpgid as u64],
    )?;
    // Native images have no registered Redox signal runtime. Clear inherited
    // user/exception handlers before entering untrusted guest code so faults
    // follow the kernel's ordinary process signal/termination path.
    let sighandler_fd = thread_fd.dup_into_upper(b"sighandler")?;
    sighandler_fd.write(&SetSighandlerData {
        user_handler: 0,
        excp_handler: 0,
        thread_control_addr: 0,
        proc_control_addr: 0,
    })?;

    if extrainfo.same_process {
        let guest_policy_fd = preflight_guest_policy(thread_fd, &grants_fd)?;
        let _siglock = crate::signal::tmp_disable_signals();
        let fds_to_close = {
            let guard = crate::current_filetable();
            guard
                .iter()
                .filter_map(|(fd, flags)| {
                    if fd == addrspace_selection_fd.as_raw_fd()
                        || fd == guest_policy_fd.as_raw_fd()
                        || fd == grants_fd.as_raw_fd()
                    {
                        None
                    } else if flags & O_CLOEXEC == O_CLOEXEC || fd == image_file.as_raw_fd() {
                        Some(fd)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        let fds_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(
                fds_to_close.as_ptr().cast::<u8>(),
                fds_to_close.len() * core::mem::size_of::<usize>(),
            )
        };
        let filetable_fd = thread_fd.dup_into_upper(b"filetable-binary")?;
        filetable_fd.call_wo(
            fds_bytes,
            CallFlags::empty(),
            &[syscall::FileTableVerb::Close as u64],
        )?;
        {
            let mut guard = crate::current_filetable();
            for fd in fds_to_close {
                let _ = guard.remove(fd);
            }
        }
        drop(filetable_fd);
        unsafe { super::deactivate_tcb(thread_fd)?; }
        addrspace_selection_fd.write(&create_set_addr_space_buf(
            grants_fd.as_raw_fd(),
            entry,
            stack_pointer,
        ))?;
        drop(guest_policy_fd);
        drop(grants_fd);
        let old_filetable_fd = {
            let mut guard = crate::current_filetable();
            guard.take()
        };
        drop(old_filetable_fd);
        crate::sys::close(addrspace_selection_fd.take())?;
        unreachable!("closing current-addrspace switches into the native image");
    }

    // The kernel activates the policy atomically with the selected address
    // space, before the pending switch can be committed.
    clear_guest_filetable(thread_fd)?;
    let guest_policy_fd = preflight_guest_policy(thread_fd, &grants_fd)?;
    addrspace_selection_fd.write(&create_set_addr_space_buf(
        grants_fd.as_raw_fd(),
        entry,
        stack_pointer,
    ))?;
    crate::sys::close(addrspace_selection_fd.take())?;
    // The guest must not inherit the loader's policy-control descriptor.
    drop(guest_policy_fd);
    Ok(None)
}

fn launch_pe(
    image_file: FdGuardUpper,
    thread_fd: &FdGuardUpper,
    proc_fd: &FdGuardUpper,
    path: &[u8],
    args: &[&[u8]],
    envs: &[&[u8]],
    extrainfo: &ExtraInfo,
) -> Result<Option<FexecResult>> {
    // This first Windows profile omits process parameters, TLS, exceptions and
    // general Win32 services; only explicit, supported imports are resolved.
    if !args.is_empty() || !envs.is_empty() {
        return Err(Error::new(ENOEXEC));
    }

    let dos = read_vec(&image_file, 0, 64)?;
    if dos.get(0..2) != Some(&b"MZ"[..]) {
        return Err(Error::new(ENOEXEC));
    }
    let pe_offset = u64::from(u32_at(&dos, 0x3C)?);
    let coff = read_vec(&image_file, pe_offset, 24)?;
    if coff.get(0..4) != Some(&b"PE\0\0"[..]) {
        return Err(Error::new(ENOEXEC));
    }
    if u16_at(&coff, 4)? != 0x8664 {
        return Err(Error::new(ENOEXEC));
    }
    let section_count = usize::from(u16_at(&coff, 6)?);
    let optional_size = usize::from(u16_at(&coff, 20)?);
    let characteristics = u16_at(&coff, 22)?;
    if section_count == 0
        || section_count > MAX_PE_SECTIONS
        || characteristics & 0x0002 == 0
        || characteristics & 0x2000 != 0
        || optional_size < 112
    {
        return Err(Error::new(ENOEXEC));
    }

    let optional_offset = pe_offset.checked_add(24).ok_or(Error::new(EOVERFLOW))?;
    let optional = read_vec(&image_file, optional_offset, optional_size)?;
    if u16_at(&optional, 0)? != 0x20B {
        return Err(Error::new(ENOEXEC));
    }
    let entry_rva = usize::try_from(u32_at(&optional, 16)?)
        .map_err(|_| Error::new(EOVERFLOW))?;
    let image_base = usize::try_from(u64_at(&optional, 24)?)
        .map_err(|_| Error::new(EOVERFLOW))?;
    let section_alignment = usize::try_from(u32_at(&optional, 32)?)
        .map_err(|_| Error::new(EOVERFLOW))?;
    let subsystem = u16_at(&optional, 68)?;
    let file_alignment = usize::try_from(u32_at(&optional, 36)?)
        .map_err(|_| Error::new(EOVERFLOW))?;
    let image_size = usize::try_from(u32_at(&optional, 56)?)
        .map_err(|_| Error::new(EOVERFLOW))?;
    let headers_size = usize::try_from(u32_at(&optional, 60)?)
        .map_err(|_| Error::new(EOVERFLOW))?;
    let directory_count = usize::try_from(u32_at(&optional, 108)?)
        .map_err(|_| Error::new(EOVERFLOW))?;
    let directory_bytes = directory_count.checked_mul(8).ok_or(Error::new(EOVERFLOW))?;
    if directory_count > 16 || optional_size < 112usize.checked_add(directory_bytes).ok_or(Error::new(EOVERFLOW))? {
        return Err(Error::new(ENOEXEC));
    }
    let (reloc_rva, reloc_size) = if directory_count > 5 {
        (
            usize::try_from(u32_at(&optional, 112 + 5 * 8)?)
                .map_err(|_| Error::new(EOVERFLOW))?,
            usize::try_from(u32_at(&optional, 112 + 5 * 8 + 4)?)
                .map_err(|_| Error::new(EOVERFLOW))?,
        )
    } else {
        (0, 0)
    };

    if image_base < 0x1_0000
        || image_base % 0x1_0000 != 0
        || image_base % PAGE_SIZE != 0
        || section_alignment < PAGE_SIZE
        || !section_alignment.is_power_of_two()
        || !(512..=65_536).contains(&file_alignment)
        || !file_alignment.is_power_of_two()
        || file_alignment > section_alignment
        || image_size == 0
        || image_size > MAX_PE_IMAGE
        || image_size % section_alignment != 0
        || image_size % PAGE_SIZE != 0
        || headers_size == 0
        || headers_size > MAX_PE_HEADERS
        || headers_size > image_size
        || headers_size % file_alignment != 0
        || entry_rva == 0
        || entry_rva >= image_size
        || subsystem != 3
    {
        return Err(Error::new(ENOEXEC));
    }
    let image_end = image_base.checked_add(image_size).ok_or(Error::new(EOVERFLOW))?;
    let stack_base = super::STACK_TOP
        .checked_sub(super::STACK_SIZE)
        .ok_or(Error::new(EOVERFLOW))?;
    if image_end > stack_base {
        return Err(Error::new(ENOEXEC));
    }

    let (import_rva, import_size) = if directory_count > 1 {
        if optional_size < 128 {
            return Err(Error::new(ENOEXEC));
        }
        (
            usize::try_from(u32_at(&optional, 120)?).map_err(|_| Error::new(EOVERFLOW))?,
            usize::try_from(u32_at(&optional, 124)?).map_err(|_| Error::new(EOVERFLOW))?,
        )
    } else {
        (0, 0)
    };
    if (import_rva == 0) != (import_size == 0) {
        return Err(Error::new(ENOEXEC));
    }
    let (iat_rva, iat_size) = if directory_count > 12 {
        (
            usize::try_from(u32_at(&optional, 112 + 12 * 8)?)
                .map_err(|_| Error::new(EOVERFLOW))?,
            usize::try_from(u32_at(&optional, 112 + 12 * 8 + 4)?)
                .map_err(|_| Error::new(EOVERFLOW))?,
        )
    } else {
        (0, 0)
    };
    if (iat_rva == 0) != (iat_size == 0) || (import_rva == 0 && iat_rva != 0) {
        return Err(Error::new(ENOEXEC));
    }
    if directory_count > 3
        && (u32_at(&optional, 136)? != 0 || u32_at(&optional, 140)? != 0)
    {
        return Err(Error::new(ENOEXEC));
    }
    if directory_count > 9 {
        if optional_size < 192 {
            return Err(Error::new(ENOEXEC));
        }
        if u32_at(&optional, 184)? != 0 || u32_at(&optional, 188)? != 0 {
            return Err(Error::new(ENOEXEC));
        }
    }
    if directory_count > 13 {
        if optional_size < 224 {
            return Err(Error::new(ENOEXEC));
        }
        if u32_at(&optional, 216)? != 0 || u32_at(&optional, 220)? != 0 {
            return Err(Error::new(ENOEXEC));
        }
    }

    // Reject loader metadata that would require CFG, bound imports, an IAT,
    // or a managed (.NET) runtime even if the ordinary import directory is empty.
    for index in [10usize, 11, 14] {
        if directory_count > index {
            let offset = 112usize
                .checked_add(index.checked_mul(8).ok_or(Error::new(EOVERFLOW))?)
                .ok_or(Error::new(EOVERFLOW))?;
            if u32_at(&optional, offset)? != 0 || u32_at(&optional, offset + 4)? != 0 {
                return Err(Error::new(ENOEXEC));
            }
        }
    }

    let section_table_offset = optional_offset
        .checked_add(optional_size as u64)
        .ok_or(Error::new(EOVERFLOW))?;
    let table_size = section_count.checked_mul(40).ok_or(Error::new(EOVERFLOW))?;
    let table_end = section_table_offset
        .checked_add(table_size as u64)
        .ok_or(Error::new(EOVERFLOW))?;
    if table_end > headers_size as u64 {
        return Err(Error::new(ENOEXEC));
    }
    let table = read_vec(&image_file, section_table_offset, table_size)?;
    let mut sections = Vec::new();
    sections.try_reserve_exact(section_count).map_err(|_| Error::new(ENOMEM))?;
    let mut ranges = Vec::<(usize, usize)>::new();
    let header_map_size = page_align(headers_size)?;
    if header_map_size > image_size {
        return Err(Error::new(ENOEXEC));
    }
    ranges.push((image_base, image_base.checked_add(header_map_size).ok_or(Error::new(EOVERFLOW))?));
    let mut entry_is_executable = false;

    for index in 0..section_count {
        let at = index.checked_mul(40).ok_or(Error::new(EOVERFLOW))?;
        let virtual_size = usize::try_from(u32_at(&table, at + 8)?)
            .map_err(|_| Error::new(EOVERFLOW))?;
        let virtual_address = usize::try_from(u32_at(&table, at + 12)?)
            .map_err(|_| Error::new(EOVERFLOW))?;
        let raw_size = usize::try_from(u32_at(&table, at + 16)?)
            .map_err(|_| Error::new(EOVERFLOW))?;
        let raw_offset = u64::from(u32_at(&table, at + 20)?);
        let flags = u32_at(&table, at + 36)?;
        let memory_size = virtual_size.max(raw_size);
        if memory_size == 0 {
            continue;
        }
        if virtual_address == 0
            || virtual_address % section_alignment != 0
            || virtual_address % PAGE_SIZE != 0
            || raw_size > memory_size
            || raw_size > 0 && raw_offset < headers_size as u64
            || raw_size > 0 && raw_offset % file_alignment as u64 != 0
        {
            return Err(Error::new(ENOEXEC));
        }
        let mapped_size = page_align(memory_size)?;
        virtual_address
            .checked_add(mapped_size)
            .filter(|end| *end <= image_size)
            .ok_or(Error::new(ENOEXEC))?;
        raw_offset.checked_add(raw_size as u64).ok_or(Error::new(EOVERFLOW))?;
        let absolute_start = image_base.checked_add(virtual_address).ok_or(Error::new(EOVERFLOW))?;
        let absolute_end = absolute_start.checked_add(mapped_size).ok_or(Error::new(EOVERFLOW))?;
        for (old_start, old_end) in &ranges {
            if absolute_start < *old_end && *old_start < absolute_end {
                return Err(Error::new(ENOEXEC));
            }
        }
        ranges.push((absolute_start, absolute_end));

        let readable = flags & 0x4000_0000 != 0;
        let writable = flags & 0x8000_0000 != 0;
        let executable = flags & 0x2000_0000 != 0;
        if writable && executable || executable && !readable {
            return Err(Error::new(ENOEXEC));
        }
        if executable
            && entry_rva >= virtual_address
            && entry_rva < virtual_address.saturating_add(memory_size)
        {
            entry_is_executable = true;
        }
        let mut protection = MapFlags::empty();
        if readable { protection |= MapFlags::PROT_READ; }
        if writable { protection |= MapFlags::PROT_WRITE; }
        if executable { protection |= MapFlags::PROT_EXEC; }
        sections.push(Section {
            virtual_address,
            image_size: memory_size,
            memory_size: mapped_size,
            raw_offset,
            raw_size,
            protection,
        });
    }
    if !entry_is_executable {
        return Err(Error::new(ENOEXEC));
    }
    if iat_rva != 0
        && !sections.iter().any(|section| {
            iat_rva >= section.virtual_address
                && iat_rva
                    .checked_add(iat_size)
                    .is_some_and(|end| {
                        end <= section.virtual_address.saturating_add(section.image_size)
                    })
        })
    {
        return Err(Error::new(ENOEXEC));
    }

    let mut image_bytes = Vec::new();
    image_bytes
        .try_reserve_exact(image_size)
        .map_err(|_| Error::new(ENOMEM))?;
    image_bytes.resize(image_size, 0);
    let header_bytes = read_vec(&image_file, 0, headers_size)?;
    image_bytes[..headers_size].copy_from_slice(&header_bytes);
    for section in &sections {
        if section.raw_size == 0 {
            continue;
        }
        let raw = read_vec(&image_file, section.raw_offset, section.raw_size)?;
        let end = section
            .virtual_address
            .checked_add(section.raw_size)
            .filter(|end| *end <= image_bytes.len())
            .ok_or(Error::new(ENOEXEC))?;
        image_bytes[section.virtual_address..end].copy_from_slice(&raw);
    }

    let grants_fd = {
        let current_addrspace_fd = thread_fd.dup_into_upper(b"addrspace")?;
        current_addrspace_fd.dup_into_upper(b"empty")?
    };
    grants_fd.fcntl(syscall::F_SETFD, O_CLOEXEC)?;

    let actual_base = match mmap_anon_remote(
        &grants_fd,
        0,
        image_base,
        image_size,
        MapFlags::PROT_NONE | MapFlags::MAP_FIXED_NOREPLACE,
    ) {
        Ok(mapped) if mapped == image_base => image_base,
        Ok(mapped) => {
            let _ = munmap_remote(&grants_fd, mapped, image_size);
            return Err(Error::new(ENOEXEC));
        }
        Err(_) => mmap_anon_remote(
            &grants_fd,
            0,
            0,
            image_size,
            MapFlags::PROT_NONE,
        )?,
    };
    let actual_end = actual_base
        .checked_add(image_size)
        .ok_or(Error::new(EOVERFLOW))?;
    if actual_base < 0x1_0000 || actual_end > stack_base {
        let _ = munmap_remote(&grants_fd, actual_base, image_size);
        return Err(Error::new(ENOEXEC));
    }
    if let Err(error) = apply_pe_relocations(
        &mut image_bytes,
        &sections,
        reloc_rva,
        reloc_size,
        image_base,
        actual_base,
    ) {
        let _ = munmap_remote(&grants_fd, actual_base, image_size);
        return Err(error);
    }
    if let Err(error) = resolve_pe_imports(
        &mut image_bytes,
        &sections,
        headers_size,
        import_rva,
        import_size,
        &grants_fd,
        stack_base,
    ) {
        let _ = munmap_remote(&grants_fd, actual_base, image_size);
        return Err(error);
    }

    // Reserve the complete image span before replacing individual regions;
    // relocations are applied to a private mapped-image buffer before entry.
    map_writable_fixed(&grants_fd, actual_base, header_map_size)?;
    write_remote(&grants_fd, actual_base, &image_bytes[..headers_size])?;
    mprotect_remote(&grants_fd, actual_base, header_map_size, MapFlags::PROT_READ)?;

    for section in &sections {
        let address = actual_base
            .checked_add(section.virtual_address)
            .ok_or(Error::new(EOVERFLOW))?;
        map_writable_fixed(&grants_fd, address, section.memory_size)?;
        write_remote(
            &grants_fd,
            address,
            &image_bytes[section.virtual_address
                ..section.virtual_address + section.memory_size],
        )?;
        mprotect_remote(&grants_fd, address, section.memory_size, section.protection)?;
    }

    let entry = actual_base.checked_add(entry_rva).ok_or(Error::new(EOVERFLOW))?;
    finish_native_exec(
        image_file,
        thread_fd,
        proc_fd,
        path,
        extrainfo,
        grants_fd,
        entry,
        40,
    )
}

fn launch_macho64(
    image_file: FdGuardUpper,
    thread_fd: &FdGuardUpper,
    proc_fd: &FdGuardUpper,
    path: &[u8],
    args: &[&[u8]],
    envs: &[&[u8]],
    extrainfo: &ExtraInfo,
) -> Result<Option<FexecResult>> {
    // The Stage G Mach-O profile is freestanding: no dyld, dylibs, relocations,
    // code-signature enforcement, TLS, argv/env block or macOS framework services.
    if args.len() > 1 || !envs.is_empty() {
        return Err(Error::new(ENOEXEC));
    }
    let header = read_vec(&image_file, 0, 32)?;
    if header.get(0..4) != Some(&[0xCF, 0xFA, 0xED, 0xFE][..])
        || u32_at(&header, 4)? != 0x0100_0007
        || u32_at(&header, 12)? != 2
    {
        return Err(Error::new(ENOEXEC));
    }
    let command_count = usize::try_from(u32_at(&header, 16)?)
        .map_err(|_| Error::new(EOVERFLOW))?;
    let command_bytes = usize::try_from(u32_at(&header, 20)?)
        .map_err(|_| Error::new(EOVERFLOW))?;
    if command_count == 0 || command_count > 4096 || command_bytes > MAX_PE_HEADERS {
        return Err(Error::new(ENOEXEC));
    }
    let commands = read_vec(&image_file, 32, command_bytes)?;
    let mut segments = Vec::<Section>::new();
    let mut ranges = Vec::<(usize, usize)>::new();
    let mut entry_file_offset = None;
    let mut base = usize::MAX;
    let mut image_end = 0usize;
    let mut has_header_segment = false;
    let mut at = 0usize;

    for _ in 0..command_count {
        let command = u32_at(&commands, at)?;
        let command_size = usize::try_from(u32_at(&commands, at + 4)?)
            .map_err(|_| Error::new(EOVERFLOW))?;
        let command_end = at.checked_add(command_size).ok_or(Error::new(EOVERFLOW))?;
        if command_size < 8 || command_end > commands.len() {
            return Err(Error::new(ENOEXEC));
        }
        match command {
            0x19 => { // LC_SEGMENT_64
                if command_size < 72 {
                    return Err(Error::new(ENOEXEC));
                }
                let name_bytes = commands
                    .get(at + 8..at + 24)
                    .ok_or(Error::new(ENOEXEC))?;
                let name_len = name_bytes.iter().position(|byte| *byte == 0).unwrap_or(16);
                let is_pagezero = name_bytes.get(..name_len).is_some_and(|name| name == &b"__PAGEZERO"[..]);
                let vm_address = usize::try_from(u64_at(&commands, at + 24)?)
                    .map_err(|_| Error::new(EOVERFLOW))?;
                let vm_size = usize::try_from(u64_at(&commands, at + 32)?)
                    .map_err(|_| Error::new(EOVERFLOW))?;
                let file_offset = u64_at(&commands, at + 40)?;
                let file_size = usize::try_from(u64_at(&commands, at + 48)?)
                    .map_err(|_| Error::new(EOVERFLOW))?;
                let max_protection = u32_at(&commands, at + 56)?;
                let initial_protection = u32_at(&commands, at + 60)?;
                let section_count = usize::try_from(u32_at(&commands, at + 64)?)
                    .map_err(|_| Error::new(EOVERFLOW))?;
                let section_table_size = section_count
                    .checked_mul(80)
                    .and_then(|size| size.checked_add(72))
                    .ok_or(Error::new(EOVERFLOW))?;
                if section_table_size > command_size
                    || file_size > vm_size
                    || initial_protection & !7 != 0
                    || max_protection & !7 != 0
                    || initial_protection & !max_protection != 0
                    || initial_protection & 6 == 6
                    || initial_protection & 4 != 0 && initial_protection & 1 == 0
                {
                    return Err(Error::new(ENOEXEC));
                }
                if !is_pagezero {
                    if vm_size == 0
                        || vm_address < 0x1_0000
                        || vm_address % PAGE_SIZE != 0
                        || file_offset % PAGE_SIZE as u64 != 0
                    {
                        return Err(Error::new(ENOEXEC));
                    }
                    file_offset.checked_add(file_size as u64).ok_or(Error::new(EOVERFLOW))?;
                    let mapped_size = page_align(vm_size)?;
                    let absolute_end = vm_address
                        .checked_add(mapped_size)
                        .ok_or(Error::new(EOVERFLOW))?;
                    for (old_start, old_end) in &ranges {
                        if vm_address < *old_end && *old_start < absolute_end {
                            return Err(Error::new(ENOEXEC));
                        }
                    }
                    ranges.push((vm_address, absolute_end));
                    base = base.min(vm_address);
                    image_end = image_end.max(absolute_end);
                    if file_offset == 0 && file_size >= 32 + command_bytes {
                        has_header_segment = true;
                    }
                    let mut protection = MapFlags::empty();
                    if initial_protection & 1 != 0 { protection |= MapFlags::PROT_READ; }
                    if initial_protection & 2 != 0 { protection |= MapFlags::PROT_WRITE; }
                    if initial_protection & 4 != 0 { protection |= MapFlags::PROT_EXEC; }
                    segments.push(Section {
                        virtual_address: vm_address,
                        image_size: vm_size,
                        memory_size: mapped_size,
                        raw_offset: file_offset,
                        raw_size: file_size,
                        protection,
                    });
                }
            }
            0x8000_0028 => { // LC_MAIN
                if command_size < 24 || entry_file_offset.is_some() {
                    return Err(Error::new(ENOEXEC));
                }
                let entry_offset = u64_at(&commands, at + 8)?;
                if entry_offset == 0 {
                    return Err(Error::new(ENOEXEC));
                }
                entry_file_offset = Some(entry_offset);
            }
            // These commands require dyld, dependency resolution, relocations,
            // TLS, signing or platform startup services not in the raw profile.
            0x0C | 0x0D | 0x0E | 0x1D | 0x1E | 0x20 | 0x21 | 0x22
            | 0x8000_0018 | 0x8000_001C | 0x8000_001F | 0x8000_0022
            | 0x8000_0023 | 0x8000_0033 | 0x8000_0034 => {
                return Err(Error::new(ENOEXEC));
            }
            // Standard metadata commands which do not participate in the
            // freestanding image's address fixups are accepted.
            0x02 | 0x0B | 0x16 | 0x17 | 0x1B | 0x24 | 0x26 | 0x29 | 0x2A
            | 0x2E | 0x31 | 0x32 => {}
            _ => return Err(Error::new(ENOEXEC)),
        }
        at = command_end;
    }
    if at != commands.len() || !has_header_segment || base == usize::MAX {
        return Err(Error::new(ENOEXEC));
    }
    let image_size = image_end.checked_sub(base).ok_or(Error::new(EOVERFLOW))?;
    if image_size == 0 || image_size > MAX_PE_IMAGE {
        return Err(Error::new(ENOEXEC));
    }
    let stack_base = super::STACK_TOP
        .checked_sub(super::STACK_SIZE)
        .ok_or(Error::new(EOVERFLOW))?;
    if image_end > stack_base {
        return Err(Error::new(ENOEXEC));
    }

    let entry_file_offset = entry_file_offset.ok_or(Error::new(ENOEXEC))?;
    let mut entry = None;
    for segment in &segments {
        let file_end = segment
            .raw_offset
            .checked_add(segment.raw_size as u64)
            .ok_or(Error::new(EOVERFLOW))?;
        if entry_file_offset >= segment.raw_offset && entry_file_offset < file_end {
            if entry.is_some() || !segment.protection.contains(MapFlags::PROT_EXEC) {
                return Err(Error::new(ENOEXEC));
            }
            let delta = usize::try_from(entry_file_offset - segment.raw_offset)
                .map_err(|_| Error::new(EOVERFLOW))?;
            entry = Some(segment.virtual_address.checked_add(delta).ok_or(Error::new(EOVERFLOW))?);
        }
    }
    let entry = entry.ok_or(Error::new(ENOEXEC))?;

    let grants_fd = {
        let current_addrspace_fd = thread_fd.dup_into_upper(b"addrspace")?;
        current_addrspace_fd.dup_into_upper(b"empty")?
    };
    grants_fd.fcntl(syscall::F_SETFD, O_CLOEXEC)?;

    for segment in &segments {
        map_writable(&grants_fd, segment.virtual_address, segment.memory_size)?;
        if segment.raw_size > 0 {
            let raw = read_vec(&image_file, segment.raw_offset, segment.raw_size)?;
            write_remote(&grants_fd, segment.virtual_address, &raw)?;
        }
        mprotect_remote(
            &grants_fd,
            segment.virtual_address,
            segment.memory_size,
            segment.protection,
        )?;
    }

    finish_native_exec(
        image_file,
        thread_fd,
        proc_fd,
        path,
        extrainfo,
        grants_fd,
        entry,
        8,
    )
}

pub(super) fn fexec_macho64(
    image_file: FdGuardUpper,
    thread_fd: &FdGuardUpper,
    proc_fd: &FdGuardUpper,
    path: &[u8],
    args: &[&[u8]],
    envs: &[&[u8]],
    extrainfo: &ExtraInfo,
) -> Result<Option<FexecResult>> {
    launch_macho64(image_file, thread_fd, proc_fd, path, args, envs, extrainfo)
}

/// Dispatch a PE32+ image through Redox's real address-space and process context
/// interfaces. The kernel scheduler and x86-64 context switch enter the original
/// image bytes; unsupported Windows runtime features fail closed.
pub(super) fn fexec_pe32_plus(
    image_file: FdGuardUpper,
    thread_fd: &FdGuardUpper,
    proc_fd: &FdGuardUpper,
    path: &[u8],
    args: &[&[u8]],
    envs: &[&[u8]],
    extrainfo: &ExtraInfo,
) -> Result<Option<FexecResult>> {
    launch_pe(image_file, thread_fd, proc_fd, path, args, envs, extrainfo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_exit_thunks_encode_the_syscall_immediate_at_the_correct_offset() {
        let expected = syscall::SYS_GUEST_EXIT as u32;
        for thunk in [exit_trampoline().unwrap(), exit_process_thunk().unwrap()] {
            assert_eq!(u32::from_le_bytes(thunk[19..23].try_into().unwrap()), expected);
            assert_eq!(&thunk[23..], &[0x0F, 0x05, 0x0F, 0x0B]);
        }
    }

    #[test]
    fn dir64_relocation_rebases_the_image_pointer() {
        let mut image = alloc::vec![0; 0x2000];
        image[0x100..0x104].copy_from_slice(&0x200u32.to_le_bytes());
        image[0x104..0x108].copy_from_slice(&10u32.to_le_bytes());
        image[0x108..0x10A].copy_from_slice(&0xA020u16.to_le_bytes());
        image[0x220..0x228].copy_from_slice(&0x1400_0200u64.to_le_bytes());
        let sections = [Section {
            virtual_address: 0x200,
            image_size: 0x1000,
            memory_size: 0x1000,
            raw_offset: 0,
            raw_size: 0,
            protection: MapFlags::empty(),
        }];

        apply_pe_relocations(
            &mut image,
            &sections,
            0x100,
            10,
            0x1400_0000,
            0x1500_0000,
        )
        .unwrap();

        assert_eq!(u64_at(&image, 0x220).unwrap(), 0x1500_0200);
    }
}
