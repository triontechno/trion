# TRION Phase 2 — Stage J: Native System Integration and Completion

**Status: implementation partially delivered; Stage J remains open. Validation deferred.** The production executable dispatcher now classifies ELF, PE32+ and Mach-O64 before selecting a loader and rejects unknown formats before format-specific parsing. The existing boot graph, native Redox runtime, Stage E capability service, and Stage H guest boundary are documented as the integration baseline in [Native System Integration](../architecture/native-system-integration.md).

## Implemented in this Stage

1. **Explicit executable dispatch.** `src/relibc/redox-rt/src/proc.rs::fexec_impl` now performs one signature classification before entering a loader:
   - ELF selects the native TRION/Redox loader.
   - MZ selects the restricted x86-64 freestanding PE32+ loader.
   - Little-endian Mach-O64 selects the restricted x86-64 freestanding Mach-O loader.
   - Unknown signatures return `ENOEXEC` before format-specific parsing.
   - Non-x86-64 targets accept only the native ELF signature.
2. **Integration contract.** Added [Native System Integration](../architecture/native-system-integration.md), documenting the actual executable path, initfs/service graph, native ABI, device boundaries, security composition, resource ownership, exclusions and deferred validation.
3. **Existing boot integration recorded.** `src/base/Makefile` already packages the Stage E capability daemon/checker into initfs; `40_drivers.target` orders them after hardware discovery; `50_rootfs.service` follows the drivers target. Optional hardware services remain weak dependencies so their absence cannot prevent rootfs startup.
4. **Existing native runtime/security integration recorded.** Stage I owns native ELF process startup and relibc runtime integration. Stage H owns the restricted guest syscall gate, delegated descriptor policy, quotas, cancellation and crash reporting. The ordinary native ELF path does not silently inherit guest restrictions.

## Remaining implementation gaps

Stage J is **not complete**. The following requirements still need source-level implementation or an authoritative source audit before this stage can be marked complete:

- A bounded, explicit service failure/restart policy has not been implemented or verified in the init service manager.
- The full kernel/runtime ABI audit has not been completed across every supported architecture and syscall family.
- Ordinary native-process resource accounting/quotas are not established by Stage H's guest-only limits; no system-wide quota is claimed.
- A complete teardown/recovery audit across native processes, threads, schemes and services remains outstanding.
- The integration contract records the existing boot graph; it does not prove the generated image contains every native runtime library or that the full boot path works.

These are implementation gaps, not validation tasks. Do not mark Stage J complete merely because the validation pass is deferred.

## Scope and exclusions

Stage J integrates the Version 1 native runtime only. It does not add Windows, macOS or Linux application compatibility, Wine-like layers, platform API emulation, a TIR application fallback, or physical ternary hardware assumptions.

## Validation gate

No tests, builds, benchmarks, fuzzing or QEMU runs are to be performed until all Phase 2 implementation stages are complete. After the remaining Stage J implementation gaps are closed, run the consolidated validation pass defined in [Phase 2](Phase%202.md).
