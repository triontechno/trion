# TRION Phase 2 — Stage I: Native Application Runtime

**Status: implementation complete for the declared native profile; validation deferred.** Stage I builds the native TRION/Redox application runtime. It does not implement Windows, macOS, or Linux application compatibility.

## Scope

Stage I completes the native ELF process-execution contract used by Version 1 applications and the runtime services required by native applications. ELF here means the TRION/Redox native ABI; it does not imply Linux ABI compatibility.

## Implementation steps

1. **Harden native ELF image validation and mapping.** Validate ELF identity, class, endianness, machine, version, executable type, program-header dimensions, PT_LOAD bounds/alignment, file ranges, mapped-page overlap, and checked address arithmetic. Malformed images must fail with an error instead of panicking, wrapping, or replacing an earlier segment.
2. **Complete interpreter handoff.** Validate PT_INTERP metadata and its relationship to the loaded image; preserve the correct program-header address, entry point, base address, and mmap floor when transferring control to the native interpreter.
3. **Complete initial process state.** Build bounded argv, envp, cwd and auxiliary vectors; enforce stack bounds/alignment; provide deterministic startup metadata and fail cleanly on oversized inputs.
4. **Make exec transitions transactional.** Audit every failure point before address-space activation, descriptor close-on-exec handling, process naming, signal state, and filetable changes. A failed exec must not leave the current process partially modified.
5. **Complete native thread/runtime startup.** Define and implement the native ABI for thread entry, TLS, signal delivery/return, process exit, and runtime initialization using existing Redox interfaces.
6. **Integrate native resource and capability policy.** Ensure native process/thread creation and runtime services respect kernel-enforced memory, descriptor, scheduling and Stage H isolation boundaries.
7. **Close the native service contract.** Document the supported native syscall, filesystem, IPC, time, synchronization and device-facing interfaces; connect them to existing system services without introducing compatibility shims for other OS ABIs.
8. **Document the supported native application profile.** Specify executable architecture, ABI, dynamic interpreter expectations, startup contract, resource limits and explicit unsupported features.

## Implemented

- ELF identity, class, byte order, machine, version, executable type and header-size validation.
- Exact program-header entry-size and nonzero-count validation, with checked table-size arithmetic and backing-file bounds.
- PT_LOAD memory/file-size, alignment, page-congruence, file-range and address-range validation.
- Rejection of overlapping mapped PT_LOAD page ranges and W+X segments; the entry point must be inside an executable load segment.
- PT_INTERP uniqueness, ordering, size, NUL termination, absolute-path and backing-file checks. Nested interpreter requests are rejected.
- Interpreter handoff preserves the main executable entry point, program-header address/count/size, interpreter base and mmap floor.
- Checked page alignment, program-header slicing, mapped-address arithmetic and unrepresentable file offsets.
- Bounded argv/envp/cwd/execfn and startup payload construction; the combined startup string data is limited to 2 MiB and argv/envp to 65,536 entries.
- Correctly sized, 16-byte-aligned initial stack with native auxv entries for image metadata, page size, real/effective IDs, secure-execution state, executable path, optional 16-byte system-random seed, signal masks, umask, cwd and inherited descriptors.
- PIE-aware static TLS initialization using PT_PHDR/AT_PHDR load-bias resolution; PT_TLS size, alignment, template address and destination ranges are checked.
- TCB allocation and TLS master-list validation, including pointer/length consistency, copied-master bounds and TLS destination bounds.
- In-place exec stages image construction before its final transition, prepares required descriptors before TLS deactivation, applies process metadata changes at the end of the commit path, and propagates address-space and interpreter metadata write failures.
- Native thread creation, TCB activation, TLS master copying, signal trampoline/return, process startup and thread exit use the existing relibc/redox-rt interfaces. Their supported contract and exclusions are documented in [the native application profile](../architecture/native-application-profile.md).
- Native service boundaries are documented: scheme-backed filesystem/namespaces, process/thread descriptors, signals, address-space operations, IPC, time and mediated device descriptors.

## Explicitly deferred or unsupported

- Runtime validation (builds, tests, fuzzing, benchmarks and QEMU) remains deferred until Stage J implementation is complete, as instructed.
- Scheduler-policy APIs that are still unimplemented by the current native runtime are outside the declared Version 1 native application profile.
- AArch64 SIMD and RISC-V vector state in signal contexts are not claimed as supported by this profile.
- Windows PE, macOS Mach-O and Linux ABI compatibility remain Version 2 exclusions.

## Explicit exclusions

- Windows PE application compatibility.
- macOS Mach-O application compatibility.
- Linux ELF/syscall ABI compatibility.
- Wine-like API layers, platform DLL/framework catalogs, or application translation.

No tests, builds, benchmarks, fuzzing, or QEMU validation are run during implementation. Those remain deferred until Stage J is implemented.
