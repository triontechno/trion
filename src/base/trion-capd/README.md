# trion-capd

Redox initfs daemon for TRION's adaptive platform capability interface.

## Published read-only scheme

The daemon registers `/scheme/trion` and exposes:

- `capabilities` — schema-v3 compile-target facts from `trion-platform`.
- `runtime-policy` — measured costs and correctness result from the bounded adaptive integer probe.
- `hardware-evidence` — PCI function inventory and ACPI symbol inventory exposed by Redox schemes. Optional device features remain unprobed.

The service starts after both `hwd` and the initfs PCI driver spawner. It uses the existing Redox `SchemeSync` API and does not add a syscall or modify a kernel ABI. ACPI/DeviceTree backend selection and probing remain owned by Redox's `hwd`; TRION records the published inventory without taking over those drivers.

## Security and failure behavior

- Report files are read-only.
- Reads honor offsets and return EOF beyond the report length.
- Invalid handles and unknown paths return errors.
- Missing PCI or ACPI schemes are reported as unavailable; they do not prevent the capability service from starting.
- No unverified CPU/device feature is advertised as supported.
- Inventory counts do not imply that optional device capabilities were probed.

## Build and test

Testing is deferred until Phase 2 implementation is complete.

```sh
cargo test --manifest-path src/base/Cargo.toml -p trion-capd
cargo check --manifest-path src/base/Cargo.toml -p trion-capd
cargo fmt --manifest-path src/base/Cargo.toml -p trion-capd -- --check
cargo clippy --manifest-path src/base/Cargo.toml -p trion-capd --all-targets -- -D warnings
```
