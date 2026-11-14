# TRION Version 1 Native System Integration

**Implementation status: integration contract recorded; validation deferred.**

This document records the production path that joins the native executable runtime, boot image, service manager, kernel interfaces, device schemes, security policy and process lifecycle. It is the Stage J integration reference. It does not introduce a second executable loader or a foreign operating-system compatibility layer.

## 1. Executable dispatch

The authoritative process-image entry is `relibc::proc::fexec_impl` in `src/relibc/redox-rt/src/proc.rs`.

Dispatch is performed from the first four bytes of the original executable:

| Image signature | Dispatch | Scope |
|---|---|---|
| ELF magic | Existing native ELF loader in `fexec_impl` | TRION/Redox native ABI |
| MZ | `proc::trion::fexec_pe32_plus` on x86-64 | Restricted freestanding PE32+ only |
| Little-endian Mach-O64 magic | `proc::trion::fexec_macho64` on x86-64 | Restricted freestanding Mach-O64 only |
| Anything else | `ENOEXEC` | Rejected before format-specific parsing |

The dispatcher does not translate, rewrite, extract, or route applications through TIR. It selects one existing native loader and preserves the original backing file. PE32+ and Mach-O64 are narrow Stage G entry profiles and do not provide Win32/NT or Darwin/dyld services. On non-x86-64 targets, only the native ELF path is selected.

## 2. Boot image and service graph

The initfs is built by `src/base/Makefile` from `INITFS_BINS`, `INITFS_DRIVERS_BINS`, and `src/base/init.initfs.d`. Stage E's `trion-capd` and `trion-cap-check` are included in the initfs binary list.

The existing early-boot ordering is:

1. The init service manager starts the early input, graphics and storage services.
2. `40_drivers.target` waits weakly for the hardware manager, initfs PCI spawner, TRION capability daemon and its one-shot checker.
3. `50_rootfs.service` follows `40_drivers.target` through a weak dependency and mounts the system root.
4. Native user applications and their runtime libraries are supplied by the mounted system root, not by pretending the early initfs is a complete application root.

Weak dependencies are intentional for optional hardware and inventory services: absence or failure of optional hardware discovery must not make the root filesystem unmountable. The service manager remains responsible for service state and dependency handling. No new restart guarantee is claimed by this integration contract.

## 3. Native runtime contract

The native application contract is defined in `mds/architecture/native-application-profile.md` and implemented by relibc/redox-rt:

- ELF image validation, mapping, interpreter handoff and bounded initial process state.
- Native process/thread descriptors and address-space operations.
- relibc pthread/TLS startup and redox-rt signal entry/return.
- Scheme-backed filesystem and namespace operations.
- Native process wait/exit, descriptor and IPC interfaces.
- Native time and mediated device descriptors.

The kernel syscall ABI remains the Redox ABI. This stage does not renumber syscalls, reinterpret Linux syscall numbers, or add foreign ABI translation.

## 4. Hardware and device boundary

Native services access hardware through the existing Redox device and scheme interfaces. The TRION capability service reports discovered platform facts and adaptive runtime policy; it is not a hardware abstraction replacement and does not grant applications direct MMIO or DMA authority.

The following remain binary at hardware/protocol boundaries: MMIO register layouts, DMA descriptors, driver ABIs, syscall structures, network frames and persistent formats. Packed ternary memory remains a logical representation backed by ordinary binary memory. No physical ternary RAM, ternary CPU execution, or ternary peripheral is assumed.

## 5. Security composition

- Ordinary native ELF processes use the existing native Redox namespace and descriptor authority.
- The restricted Stage H guest policy is activated only by a trusted supervisor's explicit guest launch path.
- Guest syscall filtering, delegated descriptor rights, quota checks, cancellation and crash reporting remain kernel-enforced.
- Guest policy is not silently applied to ordinary native ELF applications.
- PE32+/Mach-O64 freestanding launch paths retain the restricted guest boundary and do not gain general platform APIs.
- Unsupported executable formats and unsupported runtime features fail closed.

## 6. Resource lifecycle

Native process and thread lifetime remains owned by Redox process/thread descriptors, scheduler state, wait/exit reporting, address-space grants and scheme-owned descriptors. The native exec path stages image state before its final address-space transition. Stage H guest quotas and revocation remain scoped to the guest profile; they are not represented as universal quotas for ordinary native processes.

A failed native image preparation returns an error before the new image becomes the active process image. Kernel teardown and scheme descriptor ownership remain the authoritative cleanup mechanisms for process termination.

## 7. Explicit non-goals

- Windows, macOS or Linux application compatibility.
- Win32/NT, Darwin, dyld or Linux syscall emulation.
- A TIR or translator fallback for application execution.
- Direct application MMIO/DMA access.
- Claims of physical ternary hardware.
- A new generic service-restart policy or process-wide resource quota not already provided by the underlying system.

## 8. Validation gate

Implementation work is recorded separately from validation. Do not run tests, builds, benchmarks, fuzzing or QEMU during the Phase 2 implementation sequence. After Stage J implementation is frozen, the consolidated validation pass must verify:

- Native ELF, interpreter and exec paths on supported architectures.
- Explicit rejection of unknown formats and unsupported PE/Mach-O features.
- Initfs contents and boot/service dependency ordering.
- Capability daemon endpoint availability when hardware inventory schemes exist and conservative failure behavior when they do not.
- Native process/thread/TLS/signal, descriptor, namespace and address-space lifecycle.
- Stage H syscall isolation, capability revocation, quota enforcement, cancellation and crash reporting.
- Full supported system-image build and QEMU boot.

No validation result is implied by this document.
