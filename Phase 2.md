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
**Status: foundational implementation expanded; not complete.** Existing code now includes PE/Mach-O inspection, inert PE mapping, PE DIR64 relocation application, a conservative x86-64 decoder subset, bounded guest address-space/process data models, Windows import/export and API classification, macOS bundle planning, and a decoder fuzz target. Native execution, full decoding, complete dynamic linking, and platform service integration remain missing. See [Stage G](Phase%202%20Stage%20G.md).

### Stage H — Isolated guest process and execution backend
**Status: foundational model implemented; OS integration incomplete.** Safe address-space and process/CPU-state models include checked mappings, quotas and W^X policy. Redox scheduler/page-table integration, native context switching, syscall/fault handling and containment remain. See [Stage H](Phase%202%20Stage%20H.md).

### Stage I — Windows compatibility runtime
**Status: compatibility boundary implemented; runtime services incomplete.** Typed import/export resolution model and explicit API classification exist; no Windows API is marked operational. PE IAT integration, process startup, ABI and kernel-backed Win32/NT services remain. See [Stage I](Phase%202%20Stage%20I.md).

### Stage J — macOS compatibility runtime and unified dispatch
**Status: bundle-planning foundation implemented; runtime and dispatch incomplete.** Broker-fed member path validation, bounded XML bundle planning and explicit unresolved-framework diagnostics exist. Binary plist, Mach-O dyld linking, process startup, framework services and unified launcher remain. See [Stage J](Phase%202%20Stage%20J.md).

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
