use core::{cell::SyncUnsafeCell, cmp, fmt::Debug, ops::Range};

use crate::{
    DYNAMIC_PROC_INFO, RtTcb, StaticProcInfo,
    arch::*,
    auxv_defs::*,
    read_proc_meta,
    sys::{fstat, open, sys_call, sys_call_wo},
};
use redox_protocols::protocol::{O_CLOEXEC, ProcCall, ThreadCall};

use alloc::{boxed::Box, vec};

use goblin::elf::header::ET_DYN;
//TODO: allow use of either 32-bit or 64-bit programs
#[cfg(target_pointer_width = "32")]
use goblin::elf32::{
    header::Header,
    program_header::program_header32::{PF_R, PF_W, PF_X, PT_INTERP, PT_LOAD, ProgramHeader},
};
#[cfg(target_pointer_width = "64")]
use goblin::elf64::{
    header::Header,
    program_header::program_header64::{PF_R, PF_W, PF_X, PT_INTERP, PT_LOAD, ProgramHeader},
};

#[cfg(target_arch = "x86_64")]
mod trion;

use syscall::{
    CallFlags, GrantDesc, GrantFlags, MAP_FIXED_NOREPLACE, MAP_SHARED, Map, PAGE_SIZE, PROT_EXEC,
    PROT_READ, PROT_WRITE, SetSighandlerData,
    error::*,
    flag::{MapFlags, SEEK_SET},
};

pub enum FexecResult {
    Interp {
        path: Box<[u8]>,
        interp_override: InterpOverride,
    },
}
pub struct InterpOverride {
    phdrs_vaddr: usize,
    at_entry: usize,
    at_phnum: usize,
    at_phent: usize,
    name: Box<[u8]>,
    min_mmap_addr: usize,
    grants_fd: usize,
}

pub struct ExtraInfo<'a> {
    pub cwd: Option<&'a [u8]>,
    // POSIX states that while sigactions are reset, ignored sigactions will remain ignored.
    pub sigignmask: u64,
    // POSIX also states that the sigprocmask must be preserved across execs.
    pub sigprocmask: u64,
    /// File mode creation mask (POSIX)
    pub umask: u32,
    /// Thread handle
    pub thr_fd: usize,
    /// Process handle
    pub proc_fd: usize,
    /// Namespace handle
    pub ns_fd: Option<usize>,
    /// CWD handle
    pub cwd_fd: Option<usize>,
    /// Filetable handle
    pub filetable_fd: Option<usize>,
    /// Explicit descriptors the supervisor permits a restricted native guest to use.
    /// Only pre-opened descriptors are delegated; the kernel validates each one.
    pub guest_capability_fds: Option<&'a [usize]>,
    /// If the process for which the image is to be loaded the same as the currently running process
    pub same_process: bool,
}

#[expect(clippy::too_many_arguments)]
fn checked_page_align(value: usize) -> Result<usize> {
    value
        .checked_add(PAGE_SIZE - 1)
        .map(|aligned| aligned / PAGE_SIZE * PAGE_SIZE)
        .ok_or(Error::new(EOVERFLOW))
}

pub fn fexec_impl(
    image_file: FdGuardUpper,
    thread_fd: &FdGuardUpper,
    proc_fd: &FdGuardUpper,
    path: &[u8],
    args: &[&[u8]],
    envs: &[&[u8]],
    extrainfo: &ExtraInfo,
    interp_override: Option<InterpOverride>,
) -> Result<Option<FexecResult>> {
    // Native PE/Mach-O entry support is x86-64 only. Other Redox architectures
    // continue through their existing executable-format path and reject these files.
    #[cfg(target_arch = "x86_64")]
    {
        let mut format_magic = [0u8; 4];
        pread_all(&image_file, 0, &mut format_magic)?;
        if format_magic.starts_with(b"MZ") {
            return trion::fexec_pe32_plus(
                image_file,
                thread_fd,
                proc_fd,
                path,
                args,
                envs,
                extrainfo,
            );
        }
        if format_magic == [0xCF, 0xFA, 0xED, 0xFE] {
            return trion::fexec_macho64(
                image_file,
                thread_fd,
                proc_fd,
                path,
                args,
                envs,
                extrainfo,
            );
        }
    }

    // Here, we do the minimum part of loading an application, which is what the kernel used to do.
    // We load the executable into memory (albeit at different offsets in this executable), fix
    // some misalignments, and then switch address space.

    let mut header_bytes = [0_u8; size_of::<Header>()];
    pread_all(&image_file, 0, &mut header_bytes)?;
    let header = Header::from_bytes(&header_bytes);

    let expected_class = if cfg!(target_pointer_width = "64") { 2 } else { 1 };
    let machine_matches = if cfg!(target_arch = "x86_64") {
        header.e_machine == 62
    } else if cfg!(target_arch = "aarch64") {
        header.e_machine == 183
    } else if cfg!(target_arch = "riscv64") {
        header.e_machine == 243
    } else if cfg!(target_arch = "x86") {
        header.e_machine == 3
    } else {
        false
    };
    if header.e_ident[..4] != [0x7F, 0x45, 0x4C, 0x46]
        || header.e_ident[4] != expected_class
        || header.e_ident[5] != 1
        || header.e_ident[6] != 1
        || !machine_matches
        || !matches!(header.e_type, 2 | ET_DYN)
        || header.e_version != 1
        || usize::from(header.e_ehsize) != size_of::<Header>()
    {
        return Err(Error::new(ENOEXEC));
    }

    let grants_fd = if let Some(interp) = interp_override.as_ref() {
        FdGuard::new(interp.grants_fd).to_upper()?
    } else {
        let current_addrspace_fd = thread_fd.dup_into_upper(b"addrspace")?;
        current_addrspace_fd.dup_into_upper(b"empty")?
    };
    grants_fd.fcntl(syscall::F_SETFD, O_CLOEXEC)?;

    // Never allow more than 1 MiB of program headers.
    const MAX_PH_SIZE: usize = 1024 * 1024;
    let phentsize = usize::from(header.e_phentsize);
    let phnum = usize::from(header.e_phnum);
    if phentsize != size_of::<ProgramHeader>() || phnum == 0 {
        return Err(Error::new(ENOEXEC));
    }
    let pheaders_size = phentsize
        .checked_mul(phnum)
        .and_then(|size| size.checked_add(size_of::<Header>()))
        .ok_or(Error::new(EOVERFLOW))?;

    if pheaders_size > MAX_PH_SIZE {
        return Err(Error::new(E2BIG));
    }
    let mut phs_raw = vec![0_u8; pheaders_size];
    phs_raw[..size_of::<Header>()].copy_from_slice(&header_bytes);
    let phs = &mut phs_raw[size_of::<Header>()..];

    // TODO: Remove clone, but this would require more as_refs and as_muts
    let mut min_mmap_addr = interp_override
        .as_ref()
        .map(|interp| interp.min_mmap_addr)
        .unwrap_or(PAGE_SIZE);
    let mut update_min_mmap_addr = |addr: usize, size: usize| -> Result<()> {
        let end = addr.checked_add(size).ok_or(Error::new(EOVERFLOW))?;
        min_mmap_addr = cmp::max(min_mmap_addr, checked_page_align(end)?);
        Ok(())
    };

    #[cfg(target_pointer_width = "64")]
    let header_e_phoff = header.e_phoff;
    #[cfg(target_pointer_width = "32")]
    let header_e_phoff = u64::from(header.e_phoff);
    pread_all(&image_file, header_e_phoff, phs).map_err(|_| Error::new(EIO))?;

    let mut span: Option<Range<usize>> = None;
    let mut load_ranges = Vec::<Range<usize>>::new();
    for ph_idx in 0..phnum {
        let ph_start = ph_idx.checked_mul(phentsize).ok_or(Error::new(EOVERFLOW))?;
        let ph_end = ph_start.checked_add(phentsize).ok_or(Error::new(EOVERFLOW))?;
        let ph_bytes = phs.get(ph_start..ph_end).ok_or(Error::new(ENOEXEC))?;
        let segment: &ProgramHeader =
            plain::from_bytes(ph_bytes).map_err(|_| Error::new(ENOEXEC))?;
        if segment.p_type != PT_LOAD {
            continue;
        }

        let virtual_address = usize::try_from(segment.p_vaddr)
            .map_err(|_| Error::new(EOVERFLOW))?;
        let memory_size = usize::try_from(segment.p_memsz)
            .map_err(|_| Error::new(EOVERFLOW))?;
        let file_size = usize::try_from(segment.p_filesz)
            .map_err(|_| Error::new(EOVERFLOW))?;
        let file_offset = usize::try_from(segment.p_offset)
            .map_err(|_| Error::new(EOVERFLOW))?;
        let alignment = usize::try_from(segment.p_align)
            .map_err(|_| Error::new(EOVERFLOW))?;
        if memory_size == 0
            || file_size > memory_size
            || alignment > 1 && (!alignment.is_power_of_two()
                || virtual_address % alignment != file_offset % alignment)
            || virtual_address % PAGE_SIZE != file_offset % PAGE_SIZE
        {
            return Err(Error::new(ENOEXEC));
        }
        file_offset.checked_add(file_size).ok_or(Error::new(EOVERFLOW))?;
        let page_offset = virtual_address % PAGE_SIZE;
        let map_start = virtual_address - page_offset;
        let map_size = checked_page_align(
            memory_size.checked_add(page_offset).ok_or(Error::new(EOVERFLOW))?
        )?;
        let map_end = map_start.checked_add(map_size).ok_or(Error::new(EOVERFLOW))?;
        let range = map_start..map_end;
        if load_ranges.iter().any(|existing| {
            range.start < existing.end && existing.start < range.end
        }) {
            // Segment page overlap is not yet supported by the mapper. Reject it
            // rather than allowing a later MAP_FIXED to replace earlier contents.
            return Err(Error::new(ENOEXEC));
        }
        load_ranges.push(range.clone());

        span = Some(if let Some(existing) = span {
            existing.start.min(range.start)..existing.end.max(range.end)
        } else {
            range
        });
    }
    let span = span.ok_or(Error::new(ENOEXEC))?;
    let span_size = span.end.checked_sub(span.start).ok_or(Error::new(EOVERFLOW))?;
    if span_size == 0 || span_size % PAGE_SIZE != 0 {
        return Err(Error::new(ENOEXEC));
    }

    let base_addr = if header.e_type == ET_DYN {
        // PIE
        let addr = mmap_anon_remote(&grants_fd, 0, 0, span_size, MapFlags::PROT_NONE)?;
        update_min_mmap_addr(addr, span_size)?;
        addr
    } else {
        mmap_anon_remote(
            &grants_fd,
            0,
            span.start,
            span_size,
            MapFlags::MAP_FIXED_NOREPLACE,
        )?;
        update_min_mmap_addr(span.start, span_size)?;
        0
    };

    let mut phdrs_vaddr = 0;
    let mut interpreter = None;

    for ph_idx in 0..phnum {
        let ph_bytes = &phs[ph_idx * phentsize..(ph_idx + 1) * phentsize];
        let segment: &ProgramHeader =
            plain::from_bytes(ph_bytes).map_err(|_| Error::new(EINVAL))?;
        let mut flags = if segment.p_flags & PF_R == PF_R {
            syscall::PROT_READ
        } else {
            syscall::PROT_NONE
        };

        // W ^ X. If it is executable, do not allow it to be writable, even if requested
        if segment.p_flags & PF_X == PF_X {
            flags |= syscall::PROT_EXEC;
        } else if segment.p_flags & PF_W == PF_W {
            flags |= syscall::PROT_WRITE;
        }

        match segment.p_type {
            // PT_INTERP must come before any PT_LOAD, so we don't have to iterate twice.
            PT_INTERP => {
                let mut interp = vec![0_u8; segment.p_filesz as usize];
                #[cfg(target_pointer_width = "64")]
                let segment_p_offset = segment.p_offset;
                #[cfg(target_pointer_width = "32")]
                let segment_p_offset = u64::from(segment.p_offset);
                pread_all(&image_file, segment_p_offset, &mut interp)?;

                interpreter = Some(interp.into_boxed_slice());
            }
            PT_LOAD => {
                let virtual_address = usize::try_from(segment.p_vaddr)
                    .map_err(|_| Error::new(EOVERFLOW))?;
                let memory_size = usize::try_from(segment.p_memsz)
                    .map_err(|_| Error::new(EOVERFLOW))?;
                let filesz = usize::try_from(segment.p_filesz)
                    .map_err(|_| Error::new(EOVERFLOW))?;
                let file_offset = u64::try_from(segment.p_offset)
                    .map_err(|_| Error::new(EOVERFLOW))?;
                if memory_size == 0 || filesz > memory_size {
                    return Err(Error::new(ENOEXEC));
                }
                let voff = virtual_address % PAGE_SIZE;
                let vaddr = virtual_address - voff;
                let mapped_size = checked_page_align(
                    memory_size.checked_add(voff).ok_or(Error::new(EOVERFLOW))?
                )?;
                let total_page_count = mapped_size / PAGE_SIZE;

                // The case where segments overlap so that they share one page, is not handled.
                // TODO: Should it be?

                if segment.p_filesz > segment.p_memsz {
                    return Err(Error::new(ENOEXEC));
                }

                mmap_anon_remote(
                    &grants_fd,
                    0,
                    base_addr.checked_add(vaddr).ok_or(Error::new(EOVERFLOW))?,
                    total_page_count.checked_mul(PAGE_SIZE).ok_or(Error::new(EOVERFLOW))?,
                    flags | MapFlags::MAP_FIXED,
                )?;

                let segment_file_end = segment
                    .p_offset
                    .checked_add(segment.p_filesz)
                    .ok_or(Error::new(EOVERFLOW))?;
                if segment.p_offset <= header.e_phoff && header.e_phoff < segment_file_end {
                    let phdr_offset = usize::try_from(header.e_phoff - segment.p_offset)
                        .map_err(|_| Error::new(EOVERFLOW))?;
                    phdrs_vaddr = base_addr
                        .checked_add(virtual_address)
                        .and_then(|address| address.checked_add(phdr_offset))
                        .ok_or(Error::new(EOVERFLOW))?;
                }

                // TODO: Attempt to mmap with MAP_PRIVATE directly from the image file instead.

                if filesz > 0 {
                    let (_guard, dst_memory) = unsafe {
                        MmapGuard::map_mut_anywhere(
                            &grants_fd,
                            base_addr.checked_add(vaddr).ok_or(Error::new(EOVERFLOW))?, // offset
                            checked_page_align(voff.checked_add(filesz).ok_or(Error::new(EOVERFLOW))?)?, // size
                        )?
                    };
                    #[cfg(target_pointer_width = "64")]
                    let segment_p_offset = segment.p_offset;
                    #[cfg(target_pointer_width = "32")]
                    let segment_p_offset = u64::from(segment.p_offset);
                    pread_all(
                        &image_file,
                        file_offset,
                        dst_memory.get_mut(voff..voff.checked_add(filesz).ok_or(Error::new(EOVERFLOW))?)
                            .ok_or(Error::new(ENOEXEC))?,
                    )?;
                }
            }
            _ => continue,
        }
    }

    if let Some(interpreter_path) = interpreter {
        return Ok(Some(FexecResult::Interp {
            path: interpreter_path,
            interp_override: InterpOverride {
                at_entry: base_addr + header.e_entry as usize,
                at_phnum: phnum,
                at_phent: phentsize,
                phdrs_vaddr,
                name: path.into(),
                min_mmap_addr,
                grants_fd: grants_fd.take(),
            },
        }));
    }

    mmap_anon_remote(
        &grants_fd,
        0,
        STACK_TOP - STACK_SIZE,
        STACK_SIZE,
        MapFlags::PROT_READ | MapFlags::PROT_WRITE | MapFlags::MAP_FIXED_NOREPLACE,
    )?;

    let mut sp = STACK_TOP;
    let mut stack_page = Option::<MmapGuard>::None;

    let mut push = |word: usize| -> Result<()> {
        let old_page_no = sp / PAGE_SIZE;
        sp = sp
            .checked_sub(size_of::<usize>())
            .filter(|next| *next >= STACK_TOP - STACK_SIZE)
            .ok_or(Error::new(E2BIG))?;
        let new_page_no = sp / PAGE_SIZE;
        let new_page_off = sp % PAGE_SIZE;

        let page = if let Some(ref mut page) = stack_page
            && old_page_no == new_page_no
        {
            page
        } else if let Some(ref mut stack_page) = stack_page {
            stack_page.remap(new_page_no * PAGE_SIZE, PROT_READ | PROT_WRITE)?;
            stack_page
        } else {
            let new = MmapGuard::map(
                &grants_fd,
                &Map {
                    offset: new_page_no * PAGE_SIZE,
                    size: PAGE_SIZE,
                    flags: PROT_READ | PROT_WRITE,
                    address: 0, // let kernel decide
                },
            )?;

            stack_page.insert(new)
        };

        unsafe {
            page.as_mut_ptr_slice()
                .as_mut_ptr()
                .add(new_page_off)
                .cast::<usize>()
                .write(word);
        }

        Ok(())
    };

    push(0)?;
    push(AT_NULL)?;
    if let Some(ref r#override) = interp_override {
        push(r#override.at_entry)?;
        push(AT_ENTRY)?;
        push(base_addr)?;
        push(AT_BASE)?;
        push(r#override.phdrs_vaddr)?;
        push(AT_PHDR)?;
    } else {
        push(base_addr + header.e_entry as usize)?;
        push(AT_ENTRY)?;
        push(phdrs_vaddr)?;
        push(AT_PHDR)?;
    }
    push(
        interp_override
            .as_ref()
            .map_or(header.e_phnum as usize, |o| o.at_phnum),
    )?;
    push(AT_PHNUM)?;
    push(
        interp_override
            .as_ref()
            .map_or(header.e_phentsize as usize, |o| o.at_phent),
    )?;
    push(AT_PHENT)?;

    let mut total_args_envs_auxvpointee_size = args
        .iter()
        .chain(envs.iter())
        .try_fold(0usize, |total, item| {
            total.checked_add(item.len())
                .and_then(|size| size.checked_add(1))
                .ok_or(Error::new(E2BIG))
        })?;
    if let Some(cwd) = extrainfo.cwd {
        total_args_envs_auxvpointee_size = total_args_envs_auxvpointee_size
            .checked_add(cwd.len())
            .and_then(|size| size.checked_add(1))
            .ok_or(Error::new(E2BIG))?;
    }
    let args_envs_size_aligned = checked_page_align(
        total_args_envs_auxvpointee_size.max(1)
    )?;
    let target_args_env_address = mmap_anon_remote(
        &grants_fd,
        0,
        0,
        args_envs_size_aligned,
        MapFlags::PROT_READ | MapFlags::PROT_WRITE,
    )?;
    update_min_mmap_addr(target_args_env_address, args_envs_size_aligned)?;

    let mut offset = 0;

    let mut argc = 0;

    {
        let mut append = |source_slice: &[u8]| -> Result<usize> {
            let address = target_args_env_address
                .checked_add(offset)
                .ok_or(Error::new(EOVERFLOW))?;
            let end_offset = offset
                .checked_add(source_slice.len())
                .and_then(|end| end.checked_add(1))
                .filter(|end| *end <= total_args_envs_auxvpointee_size)
                .ok_or(Error::new(E2BIG))?;

            if !source_slice.is_empty() {
                let containing_page = address / PAGE_SIZE * PAGE_SIZE;
                let displacement = address - containing_page;
                let size = source_slice.len()
                    .checked_add(displacement)
                    .ok_or(Error::new(EOVERFLOW))?;
                let aligned_size = checked_page_align(size)?;
                let (_guard, memory) = unsafe {
                    MmapGuard::map_mut_anywhere(&grants_fd, containing_page, aligned_size)?
                };
                let end = displacement
                    .checked_add(source_slice.len())
                    .ok_or(Error::new(EOVERFLOW))?;
                memory.get_mut(displacement..end)
                    .ok_or(Error::new(EFAULT))?
                    .copy_from_slice(source_slice);
            }

            offset = end_offset;
            Ok(address)
        };

        if let Some(cwd) = extrainfo.cwd {
            push(append(cwd)?)?;
            push(AT_REDOX_INITIAL_CWD_PTR)?;
            push(cwd.len())?;
            push(AT_REDOX_INITIAL_CWD_LEN)?;
        }

        #[cfg(target_pointer_width = "32")]
        {
            push((extrainfo.sigignmask >> 32) as usize)?;
            push(AT_REDOX_INHERITED_SIGIGNMASK_HI)?;
        }
        push(extrainfo.sigignmask as usize)?;
        push(AT_REDOX_INHERITED_SIGIGNMASK)?;
        #[cfg(target_pointer_width = "32")]
        {
            push((extrainfo.sigprocmask >> 32) as usize)?;
            push(AT_REDOX_INHERITED_SIGPROCMASK_HI)?;
        }
        push(extrainfo.sigprocmask as usize)?;
        push(AT_REDOX_INHERITED_SIGPROCMASK)?;

        push(extrainfo.umask as usize)?;
        push(AT_REDOX_UMASK)?;

        push(extrainfo.thr_fd)?;
        push(AT_REDOX_THR_FD)?;
        push(extrainfo.proc_fd)?;
        push(AT_REDOX_PROC_FD)?;
        push(extrainfo.ns_fd.unwrap_or(usize::MAX))?;
        push(AT_REDOX_NS_FD)?;
        push(extrainfo.cwd_fd.unwrap_or(usize::MAX))?;
        push(AT_REDOX_CWD_FD)?;
        push(extrainfo.filetable_fd.unwrap_or(usize::MAX))?;
        push(AT_REDOX_FILETABLE_FD)?;

        push(0)?;

        for env in envs.iter().rev() {
            push(append(env)?)?;
        }

        push(0)?;

        for arg in args.iter().rev() {
            push(append(arg)?)?;
            argc += 1;
        }
    }

    push(argc)?;

    if let Ok(sighandler_fd) = thread_fd.dup_into_upper(b"sighandler") {
        let _ = sighandler_fd.write(&SetSighandlerData {
            user_handler: 0,
            excp_handler: 0,
            thread_control_addr: 0,
            proc_control_addr: 0,
        });
        // TODO: sync with procmgr
    }

    // TODO: Restore old name if exec failed?
    {
        let mut buf = [0; 32];
        let new_name = interp_override.as_ref().map_or(path, |o| &o.name);
        let len = new_name.len().min(32);
        buf[..len].copy_from_slice(&new_name[..len]);
        let _ = sys_call_wo(
            proc_fd.as_raw_fd(),
            &buf,
            CallFlags::empty(),
            &[ProcCall::Rename as u64],
        );
    }

    // TODO: Error handling
    let _ = sys_call(
        proc_fd.as_raw_fd(),
        CallFlags::empty(),
        &[ProcCall::DisableSetpgid as u64],
    );

    if interp_override.is_some() {
        let mmap_min_fd = grants_fd.dup_into_upper(b"mmap-min-addr")?;
        let _ = mmap_min_fd.write(&usize::to_ne_bytes(min_mmap_addr));
    }

    let addrspace_selection_fd = thread_fd.dup_into_upper(b"current-addrspace")?;

    let _ = addrspace_selection_fd.write(&create_set_addr_space_buf(
        grants_fd.as_raw_fd(),
        base_addr + header.e_entry as usize,
        sp,
    ));

    // Close all O_CLOEXEC file descriptors. TODO: close_range?
    if extrainfo.same_process {
        // NOTE: This approach of implementing O_CLOEXEC will not work in multithreaded
        // scenarios. While execve() is undefined according to POSIX if there exist sibling
        // threads, it could still be allowed by keeping certain file descriptors and instead
        // set the active file table.
        let _siglock = crate::signal::tmp_disable_signals();
        let fds_to_close = {
            let guard = crate::current_filetable();
            let mut fds = alloc::vec::Vec::new();
            for (fd, flags) in guard.iter() {
                if fd == addrspace_selection_fd.as_raw_fd() {
                    continue; // Will be closed below
                }

                if flags & O_CLOEXEC == O_CLOEXEC || fd == image_file.as_raw_fd() {
                    fds.push(fd);
                }
            }

            fds
        };

        let fds_to_close_bytes: &[u8] = unsafe {
            core::slice::from_raw_parts(
                fds_to_close.as_ptr() as *mut u8,
                fds_to_close.len() * core::mem::size_of::<usize>(),
            )
        };

        {
            let filetable_fd = thread_fd.dup_into_upper(b"filetable-binary")?;
            let _ = filetable_fd.call_wo(
                fds_to_close_bytes,
                CallFlags::empty(),
                &[syscall::FileTableVerb::Close as u64],
            );
        }
        {
            let mut guard = crate::current_filetable();
            for fd in fds_to_close {
                let _ = guard.remove(fd);
            }
        }

        unsafe {
            deactivate_tcb(thread_fd)?;
        }

        let old_filetable_fd = {
            let mut guard = crate::current_filetable();
            guard.take()
        };
        drop(old_filetable_fd);

        // Dropping this FD will cause the address space switch.
        drop(addrspace_selection_fd);
        unreachable!();
    } else {
        Ok(None)
    }
}
fn write_usizes<const N: usize>(fd: &FdGuardUpper, usizes: [usize; N]) -> Result<usize> {
    fd.write(unsafe { plain::as_bytes(&usizes) })
}
pub fn mmap_remote(
    addrspace_fd: &FdGuardUpper,
    fd: &FdGuardUpper,
    offset: usize,
    dst_addr: usize,
    len: usize,
    flags: MapFlags,
) -> Result<usize> {
    write_usizes(
        addrspace_fd,
        [
            // op
            syscall::flag::ADDRSPACE_OP_MMAP,
            // fd
            fd.as_raw_fd(),
            // "offset"
            offset,
            // address
            dst_addr,
            // size
            len,
            // flags
            flags.bits(),
        ],
    )
}
pub fn mmap_anon_remote(
    addrspace_fd: &FdGuardUpper,
    offset: usize,
    dst_addr: usize,
    len: usize,
    flags: MapFlags,
) -> Result<usize> {
    write_usizes(
        addrspace_fd,
        [
            // op
            syscall::flag::ADDRSPACE_OP_MMAP,
            // fd
            !0,
            // "offset"
            offset,
            // address
            dst_addr,
            // size
            len,
            // flags
            flags.bits(),
        ],
    )
}
pub fn mprotect_remote(
    addrspace_fd: &FdGuardUpper,
    addr: usize,
    len: usize,
    flags: MapFlags,
) -> Result<()> {
    write_usizes(
        addrspace_fd,
        [
            // op
            syscall::flag::ADDRSPACE_OP_MPROTECT,
            // address
            addr,
            // size
            len,
            // flags
            flags.bits(),
        ],
    )?;
    Ok(())
}
pub fn munmap_remote(addrspace_fd: &FdGuardUpper, addr: usize, len: usize) -> Result<()> {
    write_usizes(
        addrspace_fd,
        [
            // op
            syscall::flag::ADDRSPACE_OP_MUNMAP,
            // address
            addr,
            // size
            len,
        ],
    )?;
    Ok(())
}
pub fn munmap_transfer(
    src: &FdGuardUpper,
    dst: &FdGuardUpper,
    src_addr: usize,
    dst_addr: usize,
    len: usize,
    flags: MapFlags,
) -> Result<()> {
    write_usizes(
        dst,
        [
            // op
            syscall::flag::ADDRSPACE_OP_TRANSFER,
            // fd
            src.as_raw_fd(),
            // "offset" (source address)
            src_addr,
            // address
            dst_addr,
            // size
            len,
            // flags
            (flags | MapFlags::MAP_FIXED_NOREPLACE).bits(),
        ],
    )?;
    Ok(())
}
fn pread_all(fd: &FdGuardUpper, offset: u64, buf: &mut [u8]) -> Result<()> {
    fd.lseek(offset as isize, SEEK_SET)?;

    let mut total_bytes_read = 0;

    while total_bytes_read < buf.len() {
        total_bytes_read += match fd.read(&mut buf[total_bytes_read..])? {
            0 => return Err(Error::new(ENOEXEC)),
            bytes_read => bytes_read,
        }
    }
    Ok(())
}

pub struct MmapGuard<'a> {
    fd: &'a FdGuardUpper,
    base: usize,
    size: usize,
}
impl<'a> MmapGuard<'a> {
    pub fn map(fd: &'a FdGuardUpper, map: &Map) -> Result<Self> {
        let base = unsafe { syscall::fmap(fd.as_raw_fd(), map)? };
        Ok(Self {
            fd,
            size: map.size,
            base,
        })
    }
    pub fn remap(&mut self, offset: usize, mut flags: MapFlags) -> Result<()> {
        flags.remove(MapFlags::MAP_FIXED_NOREPLACE);
        flags.insert(MapFlags::MAP_FIXED);

        let _new_base = unsafe {
            syscall::fmap(
                self.fd.as_raw_fd(),
                &Map {
                    offset,
                    size: self.size,
                    flags,
                    address: self.base,
                },
            )?
        };

        Ok(())
    }
    #[expect(clippy::mut_from_ref)]
    pub unsafe fn map_mut_anywhere(
        fd: &'a FdGuardUpper,
        offset: usize,
        size: usize,
    ) -> Result<(Self, &'a mut [u8])> {
        let mut this = Self::map(
            fd,
            &Map {
                size,
                offset,
                address: 0,
                flags: PROT_READ | PROT_WRITE,
            },
        )?;
        let slice = unsafe { &mut *this.as_mut_ptr_slice() };

        Ok((this, slice))
    }
    pub fn addr(&self) -> usize {
        self.base
    }
    #[expect(clippy::len_without_is_empty, reason = "this len() is not costly")]
    pub fn len(&self) -> usize {
        self.size
    }
    pub fn as_mut_ptr_slice(&mut self) -> *mut [u8] {
        core::ptr::slice_from_raw_parts_mut(self.base as *mut u8, self.size)
    }
    pub fn take(mut self) {
        self.size = 0;
    }
}
impl<'a> Drop for MmapGuard<'a> {
    fn drop(&mut self) {
        if self.size != 0 {
            let _ = unsafe { syscall::funmap(self.base, self.size) };
        }
    }
}

pub(crate) struct FileBufReader {
    fd: usize,
    buf: [u8; 8192],
    pos: usize,
    cap: usize,
}

impl FileBufReader {
    pub fn from_fd(fd: usize) -> FileBufReader {
        FileBufReader {
            fd,
            buf: [0; 8192],
            pos: 0,
            cap: 0,
        }
    }
}

impl FileBufReader {
    pub fn read_le_u64(&mut self) -> Result<Option<u64>> {
        if self.pos >= self.cap {
            debug_assert!(self.pos == self.cap);
            self.cap = syscall::read(self.fd, &mut self.buf)?;
            self.pos = 0;
        }

        if self.cap == 0 {
            return Ok(None);
        }

        if self.cap - self.pos < 8 {
            unreachable!();
        }

        let num = u64::from_le_bytes(self.buf[self.pos..self.pos + 8].try_into().unwrap());

        self.pos += 8;

        Ok(Some(num))
    }
}
#[repr(transparent)]
pub struct FdGuard<const UPPER: bool = false> {
    fd: usize,
}
pub type FdGuardUpper = FdGuard<true>;
impl FdGuard<false> {
    #[inline]
    pub fn new(fd: usize) -> Self {
        Self { fd }
    }
    #[inline]
    pub fn open<T: AsRef<str>>(path: T, flags: usize) -> Result<Self> {
        open(path, flags).map(Self::new)
    }

    #[inline]
    pub fn open_into_upper<T: AsRef<str>>(path: T, flags: usize) -> Result<FdGuardUpper> {
        crate::sys::open_into_upper(path, flags)
            .map(FdGuard::new)?
            .to_upper()
    }

    #[inline]
    pub fn to_upper(self) -> Result<FdGuardUpper> {
        // Move to upper table if necessary
        let fd = if self.fd & syscall::UPPER_FDTBL_TAG == 0 {
            //TODO: use F_DUPFD_CLOEXEC?
            let fd = crate::sys::fcntl(self.fd, syscall::F_DUPFD, syscall::UPPER_FDTBL_TAG)?;
            drop(self);
            fd
        } else {
            self.take()
        };
        Ok(FdGuard::<true> { fd })
    }

    // Not implemented for UPPER to prevent misuse
    #[inline]
    pub fn as_c_fd(&self) -> Option<i32> {
        i32::try_from(self.fd).ok()
    }
}
impl<const UPPER: bool> FdGuard<UPPER> {
    #[inline]
    pub fn openat<T: AsRef<str>>(
        &self,
        path: T,
        flags: usize,
        fcntl_flags: usize,
    ) -> Result<FdGuard<false>> {
        crate::sys::openat(self.fd, path, flags, fcntl_flags).map(FdGuard::new)
    }
    #[inline]
    pub fn openat_into_upper<T: AsRef<str>>(
        &self,
        path: T,
        flags: usize,
        fcntl_flags: usize,
    ) -> Result<FdGuardUpper> {
        crate::sys::openat_into_upper(self.fd, path, flags, fcntl_flags)
            .map(FdGuard::new)?
            .to_upper()
    }

    #[inline]
    pub fn dup(&self, buf: &[u8]) -> Result<FdGuard<false>> {
        crate::sys::dup(self.fd, buf).map(FdGuard::new)
    }

    #[inline]
    pub fn dup_into_upper(&self, buf: &[u8]) -> Result<FdGuardUpper> {
        crate::sys::dup_into_upper(self.fd, buf)
            .map(FdGuard::new)?
            .to_upper()
    }

    #[inline]
    pub fn fcntl(&self, cmd: usize, arg: usize) -> Result<usize> {
        crate::sys::fcntl(self.fd, cmd, arg)
    }

    #[inline]
    pub fn fstat(&self, stat: &mut syscall::Stat) -> Result<usize> {
        fstat(self.fd, stat)
    }

    #[inline]
    pub fn lseek(&self, offset: isize, whence: usize) -> Result<usize> {
        syscall::lseek(self.fd, offset, whence)
    }

    #[inline]
    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        syscall::read(self.fd, buf)
    }

    #[inline]
    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        syscall::write(self.fd, buf)
    }

    #[inline]
    pub fn call_ro(&self, payload: &mut [u8], flags: CallFlags, metadata: &[u64]) -> Result<usize> {
        crate::sys::sys_call_ro(self.fd, payload, flags, metadata)
    }

    #[inline]
    pub fn call_wo(&self, payload: &[u8], flags: CallFlags, metadata: &[u64]) -> Result<usize> {
        crate::sys::sys_call_wo(self.fd, payload, flags, metadata)
    }

    #[inline]
    pub fn call_rw(&self, payload: &mut [u8], flags: CallFlags, metadata: &[u64]) -> Result<usize> {
        crate::sys::sys_call_rw(self.fd, payload, flags, metadata)
    }

    #[inline]
    pub fn as_raw_fd(&self) -> usize {
        self.fd
    }

    #[inline]
    pub fn take(self) -> usize {
        let fd = self.fd;
        core::mem::forget(self);
        fd
    }
}
impl<const UPPER: bool> Drop for FdGuard<UPPER> {
    #[inline]
    fn drop(&mut self) {
        let _ = crate::sys::close(self.fd);
    }
}
impl Debug for FdGuard<false> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[fd {}]", self.fd)
    }
}
impl Debug for FdGuardUpper {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[fd upper {}]", self.fd & !syscall::UPPER_FDTBL_TAG)
    }
}
pub fn create_set_addr_space_buf(
    space: usize,
    ip: usize,
    sp: usize,
) -> [u8; size_of::<usize>() * 3] {
    let mut buf = [0u8; size_of::<usize>() * 3];
    buf.copy_from_slice([space, sp, ip].map(usize::to_ne_bytes).as_flattened());
    buf
}
pub fn create_set_addr_space_buf_for_fork(
    space: usize,
    ip: usize,
    sp: usize,
    arg1: usize,
) -> [u8; size_of::<usize>() * 4] {
    let mut buf = [0u8; size_of::<usize>() * 4];
    buf.copy_from_slice([space, sp, ip, arg1].map(usize::to_ne_bytes).as_flattened());
    buf
}

/// Spawns a new context which will not share the same address space as the current one. File
/// descriptors from other schemes are reobtained with `dup`, and grants referencing such file
/// descriptors are reobtained through `fmap`. Other mappings are kept but duplicated using CoW.
pub fn fork_impl(args: &ForkArgs<'_>) -> Result<usize> {
    let old_mask = crate::signal::get_sigmask()?;
    let pid = unsafe {
        Error::demux(__relibc_internal_fork_wrapper(
            core::ptr::from_ref::<ForkArgs>(args) as usize,
        ))?
    };

    if pid == 0 {
        crate::signal::set_sigmask(Some(old_mask), None)?;
    }
    Ok(pid)
}

pub enum ForkArgs<'a> {
    Init {
        this_thr_fd: &'a FdGuardUpper,
        auth: &'a FdGuard,
        name: &'a str,
    },
    Managed,
}

pub fn fork_inner(initial_rsp: *mut usize, args: &ForkArgs) -> Result<usize> {
    let (cur_filetable_fd, new_filetable_fd, new_proc_fd, new_thr_fd, new_pid);

    {
        let cur_thr_fd = match args {
            ForkArgs::Init { this_thr_fd, .. } => this_thr_fd,
            ForkArgs::Managed => RtTcb::current().thread_fd(),
        };
        let NewChildProc {
            proc_fd,
            thr_fd,
            pid,
        } = new_child_process(args)?;
        new_proc_fd = proc_fd;
        new_thr_fd = thr_fd;
        new_pid = pid;

        // Copy existing files into new file table, but do not reuse the same file table (i.e. new
        // parent FDs will not show up for the child).
        let scratchpad = {
            cur_filetable_fd = cur_thr_fd.dup_into_upper(b"filetable-binary")?;
            new_filetable_fd = cur_filetable_fd.dup_into_upper(b"copy")?;

            // This must be done before the address space is copied.
            let proc_fd = new_proc_fd.as_ref().map_or(usize::MAX, |p| p.as_raw_fd());
            //let _ = syscall::write(1, alloc::format!("FDTBL{}PROC{}THR{}\n", *cur_filetable_fd, proc_fd, *new_thr_fd).as_bytes());

            ForkScratchpad {
                cur_filetable_fd: cur_filetable_fd.as_raw_fd(),
                new_filetable_fd: new_filetable_fd.as_raw_fd(),
                new_proc_fd: proc_fd,
                new_thr_fd: new_thr_fd.as_raw_fd(),
            }
        };
        #[cfg(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        ))]
        let arg1 = {
            let scratchpad_ptr: *const ForkScratchpad = &raw const scratchpad;
            scratchpad_ptr as usize
        };
        #[cfg(target_arch = "x86")]
        unsafe {
            let scratchpad_ptr = initial_rsp.cast::<ForkScratchpad>();
            scratchpad_ptr.write(scratchpad);
        }

        // CoW-duplicate address space.
        {
            let new_addr_space_sel_fd = new_thr_fd.dup_into_upper(b"current-addrspace")?;

            let cur_addr_space_fd = cur_thr_fd.dup_into_upper(b"addrspace")?;
            let new_addr_space_fd = cur_addr_space_fd.dup_into_upper(b"exclusive")?;

            let mut grant_desc_buf = [GrantDesc::default(); 16];
            loop {
                let bytes_read = {
                    let buf = unsafe {
                        core::slice::from_raw_parts_mut(
                            grant_desc_buf.as_mut_ptr().cast(),
                            grant_desc_buf.len() * size_of::<GrantDesc>(),
                        )
                    };
                    cur_addr_space_fd.read(buf)?
                };
                if bytes_read == 0 {
                    break;
                }

                let grants = &grant_desc_buf[..bytes_read / size_of::<GrantDesc>()];

                for grant in grants {
                    if !grant.flags.contains(GrantFlags::GRANT_SCHEME)
                        || !grant.flags.contains(GrantFlags::GRANT_SHARED)
                    {
                        continue;
                    }

                    let buf;

                    // TODO: write! using some #![no_std] Cursor type (tracking the length)?
                    #[cfg(target_pointer_width = "64")]
                    {
                        //buf = *b"grant-fd-AAAABBBBCCCCDDDD";
                        //write!(&mut buf, "grant-fd-{:>016x}", grant.base).unwrap();
                        buf = alloc::format!("grant-fd-{:>016x}", grant.base).into_bytes();
                    }

                    #[cfg(target_pointer_width = "32")]
                    {
                        //buf = *b"grant-fd-AAAABBBB";
                        //write!(&mut buf[..], "grant-fd-{:>08x}", grant.base).unwrap();
                        buf = alloc::format!("grant-fd-{:>08x}", grant.base).into_bytes();
                    }

                    let grant_fd = cur_addr_space_fd.dup_into_upper(&buf)?;

                    let mut flags = MAP_SHARED | MAP_FIXED_NOREPLACE;

                    flags.set(PROT_READ, grant.flags.contains(GrantFlags::GRANT_READ));
                    flags.set(PROT_WRITE, grant.flags.contains(GrantFlags::GRANT_WRITE));
                    flags.set(PROT_EXEC, grant.flags.contains(GrantFlags::GRANT_EXEC));

                    mmap_remote(
                        &new_addr_space_fd,
                        &grant_fd,
                        grant.offset as usize,
                        grant.base,
                        grant.size,
                        flags,
                    )?;
                }
            }
            #[cfg(any(
                target_arch = "x86_64",
                target_arch = "aarch64",
                target_arch = "riscv64"
            ))]
            let buf = create_set_addr_space_buf_for_fork(
                new_addr_space_fd.as_raw_fd(),
                __relibc_internal_fork_ret as *const () as usize,
                initial_rsp as usize,
                arg1,
            );
            #[cfg(target_arch = "x86")]
            let buf = create_set_addr_space_buf(
                new_addr_space_fd.as_raw_fd(),
                __relibc_internal_fork_ret as *const () as usize,
                initial_rsp as usize,
            );
            new_addr_space_sel_fd.write(&buf)?;
        }
        {
            // Reuse the same sigaltstack and signal entry (all memory will be re-mapped CoW later).
            //
            // Do this after the address space is cloned, since the kernel will get a shared
            // reference to the TCB and whatever pages stores the signal proc control struct.
            {
                let new_sighandler_fd = new_thr_fd.dup_into_upper(b"sighandler")?;
                new_sighandler_fd.write(&crate::signal::current_setsighandler_struct())?;
            }
            if let Some(ref proc_fd) = new_proc_fd {
                sys_call(
                    proc_fd.as_raw_fd(),
                    CallFlags::empty(),
                    &[ProcCall::SyncSigPctl as u64],
                )?;
                sys_call(
                    new_thr_fd.as_raw_fd(),
                    CallFlags::empty(),
                    &[ThreadCall::SyncSigTctl as u64],
                )?;
            }
        }
        {
            // Copy environment registers.
            let cur_env_regs_fd = cur_thr_fd.dup_into_upper(b"regs/env")?;
            let new_env_regs_fd = new_thr_fd.dup_into_upper(b"regs/env")?;

            let mut env_regs = syscall::EnvRegisters::default();
            cur_env_regs_fd.read(&mut env_regs)?;
            new_env_regs_fd.write(&env_regs)?;
        }
    }
    // TODO: This shouldn't be opt-in, make sure in the kernel that userspace can't create cyclic
    // references to leak fds.
    let (new_filetable_weak_fd, new_filetable_fd_idx) = {
        // TODO: Use file descriptor forwarding or something similar to avoid copying the file
        // table in the kernel.
        let new_filetable_sel_fd = new_thr_fd.dup_into_upper(b"current-filetable")?;
        new_filetable_sel_fd.write(&usize::to_ne_bytes(new_filetable_fd.as_raw_fd()))?;
        // downgrade to a kernel handle with only a Weak reference to the FdTbl
        let weak = new_filetable_fd.dup_into_upper(b"refresh")?;
        let new_filetable_fd_idx = new_filetable_fd.as_raw_fd();

        drop(new_filetable_fd);
        (weak, new_filetable_fd_idx)
    };

    new_filetable_weak_fd.call_wo(
        &new_filetable_weak_fd.as_raw_fd().to_ne_bytes(),
        syscall::CallFlags::FD,
        &[new_filetable_fd_idx as u64],
    )?;

    let start_fd = new_thr_fd.dup_into_upper(b"start")?;
    start_fd.write(&[0])?;
    Ok(new_pid)
}

pub struct NewChildProc {
    pub proc_fd: Option<FdGuardUpper>,

    pub thr_fd: FdGuardUpper,
    pub pid: usize,
}

pub fn new_child_process(args: &ForkArgs<'_>) -> Result<NewChildProc> {
    match *args {
        ForkArgs::Managed => {
            let proc_info = crate::static_proc_info();
            let this_proc_fd = proc_info
                .proc_fd
                .as_ref()
                .expect("cannot use ForkArgs::Managed without an existing proc info");
            let child_proc_fd = this_proc_fd.dup_into_upper(b"fork")?;
            let only_thread_fd = child_proc_fd.dup_into_upper(b"thread-0")?;
            let meta = read_proc_meta(&child_proc_fd)?;
            Ok(NewChildProc {
                proc_fd: Some(child_proc_fd),
                thr_fd: only_thread_fd,
                pid: meta.pid as usize,
            })
        }
        #[cfg(feature = "proc")]
        ForkArgs::Init { .. } => unreachable!(),

        #[cfg(not(feature = "proc"))]
        ForkArgs::Init {
            this_thr_fd: _,
            auth,
            name,
        } => {
            let thr_fd = auth.dup_into_upper(b"new-context")?;
            let buf = syscall::ProcSchemeAttrs {
                pid: 0,
                euid: 0,
                egid: 0,
                prio: !0, // Value is overwritten later
                debug_name: {
                    let mut buf = [0; 32];
                    let len = name.len().min(buf.len());
                    buf[..len].copy_from_slice(&name.as_bytes()[..len]);
                    buf
                },
            };
            let attr_fd = thr_fd
                .dup_into_upper(alloc::format!("auth-{}-attrs", auth.as_raw_fd()).as_bytes())?;
            attr_fd.write(&buf)?;
            Ok(NewChildProc {
                thr_fd,
                pid: 1, // dummy fd to distinguish child from parent
                proc_fd: None,
            })
        }
    }
}

pub unsafe fn make_init(proc_cap: usize) -> (&'static FdGuardUpper, &'static FdGuardUpper) {
    let proc_fd = FdGuard::new(
        crate::sys::openat_into_upper(proc_cap, "init", 0, 0).expect("failed to create init"),
    )
    .to_upper()
    .unwrap();

    crate::sys::sys_call_wo(
        proc_fd.as_raw_fd(),
        &RtTcb::current()
            .thread_fd()
            .dup_into_upper(&[])
            .unwrap()
            .take()
            .to_ne_bytes(),
        syscall::CallFlags::FD,
        &[],
    )
    .expect("failed to assign current thread to init process");

    let managed_thr_fd = proc_fd
        .dup_into_upper(b"thread-0")
        .expect("failed to get managed thread for init");

    let managed_thr_fd = unsafe { (*RtTcb::current().thr_fd.get()).insert(managed_thr_fd) };

    unsafe {
        STATIC_PROC_INFO.get().write(crate::StaticProcInfo {
            pid: 1,
            proc_fd: Some(proc_fd),
        })
    };
    *DYNAMIC_PROC_INFO.lock() = crate::DynamicProcInfo {
        pgid: 1,
        ruid: 0,
        euid: 0,
        suid: 0,
        rgid: 0,
        egid: 0,
        sgid: 0,
        ns_fd: None,
    };
    (
        unsafe { (*STATIC_PROC_INFO.get()).proc_fd.as_ref().unwrap() },
        managed_thr_fd,
    )
}
pub(crate) static STATIC_PROC_INFO: SyncUnsafeCell<StaticProcInfo> =
    SyncUnsafeCell::new(StaticProcInfo {
        pid: 0,
        proc_fd: None,
    });
