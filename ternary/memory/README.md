# TRION packed ternary memory

This crate stores five balanced trits in one ordinary byte. It is a logical ternary representation backed by binary storage.

## Encoding

Balanced trits map to U3 digits by adding one:

| Balanced trit | Packed digit |
|---:|---:|
| -1 | 0 |
| 0 | 1 |
| +1 | 2 |

For five digits, the byte value is `d0 + 3*d1 + 9*d2 + 27*d3 + 81*d4`. Values 0 through 242 are valid; 243 through 255 are reserved and rejected.

The caller must preserve the logical trit count. Unused positions in a partial final byte must encode zero trits, making padding canonical.

## Safety and scope

All public construction and mutation APIs validate trit values, lengths, packed-byte ranges, and padding. No unsafe code is used. The crate does not implement physical ternary RAM, virtual-memory paging, DMA, or a filesystem format.

## Test

```sh
cargo test --manifest-path ternary/memory/Cargo.toml
cargo fmt --manifest-path ternary/memory/Cargo.toml -- --check
cargo clippy --manifest-path ternary/memory/Cargo.toml --all-targets -- -D warnings
```
