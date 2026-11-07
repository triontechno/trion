# TRION Phase 2 — Stage E: Runtime and Platform Integration

<badge>Implementation delivered; automated verification pending</badge>

Stage E connects the adaptive binary/ternary runtime to the actual Redox user-space boot and platform service model. It does not implement application loading; direct Windows `.exe` and macOS `.app` execution belongs exclusively to Stage G.

## Implemented components

### 1. Target capability reporting

The `trion-platform` crate provides a versioned schema-v3 capability report containing:

- Target architecture and operating system.
- Pointer width and target endianness.
- Compiler-declared atomic widths.
- Ordinary binary execution support.
- The exact bounded TIR research/test subset.
- Explicit binary external-interface boundaries.
- Native ternary hardware remains false unless a future hardware-specific adapter verifies it.

Optional CPU and device features are never inferred from an architecture string.

### 2. Redox boot-time capability service

The `trion-capd` daemon is part of the `src/base` Cargo workspace and initfs binary list. The `40_drivers.target` starts it after the hardware-manager dependency.

It registers a read-only Redox scheme at `/scheme/trion` and exposes:

| Endpoint | Contents |
|---|---|
| `/scheme/trion/capabilities` | Schema-v2 target capability report |
| `/scheme/trion/runtime-policy` | Measured adaptive workload costs, correctness checksum, and selected representation |
| `/scheme/trion/hardware-evidence` | PCI scheme availability and enumerated function count, with explicit unknown/unprobed feature status |

The scheme uses the existing Redox `SchemeSync` interface. It does not add a kernel syscall or change an existing ABI. Report handles are read-only, offset-aware, and reject writes.

### 3. Real adaptive runtime workload

At service startup, a bounded integer workload runs through two paths:

- Binary integer accumulation.
- Balanced-ternary encode/decode followed by accumulation.

Both paths process the same bounded values. Their results must match before measured elapsed costs are passed to the shared adaptive selector. The selected representation and measured costs are published through the runtime-policy endpoint. The workload is capped at 100,000 iterations.

This is a concrete measured software workload, not a claim that the CPU executes native ternary instructions.

### 4. Hardware and ABI integration

- The capability service checks for the existing PCI scheme and counts enumerated PCI function entries after the hardware manager dependency.
- ACPI and DeviceTree discovery remain the responsibility of Redox's existing `hwd` service.
- PCI presence/count is inventory evidence only; optional device capabilities remain explicitly unprobed.
- MMIO registers, DMA descriptors, driver ABIs, syscall structures, network frames, and persistent formats remain byte-for-byte binary.
- Packed ternary memory remains a logical representation backed by ordinary memory. It is not wired into physical page allocation or DMA.
- No native ternary CPU, RAM, or peripheral support is claimed.

### 5. Build and verification automation

- Adaptive-policy CI tests the platform probe and all existing ternary crates.
- A nightly CI job tests, checks, formats, and lints `trion-capd`.
- The Redox initfs Makefile builds and packages the daemon.
- The image-release workflow builds the OS and its QEMU smoke test requires both kernel entry and the capability daemon's readiness log.

## Verification commands

Host-side tests:

```sh
cargo test --manifest-path ternary/adaptive/Cargo.toml
cargo test --manifest-path ternary/platform/Cargo.toml
cargo test --manifest-path src/base/Cargo.toml -p trion-capd
cargo check --manifest-path src/base/Cargo.toml -p trion-capd
cargo fmt --manifest-path src/base/Cargo.toml -p trion-capd -- --check
cargo clippy --manifest-path src/base/Cargo.toml -p trion-capd --all-targets -- -D warnings
```

The repository image workflow additionally runs the full Redox image build and QEMU boot smoke test. Stage E may be marked fully verified only after those required workflow jobs pass on `main`.

## Scope and limitations

- The current boot integration is validated against the repository's x86_64 QEMU image target. Other architecture targets must not be advertised as boot-verified until their own builds and boots pass.
- PCI enumeration reports only what the existing PCI scheme exposes. It does not probe arbitrary device features, benchmark devices, or infer ternary capability.
- A missing PCI scheme is reported as unavailable rather than causing service failure.
- There is no ternary physical-memory allocator, page-table encoding, kernel TIR execution, translator, TRIONEX path, or application executable engine in Stage E.
- Direct `.exe` and `.app` loading/execution remain Stage G work.

## Completion criteria

Stage E implementation is delivered. Verification is complete when the adaptive-policy tests, Redox capability-service tests/checks/lints, full x86_64 image build, and QEMU service-readiness smoke test all pass. A failed or unavailable verification run must remain visible as incomplete; documentation alone is not proof of a successful boot.
