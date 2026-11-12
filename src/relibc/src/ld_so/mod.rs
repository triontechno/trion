//! Dynamic loading and linking.

// FIXME(andypython): remove this when #![allow(warnings, unused_variables)] is
// dropped from src/lib.rs.
#![warn(warnings, unused_variables)]

use core::{mem, ptr};
use object::{
    Endianness,
    elf::{self, ProgramHeader32, ProgramHeader64},
    read::elf::ProgramHeader,
};

use self::tcb::{Master, Tcb};
use crate::{
    header::sys_auxv::{AT_NULL, AT_PHDR, AT_PHENT, AT_PHNUM},
    platform::{Pal, Sys},
    start::Stack,
};

pub const PATH_SEP: char = ':';

mod access;
pub mod debug;
pub mod dso;
pub mod linker;
pub mod start;
pub mod tcb;

static mut STATIC_TCB_MASTER: Master = Master {
    ptr: ptr::null_mut(),
    image_size: 0,
    segment_size: 0,
    offset: 0,
};

#[inline(never)]
fn static_init(
    sp: &'static Stack,
    #[cfg(target_os = "redox")] thr_fd: redox_rt::proc::FdGuardUpper,
) -> Result<(), linker::DlError> {
    const SIZEOF_PHDR64: usize = mem::size_of::<ProgramHeader64<Endianness>>();
    const SIZEOF_PHDR32: usize = mem::size_of::<ProgramHeader32<Endianness>>();

    let mut phdr_opt = None;
    let mut phent_opt = None;
    let mut phnum_opt = None;

    let mut auxv = sp.auxv();
    loop {
        let (kind, value) = unsafe { *auxv };
        if kind == AT_NULL {
            break;
        }
        match kind {
            AT_PHDR => phdr_opt = Some(value),
            AT_PHENT => phent_opt = Some(value),
            AT_PHNUM => phnum_opt = Some(value),
            _ => (),
        }
        auxv = unsafe { auxv.add(1) };
    }

    let phdr = phdr_opt.ok_or(linker::DlError::Malformed)?;
    let phent = phent_opt.ok_or(linker::DlError::Malformed)?;
    let phnum = phnum_opt.ok_or(linker::DlError::Malformed)?;
    let expected_phent = if cfg!(target_pointer_width = "64") {
        SIZEOF_PHDR64
    } else {
        SIZEOF_PHDR32
    };
    if phdr == 0 || phent != expected_phent || phnum == 0 || phnum > 4096 {
        return Err(linker::DlError::Malformed);
    }

    let page_size = Sys::getpagesize();
    if page_size == 0 || !page_size.is_power_of_two() {
        return Err(linker::DlError::Malformed);
    }

    let mut tls_master = None;
    for i in 0..phnum {
        let ph_offset = phent.checked_mul(i).ok_or(linker::DlError::Malformed)?;
        let ph_addr = phdr.checked_add(ph_offset).ok_or(linker::DlError::Malformed)?;
        let (p_align, p_filesz, p_memsz, p_type, p_vaddr) = match phent {
            SIZEOF_PHDR64 => {
                if ph_addr % mem::align_of::<ProgramHeader64<Endianness>>() != 0 {
                    return Err(linker::DlError::Malformed);
                }
                let ph = unsafe { &*(ph_addr as *const ProgramHeader64<Endianness>) };
                (
                    usize::try_from(ph.p_align(Endianness::default()))
                        .map_err(|_| linker::DlError::Malformed)?,
                    usize::try_from(ph.p_filesz(Endianness::default()))
                        .map_err(|_| linker::DlError::Malformed)?,
                    usize::try_from(ph.p_memsz(Endianness::default()))
                        .map_err(|_| linker::DlError::Malformed)?,
                    ph.p_type(Endianness::default()),
                    usize::try_from(ph.p_vaddr(Endianness::default()))
                        .map_err(|_| linker::DlError::Malformed)?,
                )
            }
            SIZEOF_PHDR32 => {
                if ph_addr % mem::align_of::<ProgramHeader32<Endianness>>() != 0 {
                    return Err(linker::DlError::Malformed);
                }
                let ph = unsafe { &*(ph_addr as *const ProgramHeader32<Endianness>) };
                (
                    usize::try_from(ph.p_align(Endianness::default()))
                        .map_err(|_| linker::DlError::Malformed)?,
                    usize::try_from(ph.p_filesz(Endianness::default()))
                        .map_err(|_| linker::DlError::Malformed)?,
                    usize::try_from(ph.p_memsz(Endianness::default()))
                        .map_err(|_| linker::DlError::Malformed)?,
                    ph.p_type(Endianness::default()),
                    usize::try_from(ph.p_vaddr(Endianness::default()))
                        .map_err(|_| linker::DlError::Malformed)?,
                )
            }
            _ => return Err(linker::DlError::Malformed),
        };

        if p_type != elf::PT_TLS {
            continue;
        }
        if tls_master.is_some()
            || p_memsz == 0
            || p_filesz > p_memsz
            || p_align > 1 && !p_align.is_power_of_two()
        {
            return Err(linker::DlError::Malformed);
        }

        let page_offset = p_vaddr % page_size;
        let tls_span = p_memsz
            .checked_add(page_offset)
            .and_then(|size| size.checked_add(page_size - 1))
            .map(|size| size / page_size * page_size)
            .ok_or(linker::DlError::Malformed)?;
        let tls_alignment = p_align.max(1);
        let tls_offset = p_memsz
            .checked_add(tls_alignment - 1)
            .map(|size| size / tls_alignment * tls_alignment)
            .ok_or(linker::DlError::Malformed)?;
        let tls_size = tls_span.max(
            tls_offset
                .checked_add(page_size - 1)
                .map(|size| size / page_size * page_size)
                .ok_or(linker::DlError::Malformed)?,
        );
        tls_master = Some((p_vaddr, p_filesz, p_memsz, tls_offset, tls_size));
    }

    if let Some((p_vaddr, p_filesz, p_memsz, tls_offset, tls_size)) = tls_master {
        unsafe {
            STATIC_TCB_MASTER.ptr = p_vaddr as *const u8;
            STATIC_TCB_MASTER.image_size = p_filesz;
            STATIC_TCB_MASTER.segment_size = p_memsz;
            STATIC_TCB_MASTER.offset = tls_offset;

            let tcb = Tcb::new(tls_size)?;
            tcb.masters_ptr = ptr::addr_of_mut!(STATIC_TCB_MASTER);
            tcb.masters_len = mem::size_of::<Master>();
            tcb.copy_masters()?;
            tcb.activate(
                #[cfg(target_os = "redox")]
                Some(thr_fd),
            );
        }
    }

    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "redox"))]
pub unsafe fn init(
    sp: &'static Stack,
    #[cfg(target_os = "redox")] thr_fd: redox_rt::proc::FdGuardUpper,
) -> Result<(), linker::DlError> {
    let tp: usize;

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        const ARCH_GET_FS: usize = 0x1003;
        let mut val = 0usize;
        syscall!(ARCH_PRCTL, ARCH_GET_FS, &raw mut val);
        tp = val;
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!(
            "mrs {}, tpidr_el0",
            out(reg) tp,
        );
    }
    #[cfg(all(target_os = "redox", target_arch = "x86"))]
    {
        let mut env = syscall::EnvRegisters::default();
        let file = thr_fd
            .dup_into_upper(b"regs/env")
            .map_err(|_| linker::DlError::Malformed)?;
        file.read(&mut env)
            .map_err(|_| linker::DlError::Malformed)?;
        tp = env.gsbase as usize;
    }
    #[cfg(all(target_os = "redox", target_arch = "x86_64"))]
    {
        let mut env = syscall::EnvRegisters::default();
        let file = thr_fd
            .dup_into_upper(b"regs/env")
            .map_err(|_| linker::DlError::Malformed)?;
        file.read(&mut env)
            .map_err(|_| linker::DlError::Malformed)?;
        tp = env.fsbase as usize;
    }
    #[cfg(all(target_os = "redox", target_arch = "riscv64"))]
    unsafe {
        core::arch::asm!(
            "mv {}, tp",
            out(reg) tp,
        );
    }

    if tp == 0 {
        static_init(
            sp,
            #[cfg(target_os = "redox")]
            thr_fd,
        )?;
    } else {
        // The thread fd must already be present in the already existing TCB. Don't close it.
        #[cfg(target_os = "redox")]
        core::mem::forget(thr_fd);
    }

    Ok(())
}

pub unsafe fn fini() {
    if let Some(tcb) = unsafe { Tcb::current() }
        && !tcb.linker_ptr.is_null()
    {
        let linker = unsafe { (*tcb.linker_ptr).lock() };
        linker.fini();
    }
}
