# TRION Phase 2 — Stage B: Packed Ternary Logical Memory

**Status:** Implementation  
**Parent:** [Phase 2 master plan](Phase%202.md) and [TRION architecture](../architecture/TRION-ARCHITECTURE.md)

## 1. Purpose

Provide a safe software abstraction for storing balanced ternary values compactly in ordinary byte-addressed memory. Stage B does not modify the kernel's physical frame allocator and does not claim that binary DRAM has become ternary RAM.

## 2. Encoding

Each logical balanced trit is in {-1, 0, +1}. Map it to an unbalanced positional digit:

- -1 → 0
- 0 → 1
- +1 → 2

Five U3 digits are packed least-significant-trit first into one byte:

`byte = d0 + 3*d1 + 9*d2 + 27*d3 + 81*d4`

The valid packed byte range is 0..=242 because 3^5 = 243. Values 243..=255 are invalid.

Five logical zero trits encode as 121. If the logical length is not a multiple of five, unused high trit positions in the final byte MUST be canonical zero trits (U3 digit 1). This makes the byte sequence canonical for a declared trit length.

The in-memory API carries the logical trit count separately; raw packed bytes without a count are not self-describing.

## 3. API

The `trion-ternary-memory` crate provides:

- `PackedTritBuffer::new_zeroed(len)`
- `PackedTritBuffer::from_balanced_trits(values)`
- `PackedTritBuffer::from_packed_bytes(len, bytes)`
- checked `get(index)` and `set(index, trit)`
- `to_balanced_trits()`, `as_packed_bytes()`, and `len()`

All public input validation is checked. Invalid digits, out-of-range indices, malformed byte values, invalid padding, and allocation/length failures return explicit errors.

## 4. Non-goals

Stage B does not implement physical ternary RAM, virtual-memory paging, a kernel allocator, DMA/MMIO ternary layouts, a persistent filesystem format, or a guaranteed 40 GB → 25 GB capacity improvement.

## 5. Verification

```sh
cargo test --manifest-path ternary/memory/Cargo.toml
cargo fmt --manifest-path ternary/memory/Cargo.toml -- --check
cargo clippy --manifest-path ternary/memory/Cargo.toml --all-targets -- -D warnings
```

Acceptance requires exhaustive testing of all 243 possible packed-byte values, rejection of invalid byte values and noncanonical padding, mutation/read round trips, empty buffers, partial final bytes, and length boundaries.
