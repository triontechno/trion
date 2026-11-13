# TRION OS — Phase 2: Adaptive Binary/Ternary Computing

**Status:** Active implementation  
**Parent:** [TRION-ARCHITECTURE.md](../architecture/TRION-ARCHITECTURE.md)  
**Prerequisite:** Phase 1 ternary core  
**Rule:** Work directly on main where practical. Do not run tests, builds, benchmarks, fuzzing, or QEMU validation until all Phase 2 implementation stages are complete.

## 1. Objective

Develop an adaptive binary/ternary architecture for TRION Version 1 while preserving conventional hardware and native ABI boundaries. Windows, macOS and Linux application compatibility are deferred to Version 2. Commodity processors are not assumed to execute ternary instructions or contain ternary RAM.

## 2. Stage plan

### Stage A — Adaptive representation foundation
**Status: implementation merged to main.** Binary, balanced-ternary, and hybrid choices; evidence-based deterministic selection; checked i128/u128 bridges. Validation is deferred.

### Stage B — Packed ternary logical memory
**Status: implementation delivered; validation deferred.** Five balanced trits per byte with checked construction, access, validation, and round-trip coverage. Backing storage remains conventional binary RAM.

### Stage C — TIR execution foundation
**Status: implementation delivered; validation deferred.** Bounded interpreter foundation. TIR remains a standalone research/test runtime, not an application execution path.

### Stage D — Translator (retired)
**Status: discontinued by architecture decision.** The former translator, TIR conversion workflow, and TRIONEX application path are retired. Stage G onward is the sole application execution architecture.

### Stage E — Runtime, kernel, and hardware integration
**Status: implementation delivered; validation deferred.** Includes target capability reporting, Redox read-only capability scheme, adaptive runtime policy evidence, PCI/ACPI inventory, binary hardware boundaries, initfs packaging, service ordering, and endpoint integration checker.

### Stage F — Measurement, hardening, and release infrastructure
**Status: implementation delivered; execution deferred.** Includes comparative benchmark harness, bounded fuzz targets, manual validation workflow, and manual release/build/QEMU workflow. No measurements or validation have been run.

### Stage G — Executable format and loader completion
**Status: implementation complete for the declared freestanding x86-64 execution profiles; validation deferred.** Includes direct PE32+ and Mach-O64 freestanding image loading into Redox process address spaces, W^X segment mapping, initial user context setup, process-manager exit handling, and bounded COSMIC Files dispatch. These narrow entry profiles are not Windows or macOS application compatibility and do not imply general platform API support. See [Stage G](Phase%202%20Stage%20G.md).

### Stage H — Isolated guest process and execution backend
**Status: implementation complete for the restricted guest profile; validation deferred.** Includes kernel-enforced per-address-space syscall isolation, authenticated exit, memory/descriptor/CPU quotas, supervisor-selected pre-opened filesystem/network/device/IPC stream capabilities with read/write enforcement, descriptor revocation, guest cancellation, structured guest crash reports, sibling termination and atomic descriptor cleanup during address-space activation. Arbitrary scheme calls, path creation, descriptor duplication and guest-created processes/threads remain denied. The guest backend is a restricted native execution security boundary, not an application compatibility layer. See [Stage H](Phase%202%20Stage%20H.md).

### Stage I — Native application runtime
**Status: implementation complete for the declared native profile; validation deferred.** Completes the TRION/Redox native ELF process and runtime contract, including bounded image loading, interpreter handoff, startup state and auxv, PIE-aware TLS initialization, in-place exec transition handling, native thread/signal runtime integration, and the documented native service ABI. The native ELF ABI is not Linux compatibility. See [Stage I](Phase%202%20Stage%20I.md) and the [native application profile](../architecture/native-application-profile.md).

### Stage J — Native system integration and completion
**Status: not yet implemented; validation deferred.** Integrates native executable dispatch, initfs and boot services, service dependency ordering, kernel/runtime ABI, device boundaries, security policy, and resource lifecycle into a coherent Version 1 system. See [Stage J](Phase%202%20Stage%20J.md).

### Version 2 compatibility (deferred)
Windows, macOS and Linux application compatibility are explicitly outside Version 1 and outside Phase 2 Stages I/J. They require a separately scoped Version 2 workstream. The repository's native ELF/Redox process path is not Linux ABI compatibility.
undefined

## 3. Global engineering gates

1. Correctness and safety outrank representation choice.
2. Never infer unsupported hardware capabilities.
3. Conversions are explicit, checked, and semantics-preserving.
4. Malformed external input must not cause unchecked panics or out-of-bounds access in new components.
5. Ternary candidates require support and semantic validation.
6. Conventional hardware, network, file, and ABI boundaries retain their specified binary layouts.
7. Keep new crates isolated until APIs and implementation are stable.
8. Target main; merge any temporary branch after implementation.
9. Original executable files remain unchanged. No application translator, TIR conversion, .trx container, translation cache, or generated intermediate executable.
10. Parsing or mapping executable bytes does not authorize execution. Unsupported features fail closed.

## 4. Phase 2 exit criteria

Phase 2 implementation is complete only when Stages A–C and E–J are implemented to their stated scope; Stage D remains intentionally retired. In particular, Stages G–J must provide the declared executable loader, isolated process executor, native runtime contract, unified native dispatch, boot/service integration, and security controls.

Only after all implementation stages are complete may the consolidated validation pass run: automated tests, fuzz campaigns, comparative benchmarks, native executable capability checks, supported OS builds, and QEMU boot validation. Until then, all such execution remains deferred.

The final project phase is **Phase 6** (formerly named Phase H); it is separate from Phase 2. The repository currently has no confirmed Phase 3–5 specifications, so their scope must not be invented or inferred from this Phase 2 plan.
