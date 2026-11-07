# TRION Platform Integration

This crate provides target capability reporting and a boundary between the shared adaptive policy and platform-facing runtime code.

## What it currently does

- Reports compile-target architecture, operating system, pointer width, endianness, and declared atomic widths.
- Reports the bounded TIR research/test runtime separately from ordinary binary execution.
- Never claims native ternary hardware from a CPU architecture name.
- Delegates adaptive representation choices to `trion-adaptive`, requiring caller-supplied comparable costs and semantic-validation evidence.
- Labels conventional MMIO, DMA, driver ABI, syscall ABI, network, and persistent-format byte slices without rewriting them.

## Commands

```sh
cargo test --manifest-path ternary/platform/Cargo.toml
cargo run --manifest-path ternary/platform/Cargo.toml
cargo fmt --manifest-path ternary/platform/Cargo.toml -- --check
cargo clippy --manifest-path ternary/platform/Cargo.toml --all-targets -- -D warnings
```

## Limits

This is a user-space software foundation. It does not probe devices, map MMIO, allocate DMA memory, modify kernel syscalls, or prove Redox boot integration. Those remain Stage E completion gates documented in [Phase 2 Stage E](../../Phase%202%20Stage%20E.md).