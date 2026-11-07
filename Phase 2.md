# TRION OS — Phase 2: Adaptive Binary/Ternary Computing

**Status:** Active implementation  
**Parent:** [TRION-ARCHITECTURE.md](TRION-ARCHITECTURE.md)  
**Prerequisite:** Phase 1 ternary core  
**Rule:** Work directly on `main` where practical; merge all completed and verified temporary-branch work into `main`.

## 1. Objective

Phase 2 develops an adaptive binary/ternary architecture that preserves conventional hardware and software compatibility. It does not claim commodity processors execute ternary instructions or contain ternary RAM.

## 2. Stage plan

### Stage A — Adaptive representation foundation
**Status: merged into main.** Binary, balanced-ternary, and hybrid choices; evidence-based deterministic selection; checked i128/u128 bridges; tests and CI.

### Stage B — Packed ternary logical memory
**Status: implementation delivered.** Five balanced trits per byte with checked construction, access, validation, and round-trip coverage. Backing storage remains conventional binary RAM. Testing is deferred until Phase 2 completion.

### Stage C — TIR execution foundation
**Status: implementation delivered.** Bounded interpreter foundation. TIR remains a standalone research/test runtime, not an application execution path. Testing is deferred until Phase 2 completion.

### Stage D — Translator (retired)
**Status: discontinued by architecture decision.** The former translator, TIR conversion workflow, and TRIONEX application path are retired. Stage G is the sole application execution engine. See [Stage D](Phase%202%20Stage%20D.md).

### Stage E — Runtime, kernel, and hardware integration
**Status: implementation delivered; testing deferred until Phase 2 completion.**

- Versioned target capability reporting in `ternary/platform`.
- Redox boot-time read-only capability scheme at `/scheme/trion`.
- Measured, correctness-checked adaptive integer workload published as runtime policy evidence.
- PCI and ACPI inventory evidence, with unprobed optional device features reported honestly.
- Existing Redox scheme/driver interfaces and byte-preserving binary MMIO, DMA, syscall, network, and storage boundaries.
- Initfs packaging, service ordering, and a one-shot endpoint integration checker.

See [Stage E](Phase%202%20Stage%20E.md) for implementation details and deferred verification.

### Stage F — Measurement, hardening, and release validation
**Status: implementation delivered; testing and measurements deferred until Phase 2 completion.**

- Equivalent binary, balanced-ternary, and hybrid workloads with semantic-result equality checks, raw timing samples, medians, host CPU/toolchain metadata, conversion overhead, and packed-memory storage/round-trip measurements.
- Cargo-fuzz targets for packed balanced-trit decoding and TRN3 deserialization, with bounded attacker-controlled allocations.
- Manual-dispatch-only adaptive validation workflow covering tests, formatting, Clippy, benchmarks, and fuzz campaigns.
- Manual-dispatch-only release workflow for x86_64 image/ISO builds and QEMU checks of kernel entry, capability-service readiness, and report endpoint validation.
- No benchmark or fuzz campaign is run during implementation; preserve raw output, corpus and crash artifacts during the post-Phase-2 validation pass.

See [Stage F](Phase%202%20Stage%20F.md).

### Stage G — Direct executable engine
**Status: planned; not implemented.**

- Directly load supported Windows PE32+ `.exe` files and macOS `.app` bundles.
- Load and validate original images; do not convert to TIR or create `.trx` files.
- No translator or TRIONEX fallback.
- Implement format-specific loaders, bounded instruction decoding/execution, platform compatibility services, process isolation, and resource/security controls.
- Treat instruction execution and Windows/macOS API compatibility as separate problems; advertise only tested compatibility.
- Reject unsupported instructions, imports, relocations, architectures, and malformed files safely.

See [Stage G](Phase%202%20Stage%20G.md) for architecture and completion gates.

## 3. Global engineering gates

1. Correctness and safety outrank representation choice.
2. Never infer unsupported hardware capabilities.
3. Conversions are explicit, checked, and semantics-preserving.
4. Malformed external input must not cause unchecked panics or out-of-bounds access in new components.
5. Ternary candidates require support and semantic validation.
6. Conventional hardware, network, file, and ABI boundaries retain their specified binary layouts.
7. Keep new crates isolated until APIs and implementation are stable.
8. Target `main`; merge any temporary branch after implementation.

## 4. Phase 2 exit criteria

Phase 2 implementation is complete only when Stages A–G are implemented to their stated scope. After implementation is complete, run the consolidated testing and validation pass, including automated checks, fuzz campaigns, comparative benchmarks, direct executable capability checks, compatibility-path tests, supported OS builds, and QEMU boot validation. Phase H — UI/UX/UC — is the separate final project phase, not a Phase 2 stage.
