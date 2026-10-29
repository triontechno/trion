# Phase 1 implementation notes

**Status:** Partial implementation on `spec/phase-1-ternary-foundations`. This is not a production-readiness claim.

## Implemented

### Core crate: packed unbalanced trits

Added checked public APIs in `ternary/core/src/lib.rs`:

- `pack_u3_u32` / `unpack_u3_u32`
- `pack_u3_u64` / `unpack_u3_u64`
- `u3_to_balanced` / `balanced_to_u3`
- `U3_U32_CAPACITY` (20) and `U3_U64_CAPACITY` (40)
- `PackedTritError` for invalid digits, excessive counts, and non-canonical packed values.

The packing order is least-significant trit first. Decoding takes an explicit count and rejects values that cannot be represented by exactly that many trits; it does not silently discard high trits. Zero trits are supported as an empty sequence encoded by zero. Balanced trits remain distinct from packed unbalanced digits.

Unit tests were added for exhaustive small sequences, u32/u64 maximum-capacity patterns, invalid digits and counts, non-canonical values, zero-count behavior, and single-digit domain conversion.

### Translator frontend

Updated `ternary/translator/src/main.rs` to report counts of candidate and non-candidate mnemonics separately. Output text emphasizes that these are heuristic triage categories only and that no translated executable is generated. This makes the current inspection-only behavior more explicit; it does not implement instruction translation.

## Deliberately not implemented yet

- A persistent serialized ternary format or versioned container.
- A canonical multi-trit signed integer wire/storage encoding. The existing `Ternary` type is a balanced-trit arithmetic representation, but it is not defined here as a serialization ABI.
- Binary/ternary bridge APIs for OS, drivers, networking, MMIO, or DMA.
- Compiler integration for `native_binary`, `ternary_logic`, or `trit_bridge` annotations.
- Semantic instruction translation, code generation, VM/interpreter output, or execution validation.
- Benchmark claims or OS build/boot validation.

## Validation status

Tests have been authored but **were not run in this editing session**. Run from the repository root:

```sh
cargo test --manifest-path ternary/core/Cargo.toml
cargo check --manifest-path ternary/translator/Cargo.toml
```

The translator check may require fetching its declared crates. Record the exact command output and toolchain version when validating. Passing these crate-level commands would not establish a successful full TRION OS build, boot, or whole-program translation.

## Next steps

1. Run the two commands above in CI or a Rust environment and fix any compiler/test failures.
2. Add property/fuzz tests for packed decoding and malformed serialized inputs.
3. Decide and document signed multi-trit encoding before exposing it as a stable format.
4. Add a translator test fixture proving inspection output never claims translated output.
5. Design bridge interfaces separately before integrating ternary representations into hardware-facing paths.
