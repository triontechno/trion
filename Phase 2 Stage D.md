# TRION Phase 2 — Stage D: Translator and Executable Compatibility

**Status:** User-space v1 implemented for a bounded x86-64 instruction and executable-format subset; final CI verification pending.  
**Prerequisites:** Stage C bounded TIR runtime.

## 1. TRION native executable format

TRION's native interpreted executable is TRIONEX v1 (.trx). It is a versioned container around canonical textual TIR v1, validated by the same parser used by the runtime. The container has a 36-byte little-endian header, exact payload length, and CRC-32/ISO-HDLC checksum.

.trx is not a PE .exe, a Mach-O binary, or a kernel-loadable native image. It is the portable TRION runtime format.

Commands:

- trion-translate translate <input> <output.tir> translates a supported source executable into TIR.
- trion-translate package <input.tir> <output.trx> packages validated TIR.
- trion-translate import <input> <output.trx> translates and packages in one step.
- trion-translate run <input.trx|input.exe|input.app> [max-instructions] loads and runs a supported TRIONEX or source executable in the bounded user-space runtime.
- trion-translate inspect <input> inspects the executable format and reports supported/unsupported instruction coverage.

## 2. TIR v1 instruction set

The bounded runtime supports:

- NOP, MOV_IMM64, MOV
- ADD, SUB, CMP
- JMP, JZ, JNZ, JL, JLE, JG, JGE
- CALL, RET

Arithmetic is 64-bit wrapping arithmetic. ADD/SUB/CMP update the zero, sign, and overflow flags used by the listed signed branches. CALL/RET use an interpreter-managed call stack; a top-level RET reads its return address from the caller-provided guest stack region. Branch targets must resolve to an instruction address. Duplicate addresses, malformed records, unsupported operations, and invalid branch targets are rejected before execution. Every run has a strict instruction budget.

The x86-64 translator emits only semantics-preserving forms of these operations: register-to-register MOV/ADD/SUB/CMP, MOV r64 immediate, direct relative branches/calls, NOP, and RET. Unsupported operands, indirect control flow, memory instructions, syscalls, and other instructions reject the complete translation; no partial program is emitted.

## 3. Executable input compatibility

- **ELF64 x86-64:** executable sections are translated. Supported executable-section RELA relocations are R_X86_64_PC32, R_X86_64_PLT32, R_X86_64_64, and image-relative R_X86_64_RELATIVE. Undefined symbols, unsupported relocation kinds, malformed relocation tables, and relocatable ET_REL objects without a final layout are rejected.
- **Windows PE32+ x86-64 (.exe):** executable sections can be imported and run when all decoded instructions are in the supported TIR subset.
- **Apple Mach-O x86-64 (.app):** a bundle executable under Contents/MacOS can be imported and run when it is a thin little-endian x86-64 Mach-O image, has no unsupported section relocations, and uses only the supported instruction subset. Ambiguous bundles are rejected.

This is executable-format import and bounded instruction execution, **not** full Windows or macOS application compatibility. The runtime does not emulate Win32/NT APIs, Cocoa/AppKit, dynamic libraries, system calls, GUI services, process startup ABIs, or arbitrary application memory. ARM64, universal Mach-O, unsupported relocations, external imports, and instructions outside the declared subset are rejected with diagnostics.

## 4. Runtime and loader boundary

The translator's user-space loader accepts TRIONEX directly or translates supported ELF/PE/Mach-O input to TIR before execution. It does not invoke the host OS loader for translated code and does not execute untrusted native bytes directly. The TIR runtime remains bounded and isolated from the kernel.

Kernel/Redox ELF-loader integration, process ABI, syscalls, memory mapping, dynamic linking, drivers, and platform services belong to later OS integration work and are not claimed by Stage D.

## 5. Verification gates

Tests cover instruction semantics, branches, calls/returns, instruction-budget exhaustion, invalid branch targets, duplicate addresses, malformed TIR, executable-format import, and ELF relocation resolution/rejection. CI must pass formatting, translator/runtime/container tests, and Clippy before this stage is marked complete.

No claim is made that arbitrary Windows/macOS applications run on TRION. Compatibility is limited to the explicit subset above.
