# trion-capd

Redox initfs daemon for TRION's adaptive platform capability interface.

## Published read-only scheme

The daemon registers `/scheme/trion` and exposes:

- `capabilities` — schema-v3 compile-target facts from `trion-platform`.
- `runtime-policy` — measured costs and correctness result from the bounded adaptive integer probe.
- `hardware-evidence` — PCI scheme availability, enumerated function count, and bounded registration-wait attempts. Optional device features remain unprobed.

The service starts as part of `40_drivers.target`, after the hardware manager dependency. It uses the existing Redox `SchemeSync` API and does not add a syscall or modify a kernel ABI.

## Security and failure behavior

- Report files are read-only.
- Reads honor offsets and return EOF beyond the report length.
- Invalid handles and unknown paths return errors.
- The daemon waits up to five seconds for asynchronously spawned `pcid` to register `/scheme/pci`. PCI scheme absence after the bounded wait is reported as unavailable; it does not prevent the capability service from starting.
- No unverified CPU/device feature is advertised as supported.

## Build and test

```sh
cargo test --manifest-path src/base/Cargo.toml -p trion-capd
cargo check --manifest-path src/base/Cargo.toml -p trion-capd
cargo fmt --manifest-path src/base/Cargo.toml -p trion-capd -- --check
cargo clippy --manifest-path src/base/Cargo.toml -p trion-capd --all-targets -- -D warnings
```
