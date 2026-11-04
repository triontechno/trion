# TRION TIR runtime

A bounded interpreter for the exact TRION TIR v1 subset currently emitted by the x86-64 translator:

- `NOP`
- `MOV_IMM64 rN <balanced-ternary-immediate>`
- `RET`

The runtime validates the complete textual TIR stream before mutating machine state. It executes linearly because TIR v1 currently has no branch or call operations. `RET` reads an eight-byte little-endian return address from the caller-provided memory at RSP, then advances RSP by eight.

The runtime enforces input and instruction-count limits, checks register and immediate encodings, and checks stack address arithmetic and bounds. Failed execution leaves the caller's register state unchanged.

This is a software interpreter running on the host CPU. It is not a native ternary processor, a kernel executor, a sandbox for arbitrary machine code, or a native executable loader.

## Test

```sh
cargo test --manifest-path ternary/runtime/Cargo.toml
cargo fmt --manifest-path ternary/runtime/Cargo.toml -- --check
cargo clippy --manifest-path ternary/runtime/Cargo.toml --all-targets -- -D warnings
```
