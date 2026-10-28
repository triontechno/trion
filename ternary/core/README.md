# Trion ternary core

A standalone, host-testable first step toward Trion's ternary computing environment. This crate implements **software balanced ternary**; it does not imply ternary hardware.

## Representation

- Trit values are -1, 0, +1.
- Text digits are `T` (−1), `0`, `1`, most-significant trit first.
- Internal trit vectors are least-significant trit first.
- Zero has one canonical representation: `[0]`.
- Decimal conversion is bounded to Rust's `i128` range; ternary arithmetic itself operates on trit vectors.

## Build and test

From this directory:

```sh
cargo test
cargo run -- from-decimal 42
cargo run -- to-decimal 1T0
cargo run -- add 1T 11
cargo run -- mul 1T 11
```

The CLI uses exact integer arithmetic where a decimal result fits in `i128`. It reports an error if conversion back to `i128` overflows.

## Scope

This is Phase 1 of Trion's ternary roadmap: arithmetic, canonical representation, parsing/formatting, and tests. It is deliberately isolated from the existing OS build so it can be validated on an ordinary development machine before integration.
