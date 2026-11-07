# TRION Phase 2 — Stage A: Adaptive Representation Foundation

**Status:** Regenerated implementation specification  
**Parent architecture:** `TRION-ARCHITECTURE.md`  
**Prerequisite:** Phase 1 ternary core  
**Implementation branch:** `phase2-stage-a-adaptive`

## 1. Purpose

Replace the binary-only assumption in the previous Stage A direction with an adaptive binary/ternary foundation. Stage A makes representation selection explicit and testable before it is wired into privileged kernel paths.

Stage A is a host-side Rust library plus specification and tests. It does not pretend that ordinary x86-64/ARM64 hardware has ternary registers, instructions, or RAM.

## 2. Deliverables

- `ternary/adaptive/`: standalone Rust crate, depending on the existing `trion-ternary` Phase 1 crate.
- A typed model for Binary, BalancedTernary, and Hybrid representation choices.
- Boundary classes for internal computation, conventional hardware, network wire, persistent binary formats, and cryptographic operations.
- Candidate metrics describing support, semantic validation, compatibility, and total comparable cost.
- Checked i128/u128 binary-to-balanced-ternary and balanced-ternary-to-binary bridge helpers built on the Phase 1 core API.
- A deterministic selector:
  - internal workloads choose the lowest-cost supported and validated path;
  - conventional binary boundaries prefer a compatible hybrid path when it is no more costly than the binary path, otherwise binary;
  - network and persistent binary boundaries remain binary-compatible;
  - cryptographic boundaries require explicit validation before a ternary/hybrid candidate can be selected;
  - unsupported or incompatible candidates are never selected.
- Unit tests for every selection rule and failure case.

## 3. Cost model

The selector consumes a caller-supplied total cost in consistent units. The caller is responsible for producing comparable values from target-specific measurements or reviewed estimates. Stage A does not fabricate benchmark data or claim that a candidate is faster.

A candidate is eligible only when it is supported and semantically validated. Boundaries that require interoperability additionally require the candidate to be compatible with that boundary.

## 4. Stage A architecture

```text
Workload + target capabilities + measured candidate costs
                         |
                         v
               Adaptive policy selector
                         |
            +------------+------------+
            |            |            |
          Binary       Ternary       Hybrid
            |            |            |
            +------------+------------+
                         |
                Explicit bridge API
                         |
             Conventional binary device,
             protocol, ABI, or file boundary
```

## 5. Non-goals

Stage A does not implement a general-purpose binary-to-ternary executable translator, ternary instruction execution, a TIR runtime, a new filesystem, a native ternary loader, physical ternary RAM, or production cryptographic algorithms. Application translation is discontinued; direct executable loading and execution are specified in Phase 2 Stage G.

## 6. Verification

Run from the repository root:

```sh
cargo test --manifest-path ternary/core/Cargo.toml
cargo test --manifest-path ternary/adaptive/Cargo.toml
cargo fmt --manifest-path ternary/adaptive/Cargo.toml -- --check
cargo clippy --manifest-path ternary/adaptive/Cargo.toml --all-targets -- -D warnings
```

The Stage A implementation is not considered verified until these commands are executed successfully in CI or on a Rust toolchain. A successful unit-test run does not prove OS bootability or hardware ternary support.

## 7. Exit criteria

Stage A is complete when the policy crate builds, all tests pass, its public contract is documented, and later kernel/compiler/loader/driver integrations can consume the selector without changing existing binary hardware interfaces.
