# TRION Version 1 Native System Integration

**Implementation status: source integration delivered; validation deferred.**

This document records the production path joining the native executable runtime, boot image, service manager, kernel interfaces, device schemes, security policy and process lifecycle. It does not introduce a second executable loader or a foreign operating-system compatibility layer.

## 1. Executable dispatch

The authoritative process-image entry is relibc::proc::fexec_impl in src/relibc/redox-rt/src/proc.rs.

Dispatch is performed from the first four bytes of the original executable:

| Image signature | Dispatch | Scope |
|---|---|---|
| ELF magic | Existing native ELF loader in fexec_impl | TRION/Redox native ABI |
| MZ | proc::trion::fexec_pe32_plus on x86-64 | Restricted freestanding PE32+ only |
| Little-endian Mach-O64 magic | proc::trion::fexec_macho64 on x86-64 | Restricted freestanding Mach-O64 only |
| Anything else | ENOEXEC | Rejected before format-specific parsing |

The dispatcher does not translate, rewrite, extract, or route applications through TIR. PE32+ and Mach-O64 do not provide Win32/NT or Darwin/dyld services. Non-x86-64 targets select only native ELF.

## 2. Boot image and service graph

The initfs is built by src/base/Makefile from INITFS_BINS, INITFS_DRIVERS_BINS, and src/base/init.initfs.d. Stage E's trion-capd and trion-cap-check are included in the initfs binary list and copied into the image.

The early-boot ordering is:

1. The init service manager starts early runtime, input, graphics and storage services.
2. 40_drivers.target waits weakly for the hardware manager, initfs PCI spawner, TRION capability daemon and its one-shot checker.
3. 50_rootfs.service follows 40_drivers.target through a weak dependency and mounts the system root.
4. Native user applications and their runtime libraries are supplied by the mounted system root, not by treating initfs as a complete application root.

Weak dependencies are intentional for optional hardware and inventory services: missing optional hardware discovery must not prevent root filesystem startup.

## 3. Service lifecycle and restart policy

Service units may opt into bounded supervision with restart_on_failure, max_restarts and restart_delay_ms. The init manager caps restart attempts at 10 and delay at 30 seconds. The policy covers failures before readiness and unsuccessful exits after readiness.

Long-running opted-in services are tracked by PID in the scheduler. PID 1's central waitpid loop is the sole child reaper and forwards the PID/status to the scheduler. A failed service is restarted only while its finite budget remains; a successful exit is not restarted. The hardware manager and TRION capability daemon opt in with three retries. Other units retain no-restart defaults.

Readiness handling for notify and scheme services remains synchronous. Failed readiness, namespace lookup or scheme registration kills and reaps the child before a retry. Scheme registration errors are returned and logged instead of being hidden by an unconditional panic.

## 4. Native runtime and kernel ABI

The native application contract is defined in mds/architecture/native-application-profile.md and implemented by relibc/redox-rt. It covers:

- ELF validation, mapping, interpreter handoff and bounded initial process state.
- Native process/thread descriptors and address-space operations.
- relibc pthread/TLS startup and redox-rt signal entry/return.
- Scheme-backed filesystem and namespace operations.
- Native process wait/exit, descriptor and IPC interfaces.
- Native time and mediated device descriptors.

The kernel syscall ABI remains the Redox ABI. Stage J does not renumber syscalls, reinterpret Linux syscall numbers, or add foreign ABI translation. Supported native ELF architecture profiles are x86, x86-64, AArch64 and RISC-V64. PE32+/Mach-O64 remain restricted x86-64 freestanding launch profiles.

## 5. Hardware and device boundary

Native services access hardware through existing Redox device and scheme interfaces. The TRION capability service reports platform facts and adaptive runtime policy; it is not a hardware abstraction replacement and does not grant applications direct MMIO or DMA authority.

MMIO register layouts, DMA descriptors, driver ABIs, syscall structures, network frames and persistent formats remain binary. Packed ternary memory remains a logical representation backed by ordinary binary memory. No physical ternary RAM, ternary CPU execution or ternary peripheral is assumed.

## 6. Security and resource lifecycle

- Ordinary native ELF processes use existing native Redox namespace and descriptor authority.
- The restricted Stage H guest policy is activated only by a trusted supervisor's explicit guest launch path.
- Guest syscall filtering, delegated descriptor rights, memory/descriptor/CPU quotas, cancellation, crash reporting and revocation remain kernel-enforced.
- Guest policy is not silently applied to ordinary native ELF applications.
- Native process/thread lifetime, address-space ownership and scheme descriptors remain kernel-managed.
- PID 1 continuously reaps children, tolerates interrupted/transient wait errors, and forwards supervised failures to the bounded restart scheduler.
- Failed native image preparation returns before the new image becomes active.

Ordinary native processes continue under existing Redox kernel resource policy; this integration does not claim universal process quotas.

## 7. Explicit non-goals

- Windows, macOS or Linux application compatibility.
- Win32/NT, Darwin, dyld or Linux syscall emulation.
- A TIR or translator fallback for application execution.
- Direct application MMIO/DMA access.
- Physical ternary hardware assumptions.
- Unbounded service restart or a new universal native-process quota.

## 8. Validation gate

Do not run tests, builds, benchmarks, fuzzing or QEMU during the Phase 2 implementation sequence. After Stage J implementation is frozen, consolidated validation must verify executable dispatch, initfs packaging, service restart/reaping behavior, native process/thread/TLS/signal and address-space lifecycle, Stage H guest enforcement, and full supported system-image/QEMU boot. No validation result is implied here.
