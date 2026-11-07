//! Restricted native PE32+ entry path for the Redox process model.
//!
//! This loader maps the original image into a fresh Redox address space. It
//! deliberately rejects imports, TLS, delay imports and non-x86-64 images;
//! Windows API/PEB/TEB compatibility belongs to the later Windows runtime.

use super::*;
use alloc::{vec, vec::Vec};

const MAX_PE_IMAGE: usize = 256 * 1024 * 1024;
const MAX_PE_HEADERS: usize = 16 * 1024 * 1024;
const MAX_PE_SECTIONS: usize = 96;

#[derive(Clone, Copy)]
struct Section {
    virtual_address: usize,
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

fn exit_trampoline(proc_fd: usize) -> Result<Vec<u8>> {
    let syscall_number: u32 = syscall::SYS_CALL
        .try_into()
        .map_err(|_| Error::new(ENOEXEC))?;
    let proc_fd: u32 = proc_fd.try_into().map_err(|_| Error::new(EBADF))?;
    let exit_verb: u32 = (ProcCall::Exit as u64)
        .try_into()
        .map_err(|_| Error::new(ENOEXEC))?;
    let mut code = vec![
        0x89, 0xC1,                         // mov ecx, eax (preserve entry return code)
        0xB8, 0, 0, 0, 0,                   // mov eax, SYS_CALL
        0xBF, 0, 0, 0, 0,                   // mov edi, proc fd
        0x48, 0x8D, 0x74, 0x24, 0xF0,       // lea rsi, [rsp - 16] (writable stack request)
        0xC7, 0x06, 0, 0, 0, 0,             // mov dword [rsi], ProcCall::Exit
        0xBA, 16, 0, 0, 0,                  // mov edx, request length
        0x45, 0x31, 0xD2,                   // xor r10d, r10d (flags)
        0x45, 0x31, 0xC0,                   // xor r8d, r8d (metadata pointer)
        0x45, 0x31, 0xC9,                   // xor r9d, r9d (metadata length)
        0x89, 0x4E, 0x08,                   // mov [rsi + 8], ecx (exit status)
        0x0F, 0x05,                         // syscall
        0x0F, 0x0B,                         // ud2 if process-manager exit unexpectedly returns
    ];
    code[3..7].copy_from_slice(&syscall_number.to_le_bytes());
    code[8..12].copy_from_slice(&proc_fd.to_le_bytes());
    code[19..23].copy_from_slice(&exit_verb.to_le_bytes());
    Ok(code)
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
    // This first native profile is intentionally freestanding: argv/env, imports,
    // TLS and Windows runtime structures are not silently fabricated.
    if args.len() > 1 || !envs.is_empty() {
        return Err(Error::new(ENOEXEC));
    }

    let dos = read_vec(&image_file, 0, 64)?;
    if dos.get(0..2) != Some(b"MZ") {
        return Err(Error::new(ENOEXEC));
    }
    let pe_offset = u64::from(u32_at(&dos, 0x3C)?);
    let coff = read_vec(&image_file, pe_offset, 24)?;
    if coff.get(0..4) != Some(b"PE\0\0") {
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
    let file_alignment = usize::try_from(u32_at(&optional, 36)?)
        .map_err(|_| Error::new(EOVERFLOW))?;
    let image_size = usize::try_from(u32_at(&optional, 56)?)
        .map_err(|_| Error::new(EOVERFLOW))?;
    let headers_size = usize::try_from(u32_at(&optional, 60)?)
        .map_err(|_| Error::new(EOVERFLOW))?;
    let directory_count = usize::try_from(u32_at(&optional, 108)?)
        .map_err(|_| Error::new(EOVERFLOW))?;

    if image_base == 0
        || image_base % PAGE_SIZE != 0
        || section_alignment < PAGE_SIZE
        || !section_alignment.is_power_of_two()
        || file_alignment == 0
        || !file_alignment.is_power_of_two()
        || file_alignment > section_alignment
        || image_size == 0
        || image_size > MAX_PE_IMAGE
        || image_size % PAGE_SIZE != 0
        || headers_size == 0
        || headers_size > MAX_PE_HEADERS
        || headers_size > image_size
        || entry_rva == 0
        || entry_rva >= image_size
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

    // Imports, TLS callbacks and delay imports require the Windows runtime.
    if directory_count > 1 {
        if optional_size < 128 {
            return Err(Error::new(ENOEXEC));
        }
        if u32_at(&optional, 120)? != 0 || u32_at(&optional, 124)? != 0 {
            return Err(Error::new(ENOEXEC));
        }
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
            || raw_size > 0 && raw_offset % file_alignment as u64 != 0
        {
            return Err(Error::new(ENOEXEC));
        }
        let virtual_end = virtual_address.checked_add(memory_size).ok_or(Error::new(EOVERFLOW))?;
        if virtual_end > image_size {
            return Err(Error::new(ENOEXEC));
        }
        let mapped_size = page_align(memory_size)?;
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
        if executable && entry_rva >= virtual_address && entry_rva < virtual_end {
            entry_is_executable = true;
        }
        let mut protection = MapFlags::empty();
        if readable { protection |= MapFlags::PROT_READ; }
        if writable { protection |= MapFlags::PROT_WRITE; }
        if executable { protection |= MapFlags::PROT_EXEC; }
        sections.push(Section {
            virtual_address,
            memory_size: mapped_size,
            raw_offset,
            raw_size,
            protection,
        });
    }
    if !entry_is_executable {
        return Err(Error::new(ENOEXEC));
    }

    let grants_fd = {
        let current_addrspace_fd = thread_fd.dup_into_upper(b"addrspace")?;
        current_addrspace_fd.dup_into_upper(b"empty")?
    };
    grants_fd.fcntl(syscall::F_SETFD, O_CLOEXEC)?;

    // Map everything writable but non-executable, populate it, then apply final
    // protections. The original PE file is never modified.
    map_writable(&grants_fd, image_base, header_map_size)?;
    let header_bytes = read_vec(&image_file, 0, headers_size)?;
    write_remote(&grants_fd, image_base, &header_bytes)?;
    mprotect_remote(&grants_fd, image_base, header_map_size, MapFlags::PROT_READ)?;

    for section in &sections {
        let address = image_base
            .checked_add(section.virtual_address)
            .ok_or(Error::new(EOVERFLOW))?;
        map_writable(&grants_fd, address, section.memory_size)?;
        if section.raw_size > 0 {
            let raw = read_vec(&image_file, section.raw_offset, section.raw_size)?;
            write_remote(&grants_fd, address, &raw)?;
        }
        mprotect_remote(&grants_fd, address, section.memory_size, section.protection)?;
    }

    map_writable(
        &grants_fd,
        stack_base,
        super::STACK_SIZE,
    )?;
    let trampoline = exit_trampoline(extrainfo.proc_fd)?;
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
    write_remote(&grants_fd, trampoline_address, &trampoline)?;
    mprotect_remote(
        &grants_fd,
        trampoline_address,
        trampoline_size,
        MapFlags::PROT_READ | MapFlags::PROT_EXEC,
    )?;

    // Windows x64 entry alignment: RSP is 8 mod 16 and the 32-byte home area
    // follows the return address. The return target is a tiny Redox exit stub.
    let stack_pointer = super::STACK_TOP
        .checked_sub(40)
        .ok_or(Error::new(EOVERFLOW))?;
    write_remote(
        &grants_fd,
        stack_pointer,
        &trampoline_address.to_ne_bytes(),
    )?;

    let entry = image_base.checked_add(entry_rva).ok_or(Error::new(EOVERFLOW))?;
    let addrspace_selection_fd = thread_fd.dup_into_upper(b"current-addrspace")?;
    addrspace_selection_fd.write(&create_set_addr_space_buf(
        grants_fd.as_raw_fd(),
        entry,
        stack_pointer,
    ))?;

    let mut name = [0u8; 32];
    let name_len = path.len().min(name.len());
    name[..name_len].copy_from_slice(&path[..name_len]);
    let _ = sys_call_wo(
        proc_fd.as_raw_fd(),
        &name,
        CallFlags::empty(),
        &[ProcCall::Rename as u64],
    );
    let _ = sys_call(
        proc_fd.as_raw_fd(),
        CallFlags::empty(),
        &[ProcCall::DisableSetpgid as u64],
    );
    if let Ok(sighandler_fd) = thread_fd.dup_into_upper(b"sighandler") {
        let _ = sighandler_fd.write(&SetSighandlerData {
            user_handler: 0,
            excp_handler: 0,
            thread_control_addr: 0,
            proc_control_addr: 0,
        });
    }

    if extrainfo.same_process {
        let _siglock = crate::signal::tmp_disable_signals();
        let fds_to_close = {
            let guard = crate::current_filetable();
            guard
                .iter()
                .filter_map(|(fd, flags)| {
                    if fd == addrspace_selection_fd.as_raw_fd() {
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
        let _ = filetable_fd.call_wo(
            fds_bytes,
            CallFlags::empty(),
            &[syscall::FileTableVerb::Close as u64],
        );
        {
            let mut guard = crate::current_filetable();
            for fd in fds_to_close {
                let _ = guard.remove(fd);
            }
        }
        unsafe { super::deactivate_tcb(thread_fd)?; }
        let old_filetable_fd = {
            let mut guard = crate::current_filetable();
            guard.take()
        };
        drop(old_filetable_fd);
        drop(addrspace_selection_fd);
        unreachable!("closing current-addrspace switches into the new PE image");
    }

    drop(addrspace_selection_fd);
    Ok(None)
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
