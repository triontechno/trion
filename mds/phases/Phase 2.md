# TRION OS — Phase 2: Adaptive Binary/Ternary Computing

**Status:** Active implementation  
**Parent:** [TRION-ARCHITECTURE.md](../architecture/TRION-ARCHITECTURE.md)  
**Prerequisite:** Phase 1 ternary core  
**Rule:** Work directly on main where practical. Do not run tests, builds, benchmarks, fuzzing, or QEMU validation until all Phase 2 implementation stages are complete.

## 1. Objective

Develop an adaptive binary/ternary architecture that preserves conventional hardware and software compatibility. Commodity processors are not assumed to execute ternary instructions or contain ternary RAM.

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
**Status: implementation complete for the declared freestanding x86-64 profile; validation deferred.** Includes direct native PE32+ and Mach-O64 loading into real Redox process address spaces, W^X segment mapping, initial user context setup through the process scheme, process-manager exit trampoline, complete semantics for the declared bounded interpreter subset, and COSMIC Files .exe/.app dispatch. Unsupported imports, dyld, platform APIs, environment/argument blocks and other runtime features fail closed. General Windows and macOS application compatibility remains in Stages I and J. See [Stage G](Phase%202%20Stage%20G.md).

### Stage H — Isolated guest process and execution backend
**Status: restricted syscall isolation, authenticated guest exit, mapped-memory/descriptor/CPU quotas and process-manager fault diagnostics implemented; full isolation incomplete.** The loader uses Redox address-space selection, user register setup, scheduler/context switching and process-manager exit. Filesystem/network/device/IPC capabilities and brokered I/O, guest-specific crash reports, complete thread/descendant policy and full cleanup auditing remain. See [Stage H](Phase%202%20Stage%20H.md).

### Stage I — Windows compatibility runtime
**Status: restricted freestanding PE32+ launch, DIR64 rebasing, minimal PEB/TEB and `ExitProcess`/process-identity imports exist; the selected profile remains incomplete.** Broader import/export resolution and API classification exist; full process parameters, loader module lists, TLS, unwind handling and kernel-backed Win32/NT services remain. See [Stage I](Phase%202%20Stage%20I.md).

### Stage J — macOS compatibility runtime and unified dispatch
**Status: restricted freestanding Mach-O launch and XML .app routing implemented; general macOS runtime incomplete.** Broker-fed member validation, XML bundle planning, native segment mapping and minimal `argc`/`argv[0]` startup stack exist. Binary plist, dyld linking, full Darwin startup, frameworks, code-signature policy and macOS services remain. See [Stage J](Phase%202%20Stage%20J.md).

### Linux compatibility — not currently specified or implemented
The Phase 2 plan defines PE32+ Windows and Mach-O macOS profiles, but no Linux compatibility profile, Linux distribution/version target, or Linux syscall ABI layer. The repository's native ELF/Redox process path is not Linux ABI compatibility. Linux compatibility is therefore outside the current Phase 2 stages and must not be attributed to Stages G, I, or J. Any future Linux support requires a separately scoped stage/workstream defining its ELF and architecture profile, Linux process ABI, syscall/service surface, runtime/dependency policy, and compatibility matrix before implementation.

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

Phase 2 implementation is complete only when Stages A–C and E–J are implemented to their stated scope; Stage D remains intentionally retired. In particular, Stage G–J must provide the declared loader, decoder, isolated process executor, dynamic linking, Windows/macOS compatibility profiles, launcher integration, and security controls.

Only after all implementation stages are complete may the consolidated validation pass run: automated tests, fuzz campaigns, comparative benchmarks, direct executable capability checks, compatibility-path tests, supported OS builds, and QEMU boot validation. Until then, all such execution remains deferred.

The final project phase is **Phase 6** (formerly named Phase H); it is separate from Phase 2. The repository currently has no confirmed Phase 3–5 specifications, so their scope must not be invented or inferred from this Phase 2 plan.
