# Trion ternary core

This host-testable crate implements software balanced ternary and checked unbalanced-trit packing. It runs on ordinary binary hardware and makes no claim of native ternary instructions.

## Representations and integer bridge

`Ternary` stores balanced trits `-1, 0, +1` least-significant first. Its unique canonical representation has no high zero trits; zero is `[0]`. Display and parsing use most-significant-first `T`, `0`, `1` text. This is a signed multi-trit integer representation: an n-trit word represents the exact integer `sum(trit[i] * 3^i)` and spans `[-(3^n-1)/2, +(3^n-1)/2]`.

`Ternary::from_i128` and `from_u128` convert exactly to arbitrary-length balanced values; `to_i128` and `to_u128` are checked conversions. Negative-to-unsigned conversion and integer overflow return errors. `try_add`, `try_sub`, `try_negated`, and `try_mul` return allocation/size failures; arithmetic does not wrap or saturate. Existing `add`, `sub`, `negated`, and `mul` convenience methods retain their infallible signatures.

Unbalanced digits U3 `{0, 1, 2}` are a separate domain. `pack_u3_u8/u16/u32/u64` and matching `unpack_` functions store `sum(digit[i] * 3^i)`, with trit 0 least significant. Capacities are 5, 10, 20, and 40 trits, respectively. All packed decoders validate count and range. `u3_to_balanced` and `balanced_to_u3` convert exactly one digit; they do not encode/decode signed multi-trit integers. `compare_u3_u32/u64` validates both operands for the same declared count before comparison.

## TRN3 serialization

`serialize_balanced` and `deserialize_balanced` use the stable version 1 format:

| Offset | Size | Meaning |
|---|---:|---|
| 0 | 4 | ASCII magic `TRN3` |
| 4 | 1 | Version `1` |
| 5 | 1 | Domain `1` (balanced B3) |
| 6 | 4 | Trit count, unsigned little-endian |
| 10 | count | U3 byte codes in least-significant-first order: `0` = B3 −1, `1` = 0, `2` = +1 |

The only canonical zero is count 1 with payload `[1]`. Noncanonical leading zeroes, extra/truncated data, invalid digits, unsupported versions, and unsupported domains are rejected. Future incompatible layouts must use a new version; unknown versions are never guessed. The serialization is a conventional binary container; payload encoding is explicitly ternary.

## CLI

From this directory:

```sh
cargo test
cargo run -- from-decimal 42
cargo run -- to-decimal 1T0
cargo run -- add 1T 11
cargo run -- mul 1T 11
cargo run -- pack-u8 0,1,2
cargo run -- unpack-u64 42 4
```

Binary interfaces, including MMIO, DMA, network packets, and existing file formats, remain binary unless a documented interface explicitly adopts a ternary format.
