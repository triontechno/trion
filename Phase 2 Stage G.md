# TRION Phase 2 — Stage G: Direct Executable Engine

**Status: Planned; not implemented.**  
**Prerequisites:** Stage E runtime/platform integration and Stage F security/validation gates.

## 1. Objective

Build TRION's single application compatibility engine. Users should be able to open supported Windows .exe files and macOS .app bundles directly, as users open applications in Windows and macOS.

TRION loads the original executable format, validates it, decodes machine instructions as execution proceeds, and executes supported instructions in its own controlled execution environment. The source application is not converted into TIR or repackaged as .trx.

## 2. Non-negotiable architecture

- One application execution path: the direct executable engine.
- No application translator.
- No TIR conversion for application launching.
- No TRIONEX/.trx application container.
- No translation cache or generated intermediate executable.
- No fallback to a translator when an instruction or API is unsupported.
- Original .exe files and .app bundles remain unchanged.
- Unsupported inputs fail with actionable compatibility diagnostics.

The old Stage D translator/TRIONEX implementation is retired and must not be revived as a parallel application execution path.

## 3. User-facing contract

- The user selects an .exe or .app in the TRION application launcher or file manager.
- TRION identifies the executable format, architecture, dependencies, and required platform services.
- The engine validates and loads the original image.
- The engine decodes and executes supported machine instructions directly from the loaded image.
- The engine presents compatibility status and explains missing instructions, imports, libraries, or services.
- No translated copy is required or generated.

## 4. Engine architecture

### 4.1 Common application engine

Shared components provide executable identification and validation; guest process address space and memory management; instruction decoding and execution; register, flags, stack, control-flow, exception, and fault semantics; import/dependency resolution interfaces; process lifecycle, resource accounting, and diagnostics; and isolation/permission enforcement.

### 4.2 Windows executable path

Support Windows PE32+ .exe images through a dedicated PE loader and Windows compatibility layer. The loader must validate headers, sections, image base, relocations, imports, entry point, and memory protections. The compatibility layer must implement Windows process/runtime conventions and APIs required by each declared supported application class.

### 4.3 macOS application path

Support macOS .app bundles through bundle inspection, executable selection, a Mach-O loader, and a macOS compatibility layer. The loader must validate bundle metadata, architectures, load commands, segments, relocations, dependencies, and entry point. The compatibility layer must implement runtime conventions and frameworks required by each declared supported application class.

Windows and macOS services remain distinct platform layers over shared engine primitives. A .app bundle is not itself a single executable; TRION must locate and validate its actual Mach-O executable and required bundle resources.

### 4.4 Instruction engine

- Decode instructions from the original loaded guest image at runtime.
- Execute supported source-ISA semantics in a bounded interpreter/emulator.
- Do not emit TIR, .trx, or another intermediate executable.
- Implement instruction semantics in complete, tested groups.
- Model architectural state and memory accurately.
- Handle faults, exceptions, control flow, calls, returns, and system transitions.
- Reject unsupported and privileged instructions safely.
- Publish exact supported ISA/feature coverage.

### 4.5 Platform compatibility

Instruction execution alone is not Windows/macOS compatibility. Each supported application class requires the relevant ABI, process/thread model, system calls, dynamic library loading, filesystem behavior, synchronization, graphics, audio, networking, and framework APIs.

Do not claim that arbitrary Windows or macOS applications run until those services are implemented and tested. Compatibility claims must identify tested application classes and API coverage.

### 4.6 Security and isolation

- Treat executable files, bundles, libraries, and metadata as untrusted.
- Validate every offset, size, address, relocation, mapping, and permission.
- Isolate guest processes from the TRION kernel and other processes.
- Enforce guest memory permissions and prevent unsafe writable/executable mappings.
- Restrict filesystem, network, device, and system-service access through explicit policy.
- Apply CPU, memory, time, and process resource limits.
- Fuzz executable loaders, instruction decoders, dependency parsers, and compatibility boundaries.
- Never execute untrusted guest instructions in kernel mode.

## 5. Implementation milestones

1. Specify the guest architecture, process model, fault model, and compatibility contract.
2. Implement and fuzz the instruction decoder.
3. Implement the execution engine and verify instruction semantics against trusted references.
4. Implement validated PE32+ loading and the Windows guest process foundation.
5. Implement validated Mach-O loading and macOS .app bundle selection.
6. Implement dependency loading and platform ABI/system-service layers.
7. Add graphics, input, audio, filesystem, networking, and other APIs required by selected application classes.
8. Integrate launcher/file-manager dispatch, process isolation, permissions, diagnostics, and resource controls.
9. Publish a tested compatibility matrix and complete end-to-end application tests.

## 6. Completion criteria

- Supported .exe and .app fixtures launch from their original formats.
- No TIR or .trx artifacts are created or required.
- Decoder and execution semantics pass differential and regression tests.
- PE and Mach-O loaders pass malformed-input, relocation, dependency, and memory-permission tests.
- Required Windows/macOS APIs and ABI behavior are documented and tested for every claimed application class.
- Sandboxing, resource controls, fuzzing, and fault handling pass their security gates.
- End-to-end launch tests pass on declared supported TRION configurations.

**This is the only planned application execution engine in TRION.** Compatibility is limited to the tested architecture, instruction, dependency, ABI, and platform-service matrix; it is not an automatic promise of universal Windows/macOS compatibility.
