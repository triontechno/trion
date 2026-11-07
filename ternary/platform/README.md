# TRION Platform Integration

This crate reports verified build-target facts and connects the shared adaptive representation policy to platform-facing runtime workloads.

## Capability report

`CapabilityReport::for_build_target()` reports architecture, OS, pointer width, endianness, declared atomic widths, the bounded legacy TIR test subset, and binary external-interface boundaries. It never infers native ternary hardware.

## Adaptive runtime workload

`run_adaptive_integer_probe(iterations)` runs a bounded binary integer workload and the equivalent balanced-ternary encode/decode workload, measures both, checks that their checksums match, and only then calls the shared adaptive selector with those measured costs. The iteration count is capped at 100,000.

## Redox service

The `trion-capd` daemon publishes capability and runtime evidence through the read-only `/scheme/trion` scheme:

- `capabilities`
- `runtime-policy`
- `hardware-evidence`

The service uses the existing Redox SchemeSync ABI. PCI evidence is limited to scheme availability and enumerated function count; optional device features remain unprobed. See [Stage E](../../Phase%202%20Stage%20E.md).

## Commands

```sh
cargo test --manifest-path ternary/platform/Cargo.toml
cargo run --manifest-path ternary/platform/Cargo.toml
cargo test --manifest-path src/base/Cargo.toml -p trion-capd
```

No code here claims native ternary CPU, RAM, DMA, or device support. Conventional hardware, kernel, network, and persistent-format boundaries retain their binary layouts.
