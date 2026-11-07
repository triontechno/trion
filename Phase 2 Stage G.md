# TRION Phase 2 — Stage G: Direct Executable Decoding and Execution

**Status: Planned; not implemented.**  
**Prerequisites:** Stage D executable-format foundations and Stage E runtime/platform integration.

## 1. Objective

Allow users to open supported Windows `.exe` and macOS `.app` applications directly in TRION without first translating them into TIR or packaging them as `.trx` files.

This stage adds a direct execution path alongside (not instead of) the existing translator and TRIONEX runtime.

"Direct" means that TRION loads the source executable in its native executable format, decodes machine instructions as execution proceeds, and interprets supported instructions without producing an intermediate translated executable. It does **not** mean executing untrusted host machine code directly on the CPU.

## 2. User-facing contract

- A user can request to open a supported `.exe` or `.app` through the TRION executable launcher.
- The launcher identifies the format and architecture, validates the image, and checks compatibility.
- The direct execution engine decodes and executes supported instructions from the original image.
- The original executable and application bundle remain unchanged.
- No `.trx` conversion, translation cache, or intermediate TIR file is required for this path.
- Unsupported instructions, imports, relocations, architectures, or platform services produce explicit diagnostics; they are never silently approximated.
- The existing `.trx` execution and optional translation/import workflows remain available.

## 3. Architecture

### 3.1 Executable launcher and dispatch

Provide a common open/launch interface that dispatches:
- `.trx` to the existing TRIONEX loader and TIR runtime.
- Windows PE32+ `.exe` to the PE image loader and direct x86-64 execution engine.
- macOS `.app` bundles to bundle inspection, Mach-O selection, the Mach-O loader, and the direct execution engine.

Bundle selection must reject ambiguous or unsupported application layouts rather than guessing.

### 3.2 Format-specific loaders

Implement validated loaders for the declared formats. They must handle image headers, sections/segments, entry points, image-relative addressing, required relocations, permissions, and bounds checks. Loader work must not assume that Stage D's current import-only parser is a complete process loader.

### 3.3 Direct decoder and execution engine

Build a dedicated direct-execution component that:
- Decodes source x86-64 instructions at runtime from the loaded image.
- Executes supported instructions through an interpreter, without emitting TIR or another intermediate executable.
- Models architecturally visible registers, flags, instruction pointer, stack, and memory accesses with checked bounds and defined semantics.
- Handles control flow, calls, returns, and faults.
- Enforces instruction/time/resource budgets and safe termination.
- Rejects unsupported or privileged instructions and unsafe execution modes.
- Has independently tested instruction semantics and decoder validation.

Instruction coverage must be expanded in complete, semantics-preserving groups. Do not claim general x86-64 compatibility until the required ISA coverage and tests exist.

### 3.4 Platform compatibility services

Instruction decoding alone does not provide Windows or macOS application compatibility. Implement explicit compatibility services for the APIs and runtime conventions required by each supported application class.

Windows and macOS compatibility should be separate platform layers over shared loader/runtime primitives. Resolve imports only against implemented, documented services. Dynamic linking, process/thread behaviour, filesystem semantics, graphics, networking, and system calls require explicit contracts and tests before applications depending on them are advertised as supported.

### 3.5 Memory and security

- Treat all executable files and bundles as untrusted.
- Validate all offsets, sizes, arithmetic, mappings, permissions, and relocation targets before use.
- Keep the interpreter and application in a process isolation boundary.
- Enforce executable/readable/writable page permissions and prevent writable-executable mappings unless a narrowly justified policy permits them.
- Restrict filesystem, network, device, and system-service access through explicit permissions.
- Do not execute untrusted native bytes in kernel mode.
- Fuzz executable parsers and instruction decoding; malformed input must not cause unchecked panic, out-of-bounds access, or memory corruption.

## 4. Relationship to translation and ternary execution

Direct execution is an additional compatibility path, not a replacement for the Stage D translator:
- Direct path: source executable → validated image → runtime instruction decoding → direct interpreter.
- Translation path: source executable → translator → TIR → optional TRIONEX packaging → TIR runtime.

The direct interpreter initially executes the source ISA semantics in software. It does not establish native ternary hardware execution. Future adaptive execution may choose a validated direct interpreter, translated TIR, or another backend based on measured capability and cost, but that selection must preserve identical program semantics.

## 5. Implementation milestones

1. Specify the direct engine's execution model, fault model, resource limits, and supported ISA subset.
2. Implement and fuzz a standalone x86-64 decoder with precise instruction-length and operand validation.
3. Implement a tested interpreter for an initial complete instruction subset, including registers, flags, branches, stack, and bounded memory.
4. Implement the PE32+ image loader and direct `.exe` launch path.
5. Implement the thin x86-64 Mach-O loader and `.app` bundle selection/launch path.
6. Add import resolution and the minimum explicitly scoped Windows/macOS compatibility services.
7. Integrate process isolation, permissions, resource controls, diagnostics, and desktop launch dispatch.
8. Add differential tests against trusted reference execution for supported instructions, malformed-input fuzzing, integration tests, and documented compatibility fixtures.

Milestones may be delivered incrementally, but the stage must not be marked complete until the declared support matrix, security gates, and end-to-end launch tests pass.

## 6. Verification and completion criteria

- Decoder tests cover valid encodings, prefixes, operand forms, truncation, invalid encodings, and unsupported instructions.
- Interpreter tests cover register and flag semantics, arithmetic edge cases, memory bounds, branches, calls/returns, faults, and execution budgets.
- PE and Mach-O loaders have malformed-input, relocation, section/segment, permissions, and entry-point tests.
- End-to-end tests open supported `.exe` and `.app` fixtures without producing `.trx` or TIR artifacts.
- Compatibility claims are tied to a published matrix of tested architectures, instructions, imports, and platform services.
- Fuzzing, formatting, linting, CI, and relevant OS integration/boot checks pass.

Stage G does not promise that arbitrary Windows or macOS applications will run. Support is limited to the tested executable, instruction, and platform-API matrix.
