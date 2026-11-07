# Phase 2 fuzzing harness

This is the Stage F fuzz-target source. It is intentionally not executed during Phase 2 implementation; fuzz campaigns run in the consolidated post-Phase-2 validation pass.

## Targets

- `packed_trit_buffer`: adversarial logical lengths, malformed packed bytes, reserved encodings, padding, and canonical decode/re-encode invariants. Input length is capped at one million trits to bound fuzz-worker allocations.
- `trn3_deserialize`: arbitrary TRN3 byte streams, malformed headers/counts/payloads, and canonical deserialize/serialize round trips.

## Run after Phase 2 implementation

Install `cargo-fuzz` with the nightly Rust toolchain, then from `ternary/fuzz`:

```sh
cargo fuzz run packed_trit_buffer -- -max_len=1048584
cargo fuzz run trn3_deserialize -- -max_len=1048576
```

Preserve corpus, crash artifacts, toolchain versions, duration, and coverage output as release evidence. Stage G must add dedicated PE32+ and Mach-O loader/instruction-decoder targets when those parsers exist.
