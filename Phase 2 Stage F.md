# TRION Phase 2 — Stage F: Measurement, Hardening, and Release Validation

**Implementation status:** Initial benchmark harness and deterministic parser robustness regression tests merged into `main`; sustained fuzzing, comparative measurements, and boot evidence remain required.

## Required evidence

- Measure binary, balanced-ternary, and hybrid implementations on equivalent workloads.
- Include encoding/decoding, conversion, allocation, metadata, and compatibility overhead.
- Record host CPU, compiler/toolchain, workload, input sizes, repetitions, and raw measurements.
- Fuzz packed-memory decoding and, when Stage G is implemented, direct PE/Mach-O loaders and instruction decoding.
- Run unit tests, formatting, Clippy, and architecture-specific build checks.
- Build the supported OS images and boot each declared QEMU configuration.
- Publish results without claiming speed, density, energy, or security gains unless the measurements support them.

## CI regression gates

The adaptive workflow runs the Stage A/B policy and packed-memory tests, the Stage C legacy TIR runtime tests and Stage E capability-report tests; Stage G direct-engine tests will be added when implemented. It runs rustfmt and Clippy with warnings denied.

These deterministic tests are regression coverage, not a substitute for a sustained fuzzing campaign or performance benchmark suite.

## Release gate

Phase 2 cannot be declared complete until all Stage A–F criteria are met, the direct engine's supported executable, instruction, and platform-service subset is accurately documented, malformed inputs are rejected safely, measured comparisons include overhead, and the supported OS image boots in the declared QEMU configurations. A green library CI workflow alone is insufficient.
