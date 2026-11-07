# TRION Phase 2 — Stage G: Direct Executable Engine

**Status: foundational implementation expanded; execution and compatibility remain incomplete.**  
**Branch:** main  
**Validation:** deliberately deferred until all Phase 2 implementation stages are complete.

## 1. Objective

Build TRION's single application compatibility engine. The intended user experience is to select a supported Windows .exe or macOS .app and have TRION load the original executable directly.

The engine preserves the original executable format. It must not translate applications to TIR, create .trx containers, emit translated machine code, or maintain a translation cache. Native CPU execution is permitted only inside a properly isolated guest process with a correct ABI and platform-service implementation.

## 2. Non-negotiable architecture

- One application execution path: the direct executable engine.
- No application translator, TIR application conversion, TRIONEX/.trx container, translation cache, or generated intermediate executable.
- No translator fallback for unsupported instructions, imports, relocations, architectures, or APIs.
- Original .exe files and .app bundles remain unchanged.
- Parsing or mapping an image as inert bytes is not permission to execute it.
- Unsupported inputs fail with actionable diagnostics.
- Never execute guest code in kernel mode.

## 3. Implemented source foundations (not yet validated)

### Format inspection
- Bounded PE32+ x86-64 header, COFF, optional-header, section-table, and selected data-directory inspection.
- PE section range and overlap validation, import-library enumeration, and inert mapping at the preferred image base.
- Bounded little-endian Mach-O 64 x86-64 header, load-command, segment, dependency, and LC_MAIN entry-point inspection.
- XML Info.plist CFBundleExecutable selection against broker-enumerated bundle members.

### New shared execution primitives
- `guest.rs`: bounded guest address-space model, page alignment, mapping overlap checks, memory quotas, read/write permission checks, W^X enforcement, and process/CPU-state records. It is a safe model only; it is not wired to the Redox scheduler or hardware execution context.
- `decoder.rs`: conservative bounded x86-64 decoder subset for NOP, RET, INT3, HLT, SYSCALL, register push/pop, MOV reg64 immediate, relative CALL/JMP and near conditional branches. Unknown opcodes fail closed. This is not a complete x86-64 decoder and does not execute instructions.
- `loader.rs`: checked PE32+ DIR64 base relocation application over an inert mapped image; bounded import-by-name/ordinal parsing, export-table resolution and deferred IAT population. IAT writes are applied only after every import resolves. Unsupported relocation types and unresolved imports fail closed. A verified DLL catalog and operational Windows API exports are still required.
- `windows.rs`: import symbol/export table model and explicit API classification. No Windows API shim is advertised as implemented; unresolved imports fail closed.
- `macos.rs`: broker-fed .app member path validation, bounded bundle planning, and explicit unresolved-framework decisions. It does not traverse host filesystems or implement dyld.

### Fuzzing source
- Fuzz targets exist for PE32+, Mach-O 64, app bundle metadata, packed ternary memory, TRN3 deserialization, and the x86-64 decoder.
- Fuzz campaigns have not been run.

## 4. Remaining implementation

### 4.1 Common execution engine
- Complete x86-64 decoder and instruction semantics for the declared compatibility profile.
- Architecture-specific native execution boundary and context switching integrated with Redox process/thread lifecycle.
- Syscall/fault transitions, signal delivery, thread lifecycle, resource accounting and process teardown.
- Enforced isolation and brokered filesystem/network/device access.
- Loader-to-address-space mapping with page permissions and entry-point validation.
- Launcher/file-manager dispatch and compatibility diagnostics.

### 4.2 Windows PE32+ path
- Versioned DLL catalog, dependency search policy, forwarder resolution and operational platform export table. The bounded import parser/resolver and IAT population primitive now exist.
- TLS callbacks, exception/unwind metadata, process startup and Windows calling conventions.
- Kernel-backed Win32/NT service implementations for a documented application profile.
- Filesystem semantics, synchronization, console, timing and process/thread services.
- Correct image permission transitions and W^X enforcement.

### 4.3 macOS .app / Mach-O path
- Binary Info.plist support, architecture selection and safe resource handling.
- Mach-O relocation, dyld rebasing/binding, chained fixups and dependency resolution.
- Explicit code-signature and entitlement policy.
- macOS entry/runtime conventions, process services and selected framework implementations.
- Correct segment mapping, memory permissions and W^X enforcement.

## 5. Security and validation

All executable files, bundle metadata and dependencies are untrusted. Every offset, length, address, relocation, mapping and permission must be checked. Resource limits must be enforced before allocation. Writable/executable mappings are prohibited. Filesystem, network and device access must be brokered through explicit policy.

No test, build, fuzz campaign, benchmark, or QEMU validation has been run for the new implementation. They remain deferred until all Phase 2 implementation stages are complete.

## 6. Completion criteria

Stage G is complete only when:
1. PE32+ and Mach-O loaders support the declared image features, relocations, dependencies and entry-point semantics.
2. A guest process/address-space backend provides isolation, W^X, resource limits and fault containment and is integrated with the OS process model.
3. A complete bounded x86-64 decoder and direct native execution boundary are integrated without application translation.
4. Windows and macOS ABI, syscall, runtime and framework layers support explicitly declared application classes.
5. Launcher/file-manager dispatch opens original files and reports exact compatibility blockers.
6. Loader, decoder, dependency and compatibility fuzz campaigns complete with no unresolved security findings.
7. Differential, malformed-input, process-isolation and end-to-end launch tests pass on the declared supported TRION configuration.
8. A compatibility matrix identifies tested architectures, imports, APIs and application classes.

**Current result:** useful source foundations now exist for address-space policy, a conservative decoder subset, PE DIR64 relocation, Windows import modeling and macOS bundle planning. These are not a functioning executable runtime. No .exe/.app launch support is claimed.
