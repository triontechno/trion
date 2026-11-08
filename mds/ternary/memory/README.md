# TRION packed ternary memory

This crate stores five balanced trits in one ordinary byte. It is a logical ternary representation backed by binary storage.

## Encoding

Balanced trits map to U3 digits by adding one: -1 → 0, 0 → 1, +1 → 2.

Five digits are packed least-significant first:
`byte = d0 + 3*d1 + 9*d2 + 27*d3 + 81*d4`.

Values 0..=242 are valid; 243..=255 are reserved. The logical trit count must be retained separately. Unused positions in a partial final byte must encode zero trits, making padding canonical.

## Safety and scope

The crate validates values, lengths, packed-byte ranges, and padding. It uses no unsafe code. It does not implement physical ternary RAM, virtual-memory paging, DMA, or a filesystem format.

## Verification

```sh
cargo test --manifest-path ternary/memory/Cargo.toml
cargo fmt --manifest-path ternary/memory/Cargo.toml -- --check
cargo clippy --manifest-path ternary/memory/Cargo.toml --all-targets -- -D warnings
```


Stage B validation note: the canonical decoder rejects reserved byte values and nonzero padding trits; the test suite exhaustively covers all 243 valid packed-byte patterns.
