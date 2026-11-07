# TRION OS — Phase 2: Adaptive Binary/Ternary Computing

**Status:** Active implementation  
**Parent:** [TRION-ARCHITECTURE.md](TRION-ARCHITECTURE.md)  
**Prerequisite:** Phase 1 ternary core  
**Rule:** All implementation work targets `main`. If direct work on `main` is unsuitable, use a temporary branch and merge verified work back into `main`.

## 1. Objective

Phase 2 turns TRION's adaptive binary/ternary architecture into working software components. It preserves conventional binary compatibility while making ternary representations usable, measurable, and independently testable. It does not claim commodity processors execute ternary instructions or contain ternary RAM.

## 2. Stage plan

### Stage A — Adaptive representation foundation
**Status: merged into main.** Binary, balanced-ternary, and hybrid choices; evidence-based deterministic selection; checked i128/u128 bridges; tests and CI.

### Stage B — Packed ternary logical memory
**Status: implemented and CI-verified in main.** Five balanced trits are packed per byte with checked construction, access, validation, and round-trip tests. Backing storage remains conventional binary RAM.

### Stage C — TIR execution foundation
**Status: bounded interpreter implemented and CI-verified in main.** TIR remains a standalone research/test runtime, not an application execution path.

### Stage D — Translator (retired)
**Status: discontinued by architecture decision.** The former translator, TIR conversion workflow, and TRIONEX application path are retired. Stage G is the sole application execution engine. See [Stage D](Phase%202%20Stage%20D.md).

### Stage E — Runtime, kernel, and hardware integration
**Status:** User-space target capability reporting, adaptive-policy delegation, and byte-preserving binary-boundary contracts implemented in `ternary/platform`; Redox boot-time service integration and QEMU verification remain open.

- Report build-target facts without guessing optional hardware capabilities.
- Consume adaptive decisions using caller-supplied measured/validated evidence.
- Preserve binary MMIO, DMA, driver ABI, syscall ABI, network, and storage boundaries.
- Integrate a permission-controlled capability service into the Redox user-space boot path.
- Validate supported Redox targets and QEMU boot behavior.
- Do not alter physical frame allocation or page tables merely to label memory ternary.

See [Stage E](Phase%202%20Stage%20E.md) for remaining completion gates.

### Stage F — Measurement, hardening, and release validation
**Status:** Initial benchmark harness and deterministic malformed-input regression tests implemented; sustained fuzzing, comparative result publication, full OS build, and QEMU boot validation remain open.

- Benchmark equivalent binary, ternary, and hybrid workloads including conversion and compatibility overhead.
- Fuzz packed-memory decoders and, when Stage G exists, direct PE/Mach-O loaders and instruction decoding.
- Run tests, formatting, Clippy, architecture builds, and QEMU boot validation.
- Publish measured results; make no unsupported performance, capacity, energy, or security claims.

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
7. Test success and rejection paths.
8. Keep new crates isolated until APIs and tests are stable.
9. Target `main`; merge any temporary branch after verification.

## 4. Phase 2 exit criteria

Phase 2 is complete only when Stages A–G are implemented to their stated scope, automated checks pass, direct executable capabilities are accurately described, compatibility paths are tested, and the supported OS build boots in declared QEMU configurations. Phase H — UI/UX/UC — is the separate final project phase, not a Phase 2 stage.