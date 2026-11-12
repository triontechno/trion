# TRION Phase 2 — Stage I: Native Application Runtime

**Status: implementation in progress; validation deferred.** Stage I builds the native TRION/Redox application runtime. It does not implement Windows, macOS, or Linux application compatibility.

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

## Implemented in this pass

- ELF class, byte order, version, machine and executable-type validation.
- Exact program-header entry-size and nonzero-count validation with checked table-size arithmetic.
- PT_LOAD memory/file-size, alignment, page-congruence and address-range validation.
- Rejection of overlapping mapped PT_LOAD page ranges rather than allowing MAP_FIXED to overwrite a prior segment.
- Checked page alignment, program-header slicing, mapped-address arithmetic and argument/environment allocation.
- Bounded startup-string copying and stack growth; malformed or oversized startup state fails explicitly.

## Remaining implementation

- Full PT_INTERP validation and robust interpreter handoff.
- Complete auxv/argv/envp/cwd construction and validation across all supported native launch paths.
- Transactional exec cleanup and close-on-exec semantics.
- Native TLS, signal, thread startup and runtime service contract audit.
- End-to-end integration with Stage H quotas/capabilities and native service managers.
- Final supported-profile documentation after implementation is complete.

## Explicit exclusions

- Windows PE application compatibility.
- macOS Mach-O application compatibility.
- Linux ELF/syscall ABI compatibility.
- Wine-like API layers, platform DLL/framework catalogs, or application translation.

No tests, builds, benchmarks, fuzzing, or QEMU validation are run during implementation. Those remain deferred until Stage J is implemented.
