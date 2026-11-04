# TRION executable container

TRIONEX v1 (`.trx`) is a versioned interpreted program container carrying canonical textual TIR v1. It is not native machine code and is not yet a kernel-loadable OS executable.

Header size: 36 bytes. All integers are little-endian.

| Offset | Size | Field | Required value |
|---:|---:|---|---|
| 0 | 8 | Magic | `TRIONEX\0` |
| 8 | 2 | Version | 1 |
| 10 | 2 | Header length | 36 |
| 12 | 2 | Architecture | 1 = x86-64 TIR register model |
| 14 | 2 | Payload kind | 1 = textual TIR v1 |
| 16 | 4 | Flags | 0 |
| 20 | 4 | Reserved | 0 |
| 24 | 8 | Payload length | Exact byte count |
| 32 | 4 | CRC-32/ISO-HDLC | Payload checksum |

The file length must equal 36 plus payload length. Trailing bytes, unsupported fields, checksum mismatch, and invalid TIR are rejected.

API: `TrionExecutable::from_tir`, `parse`, `to_bytes`, and `payload`.

Run tests with `cargo test --manifest-path ternary/executable/Cargo.toml`.
