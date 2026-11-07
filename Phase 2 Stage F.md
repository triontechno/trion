# TRION Phase 2 — Stage F: Measurement, Hardening, and Release Validation

<badge>Implementation delivered; all execution deferred until Phase 2 completion</badge>

Stage F supplies the measurement, fuzzing, and release-validation infrastructure for Phase 2. It does not execute those campaigns during implementation.

## 1. Comparative benchmark implementation

`ternary/benchmarks` compares equivalent deterministic integer-sum workloads across:

- **Binary:** conventional `i128` accumulation.
- **Balanced ternary:** convert each input to `Ternary`, accumulate using checked balanced-ternary addition, then convert the result to the binary output boundary.
- **Hybrid:** convert each input into the internal balanced-ternary representation, convert it back at the conventional binary boundary, and accumulate in `i128`.
- **Packed-memory round trip:** encode and decode a balanced-trit sequence using `PackedTritBuffer`.

Every measured candidate must return the same result; the harness rejects mismatched results. It records seven raw timing samples per candidate, medians, input count, host architecture/OS/CPU, pointer width, Rust compiler version, backing-storage sizes, and packed-storage ratios. It reports diagnostic measurements only and does not assert that ternary is faster or more efficient.

Run after Phase 2 implementation:

```sh
cargo run --manifest-path ternary/benchmarks/Cargo.toml --release
rustc -Vv
```

Retain raw output and host/toolchain metadata for every comparison. Repeat on quiet hosts and supported target architectures before making any general claim.

## 2. Fuzzing infrastructure

The standalone `ternary/fuzz` cargo-fuzz package contains:

- `packed_trit_buffer`: arbitrary packed-byte streams, logical lengths, reserved byte values, padding validation, and canonical decode/re-encode invariants. Fuzzer-controlled allocations are capped at one million logical trits.
- `trn3_deserialize`: arbitrary TRN3 input, malformed headers/counts/payloads, and canonical deserialize/serialize round trips.

Run after Phase 2 implementation:

```sh
cargo install cargo-fuzz --locked
cd ternary/fuzz
cargo fuzz run packed_trit_buffer -- -max_total_time=60 -max_len=1048584
cargo fuzz run trn3_deserialize -- -max_total_time=60 -max_len=1048576
```

Preserve corpus, crash artifacts, toolchain versions, campaign duration, and coverage. Stage G must add dedicated PE32+ and Mach-O loader/instruction-decoder fuzz targets when those parsers are implemented.

## 3. Automated validation workflows

- `.github/workflows/adaptive.yml` contains adaptive, packed-memory, TIR research-runtime, platform, capability-service, formatting, lint, benchmark, and bounded fuzz validation.
- The adaptive workflow is **manual-dispatch only** so implementation pushes do not launch tests before Phase 2 is complete.
- `.github/workflows/trion-release.yml` builds the x86_64 desktop image and live ISO, verifies artifacts, and runs QEMU checks for kernel entry, TRION capability-service readiness, and user-space validation of the three capability endpoints.
- The release workflow is also **manual-dispatch only** during implementation. Run it in the consolidated post-Phase-2 validation pass; it publishes a GitHub Release, so dispatch only when a release is intended.

## 4. Release acceptance gates

Stage F's infrastructure is implemented. Its validation evidence is intentionally pending and must be produced after all Phase 2 implementation stages are complete:

1. Unit, integration, formatting, and Clippy workflows pass.
2. Binary, ternary, and hybrid benchmarks produce equal semantic results and retain raw comparative measurements with host/toolchain metadata.
3. Fuzz campaigns complete without unresolved crashes; findings are fixed and regression fixtures retained.
4. Supported OS image and live ISO build successfully.
5. QEMU observes kernel entry, `/scheme/trion` registration, and successful user-space validation of all three capability endpoints.
6. Any Stage G direct-engine tests and loader fuzz targets are added when Stage G implementation exists; no executable compatibility is claimed before that.

A green library workflow alone is not a release gate. No performance, density, energy, security, or hardware-native ternary claim may be made without corresponding evidence.
