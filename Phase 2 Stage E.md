# TRION Phase 2 — Stage E: Runtime and Platform Integration

**Implementation status:** User-space target reporting, adaptive-policy integration, and binary-boundary contracts are implemented in `ternary/platform`. **Stage E is not complete:** Redox boot-time service integration and QEMU verification remain outstanding.

## Delivered

- `ternary/platform` exposes a versioned schema-v2 capability report.
- Reports compile-target architecture, OS, pointer width, endianness, and compiler-declared atomic widths.
- Distinguishes ordinary binary execution from the isolated software TIR research/test runtime.
- Explicitly reports only NOP, MOV_IMM64, and RET for the bounded TIR v1 test runtime.
- Never infers native ternary hardware from an architecture name.
- Delegates representation selection to `trion-adaptive`; callers supply comparable costs and semantic-validation evidence.
- Defines byte-preserving boundary labels for MMIO, DMA descriptors, driver ABIs, syscall ABIs, network frames, and persistent formats.
- Adds tests for target facts, conservative hardware reporting, adaptive selection, and byte-preserving boundaries.
- CI includes the platform crate in pull-request path filters and test/format/lint jobs.

## Runtime and hardware contract

1. Capability reports describe verified facts, not aspirations. Architecture names do not imply optional CPU/device features.
2. Adaptive selection consumes caller-supplied comparable candidate costs and semantic-validation flags. The platform layer fabricates no benchmark data.
3. The legacy TIR test runtime remains isolated in user space. It is not an application loader, launcher, kernel execution path, or Stage G fallback.
4. Physical page allocation, page tables, DMA descriptors, MMIO registers, driver ABIs, syscalls, and network frames retain their established binary representation.
5. Packed ternary memory remains a logical runtime data structure over ordinary binary-backed memory.
6. Redox device drivers are user-space services. Device-specific discovery belongs in the relevant driver/service and requires actual device evidence.
7. Capability consumers must validate the schema version and reject unsupported versions rather than silently assume capabilities.

## Remaining completion gates

Stage E remains open until all of these are implemented and verified:

- A Redox-native boot-time capability service publishes the schema-v2 report through an explicit, permission-controlled system interface.
- The service collects only capabilities supported by platform/driver evidence and records the evidence source; unprobed capabilities remain unknown/false.
- At least one real, bounded user-space workload consumes the adaptive policy with measured or reviewed comparable costs and semantic tests.
- Kernel/user-space interfaces are documented against existing Redox syscall, scheme, memory-map, and driver-service contracts. Do not invent a syscall solely to expose the report.
- DMA/MMIO/device paths retain hardware-required layouts and ordering; no ternary reinterpretation is introduced.
- Cross-target builds and QEMU boot tests verify service startup, target facts, and safe failure when a source is unavailable.
- Any native ternary hardware claim has a concrete adapter, identification method, validation procedure, and test evidence.

## Explicit non-goals

- No ternary physical-memory allocator or page-table encoding on conventional hardware.
- No ternary reinterpretation of DMA descriptors, MMIO registers, syscall structures, driver ABIs, or network packets.
- No claim of native ternary CPU, RAM, or device support.
- No executable loader or application engine; direct `.exe`/`.app` loading and execution belong exclusively to Stage G.
- No TIR execution in the kernel and no translator/TRIONEX path.

## Verification

Run from the repository root:

```sh
cargo test --manifest-path ternary/adaptive/Cargo.toml
cargo test --manifest-path ternary/platform/Cargo.toml
cargo fmt --manifest-path ternary/platform/Cargo.toml -- --check
cargo clippy --manifest-path ternary/platform/Cargo.toml --all-targets -- -D warnings
```

Host-side checks do not prove Redox cross-compilation, boot-time service integration, hardware capability discovery, or QEMU bootability.

## Exit criteria

Stage E is complete only when every remaining completion gate above has an implementation and reproducible verification evidence. The current platform crate is a tested user-space foundation, not completed kernel/hardware integration.