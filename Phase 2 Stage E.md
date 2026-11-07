# TRION Phase 2 — Stage E: Runtime and Platform Integration

<badge>Implementation updated; testing deferred until Phase 2 completion</badge>

Stage E connects the adaptive binary/ternary runtime to the actual Redox user-space boot and platform service model. It does not implement application loading; direct Windows `.exe` and macOS `.app` execution belongs exclusively to Stage G.

## Implemented components

### 1. Target capability reporting

The `trion-platform` crate provides a versioned schema-v3 capability report containing target architecture, operating system, pointer width, endianness, compiler-declared atomic widths, ordinary binary execution support, and explicit binary external-interface boundaries. Optional CPU and device features are never inferred from an architecture string. Native ternary hardware remains unverified unless a future hardware-specific adapter proves it.

### 2. Redox boot-time capability service

The `trion-capd` daemon is part of the `src/base` Cargo workspace and initfs binary list. The `40_drivers.target` starts it after the hardware manager and initfs PCI driver spawner.

It registers a read-only Redox scheme at `/scheme/trion` and exposes:

| Endpoint | Contents |
|---|---|
| `/scheme/trion/capabilities` | Schema-v3 target capability report |
| `/scheme/trion/runtime-policy` | Measured adaptive workload costs, correctness checksum, and selected representation |
| `/scheme/trion/hardware-evidence` | PCI and ACPI inventory availability/counts, with explicit unknown/unprobed feature status |

The scheme uses the existing Redox `SchemeSync` interface. It does not add a kernel syscall or change an existing ABI. Report handles are read-only, offset-aware, and reject writes.

### 3. Runtime and kernel/service integration

At service startup, a bounded integer workload runs through binary accumulation and balanced-ternary encode/decode plus accumulation. Both paths process the same bounded values and must produce matching results. Five interleaved samples are measured; median costs are passed to the shared adaptive selector and published through the runtime-policy endpoint.

The service is integrated with Redox's existing daemon readiness, scheme registration, namespace restriction, and request-processing lifecycle. It deliberately uses existing kernel and scheme interfaces: Stage E does not introduce a new syscall, alter kernel ABI layouts, or claim that the kernel executes TIR or native ternary instructions.

### 4. Hardware and device-boundary integration

- The service inventories entries exposed by `/scheme/pci` and `/scheme/acpi/symbols` after Redox hardware discovery and PCI driver spawning.
- ACPI and DeviceTree backend selection/probing remain owned by Redox's existing `hwd` service.
- PCI and ACPI counts are inventory evidence only; they do not imply that optional device features have been probed.
- Missing inventory schemes are reported as unavailable, not silently treated as proof that no hardware exists.
- MMIO registers, DMA descriptors, driver ABIs, syscall structures, network frames, and persistent formats remain byte-for-byte binary.
- Packed ternary memory remains a logical representation backed by ordinary memory. It is not wired into physical page allocation or DMA.
- No native ternary CPU, RAM, or peripheral support is claimed.

### 5. Initfs packaging and service startup

- `trion-capd` and `trion-cap-check` are members of the `src/base` Cargo workspace.
- Both binaries are included in `INITFS_BINS` and copied into the generated initfs.
- `40_drivers.target` includes the capability daemon and one-shot integration checker.
- `50_trion-capd.service` has weak ordering dependencies on both `40_hwd.service` and `40_pcid-spawner-initfs.service`.
- `55_trion-cap-check.service` runs after the daemon and reads all three published endpoints, validating their schemas and conservative claims.
- The integration checker rejects inconsistent availability/count pairs and unverified native-ternary claims.

## Verification commands

Testing is intentionally deferred until all Phase 2 implementation stages are complete. At that point run:

```sh
cargo test --manifest-path ternary/adaptive/Cargo.toml
cargo test --manifest-path ternary/platform/Cargo.toml
cargo test --manifest-path src/base/Cargo.toml -p trion-capd
cargo test --manifest-path src/base/Cargo.toml -p trion-cap-check
cargo check --manifest-path src/base/Cargo.toml -p trion-capd -p trion-cap-check
cargo fmt --manifest-path src/base/Cargo.toml -p trion-capd -p trion-cap-check -- --check
cargo clippy --manifest-path src/base/Cargo.toml -p trion-capd -p trion-cap-check --all-targets -- -D warnings
```

The full Redox image build and QEMU boot validation also belong to the post-Phase-2 test pass. Stage E implementation must not be represented as boot-verified before those checks run.

## Scope and limitations

- The intended boot integration target is the repository's x86_64 QEMU image. Other architecture targets must not be advertised as boot-verified until separately checked.
- PCI/ACPI inventory records only what the existing Redox schemes expose. It does not probe arbitrary device features, benchmark devices, or infer ternary capability.
- There is no ternary physical-memory allocator, page-table encoding, kernel TIR execution, translator, TRIONEX path, or application executable engine in Stage E.
- Direct `.exe` and `.app` loading/execution remain Stage G work.
