# TRION OS — Phase 2: Adaptive Binary/Ternary Computing

**Status:** Active implementation  
**Parent:** [TRION-ARCHITECTURE.md](TRION-ARCHITECTURE.md)  
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
**Status: partial; not complete.** Existing code inspects PE32+ x86-64 and Mach-O 64 x86-64 metadata, validates selected ranges, maps PE bytes as inert memory, enumerates dependencies, and selects an app-bundle executable from bounded XML metadata. It does not apply relocations, resolve symbols, or execute code. Complete bounded PE/Mach-O loading, relocation handling, and instruction decoding without introducing translation.

### Stage H — Isolated guest process and execution backend
**Status: not implemented.** Implement guest address-space ownership, process/thread lifecycle, page mapping, W^X, CPU state, direct execution boundary, syscall/fault transitions, resource accounting, and containment. Guest code must never execute in kernel mode.

### Stage I — Windows compatibility runtime
**Status: not implemented.** Implement a declared, bounded Windows application compatibility profile: PE imports and IAT resolution, process startup, calling conventions, required runtime libraries, selected Win32/NT services, filesystem semantics, synchronization, and explicit unsupported-API diagnostics. No general Windows compatibility claim without evidence.

### Stage J — macOS compatibility runtime and unified dispatch
**Status: not implemented.** Implement a declared, bounded macOS profile: .app traversal, Info.plist handling, Mach-O relocations and dyld binding, dependency/framework resolution, process startup, selected system services, resource handling, launcher/file-manager dispatch, and actionable compatibility reporting. No general macOS compatibility claim without evidence.

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
