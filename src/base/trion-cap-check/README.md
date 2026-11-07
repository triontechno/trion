# trion-cap-check

One-shot initfs integration test for the TRION capability service.

The service runs after `trion-capd` during `40_drivers.target`. It opens and validates:

- `/scheme/trion/capabilities` — schema-v3 target facts and conservative hardware-boundary claims.
- `/scheme/trion/runtime-policy` — five-sample median costs, bounded iteration count, checksum, and adaptive selection.
- `/scheme/trion/hardware-evidence` — PCI scheme availability/count and explicit unprobed/unknown device feature state.

It exits nonzero if any endpoint is missing, malformed, inconsistent, or advertises unverified native ternary hardware. The release QEMU smoke test requires its success log, proving the service is not merely compiled or registered but readable from another user-space process.

Run host-side checks with:

```sh
cargo test --manifest-path src/base/Cargo.toml -p trion-cap-check
cargo check --manifest-path src/base/Cargo.toml -p trion-cap-check
```
