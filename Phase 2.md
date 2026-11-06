# TRION OS — Phase 2: Adaptive Binary/Ternary Computing

**Status:** Active implementation  
**Parent:** [TRION-ARCHITECTURE.md](TRION-ARCHITECTURE.md)  
**Prerequisite:** Phase 1 ternary core and translator foundation  
**Rule:** All implementation work targets `main`. If direct work on `main` is unsuitable, use a temporary branch and merge verified work back into `main`.

## 1. Objective

Phase 2 turns TRION's adaptive binary/ternary architecture into working software components. It must preserve conventional binary compatibility while making ternary representations usable, measurable, and independently testable.

The Phase 2 deliverable is not a claim that commodity processors execute ternary instructions or contain ternary RAM. It is a layered system that can represent, convert, select, execute, and eventually integrate ternary-aware workloads without breaking existing hardware and software interfaces.

## 2. Stage plan

### Stage A — Adaptive representation foundation
**Status: merged into main.**

- Binary, balanced-ternary, and hybrid representation choices.
- Capability, semantic-validation, compatibility, and cost inputs.
- Deterministic adaptive selection.
- Checked i128/u128 binary ↔ balanced-ternary bridges.
- Selection tests and CI.

### Stage B — Packed ternary logical memory
**Status: implemented and CI-verified in main.**

- Define a version-independent in-memory packed-trit buffer.
- Pack five balanced trits into each byte using the explicit mapping B3 {-1,0,+1} → U3 {0,1,2}.
- Validate all packed values and canonical zero padding.
- Provide checked construction, read, write, and conversion APIs.
- Test exhaustive one-byte encodings, boundaries, invalid data, and round trips.
- Document that the backing bytes are still held in conventional binary RAM.

Stage B is a logical storage abstraction, not a kernel physical-memory allocator and not physical ternary RAM.

### Stage C — TIR execution foundation
**Status: bounded interpreter implemented and CI-verified in main.**

- Define execution semantics for every TIR v1 operation.
- Build a bounded, deterministic TIR interpreter for the exact supported subset.
- Validate register IDs, instruction count, stack accesses, addresses, and return behavior.
- Add differential tests against equivalent native x86-64 snippets where safe test infrastructure permits.
- Keep parse, validation, execution, and executable generation as separate capabilities.
- Do not execute untrusted TIR in the kernel.

### Stage D — Translator expansion and executable compatibility
**Status:** TRIONEX v1 container and validated TIR packaging CLI implemented; instruction expansion and executable/runtime compatibility remain in progress.

- Expand instruction support only in semantically complete, independently tested groups.
- Add control-flow and relocation handling before accepting binaries that require them.
- Specify a versioned native TRION executable container and loader contract before calling it executable.
- Preserve ordinary ELF/binary support and provide explicit fallbacks for unsupported code.
- Implement ternary-to-binary output only for a declared target ISA and ABI.
- Require differential semantic tests and reject unsupported instructions rather than approximating them.

See [Stage E]("Phase%202%20Stage%20E.md") for the capability-report contract and remaining kernel integration gates.

### Stage E — Runtime, kernel, and hardware integration
**Status:** User-space capability-report foundation implemented and CI-verified; Redox kernel, driver, DMA/MMIO, syscall/ABI, and verified hardware integration remain open.

- Add target capability discovery and a stable capability report.
- Integrate adaptive decisions first in user-space libraries and controlled runtime paths.
- Define memory, driver, DMA, MMIO, syscall, and ABI boundaries explicitly.
- Only then integrate selected APIs into Redox kernel subsystems, with architecture-specific builds and boot tests.
- Do not change physical frame allocation or page-table representation merely to label memory ternary.
- Keep network protocols and conventional device interfaces binary-compatible.

See [Stage F]("Phase%202%20Stage%20F.md") for benchmark, fuzzing, and release-validation gates.

### Stage F — Measurement, hardening, and release validation
**Status:** Initial representation benchmark harness and deterministic malformed-input regression tests implemented and CI-verified; sustained fuzzing, comparative result publication, full OS build, and QEMU boot validation remain open.

- Benchmark binary, ternary, and hybrid implementations on equivalent workloads.
- Include conversion, memory, metadata, and compatibility overhead.
- Fuzz packed-memory decoders, TIR parsers, and translator input handling.
- Run tests, formatting, Clippy, architecture builds, and QEMU boot validation.
- Publish measured results; do not claim speed, capacity, energy, or security gains without evidence.

## 3. Global engineering gates

1. Correctness and safety outrank representation choice.
2. No unsupported hardware capability may be inferred.
3. Every conversion is explicit, checked, and semantics-preserving.
4. No malformed external input may cause an unchecked panic or out-of-bounds access in the new components.
5. A ternary candidate is not selected without support and semantic validation.
6. Conventional hardware, network, file, and ABI boundaries retain their specified binary layout.
7. Tests must cover both success and rejection paths.
8. New crates remain isolated until their APIs and tests are stable; kernel integration is a later stage.
9. The default destination is `main`; temporary branches must be merged back after verification.

## 4. Phase 2 exit criteria

Phase 2 is complete only when Stages A–F have been implemented to their stated scope, automated checks pass, translator/runtime capabilities are accurately described, compatibility paths are tested, and the supported OS build boots in the declared QEMU configurations.

A successful library test suite alone does not prove that the OS boots or that ternary hardware exists.
