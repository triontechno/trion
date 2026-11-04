# TRION Phase 2 — Stage C: TIR Execution Foundation

**Status:** Implemented and CI-verified in main  
**Prerequisites:** Stage A adaptive policy and Stage B checked packed memory  
**Current input:** Canonical textual TRION-TIR v1

## 1. Purpose

Execute the exact small instruction subset emitted by the current translator in a bounded software interpreter. This makes TIR execution a separately testable capability without claiming that TIR is a native executable or that the OS can yet load and run translated applications.

## 2. Supported operations

- `NOP`: no architectural state change.
- `MOV_IMM64 rN value`: set one of the 16 modeled x86-64 general-purpose registers to a nonnegative 64-bit immediate encoded canonically in balanced ternary.
- `RET`: read an eight-byte little-endian return address from the caller-provided memory at the modeled RSP address, then increment RSP by eight.

Execution is linear. TIR v1 has no branches, calls, memory writes, arithmetic, flags, or system calls. Any other operation is rejected.

## 3. Safety contract

- Validate UTF-8, header/version, final newline, record shape, addresses, register IDs, and canonical ternary immediates before execution.
- Bound TIR input size and instruction count.
- Require the caller to provide a memory base and immutable memory slice.
- Check RSP subtraction, address conversion, range end, memory slice bounds, and RSP increment.
- Enforce a caller-supplied maximum number of executed instructions.
- Execute against a local register copy and commit it only on successful completion or RET; errors leave caller registers unchanged.
- Do not execute native machine code or enter kernel mode.

## 4. API

The `trion-tir-runtime` crate provides `MachineState`, `ExecutionOutcome`, `RuntimeError`, and `execute_tir(bytes, state, max_instructions)`.

## 5. Verification

Tests cover all supported operations, canonical immediate parsing, malformed input, unsupported instructions, stack bounds, RSP overflow, instruction limits, and no-partial-state-mutation on failure.

## 6. Non-goals

Stage C does not add translator instructions, control flow, a JIT, a native ternary CPU backend, a binary executable format, a system-call ABI, or kernel execution. These require separate design and semantic tests in Stages D and E.
