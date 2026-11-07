# TRION Phase 2 — Stage G: Direct Executable Engine

**Status: implementation partially delivered; direct execution is not implemented.**  
**Prerequisites:** Stage E runtime/platform integration and Stage F security/validation gates.

## 1. Objective

Build TRION's single application compatibility engine. The intended user experience is to select a supported Windows .exe or macOS .app and have TRION load the original executable directly.

The engine must preserve the original executable format. It must not translate applications to TIR, create .trx containers, emit translated machine code, or maintain a translation cache. Native CPU execution is permitted only inside a properly isolated guest process with a correct ABI and platform-service implementation.

## 2. Non-negotiable architecture

- One application execution path: the direct executable engine.
- No application translator, TIR application conversion, TRIONEX/.trx container, translation cache, or generated intermediate executable.
- No translator fallback for unsupported instructions, imports, relocations, architectures, or APIs.
- Original .exe files and .app bundles remain unchanged.
- Parsing or mapping an image as inert bytes is not permission to execute it.
- Unsupported inputs fail with actionable diagnostics.
- Never execute guest code in kernel mode.

## 3. Implemented foundation

The isolated Rust crate at ternary/executable provides:

- Bounded PE32+ x86-64 header, COFF, optional-header, section-table, and data-directory inspection.
- PE section range and overlap validation, import-library enumeration, and inert mapping at the preferred image base.
- Bounded little-endian Mach-O 64 x86-64 header, load-command, segment, dependency, and LC_MAIN entry-point inspection.
- XML Info.plist CFBundleExecutable selection against a caller-supplied bundle member listing.
- Explicit launch assessment that reports missing runtime components and always refuses to claim launchability.
- Dedicated cargo-fuzz targets for PE32+, Mach-O 64, and bundle metadata.

The mapped PE bytes are inert memory. No executable permissions are assigned and no guest instruction is executed.

## 4. Remaining implementation

### 4.1 Common execution engine

Still required:
- Guest process creation, address-space ownership, page mapping and permission enforcement.
- x86-64 instruction decoder with bounded input handling and a documented supported-instruction matrix.
- Native execution boundary, CPU state, exception/fault delivery, syscall transitions, thread lifecycle and resource accounting.
- A secure process isolation policy integrated with the Redox process model.
- Launcher/file-manager dispatch and actionable compatibility reporting.

### 4.2 Windows PE32+ path

Still required:
- Base-relocation application and rebasing.
- Import-by-name/ordinal parsing, dependency search policy, symbol resolution and IAT population.
- TLS callbacks, exception/unwind metadata, process startup, Windows calling conventions and runtime behavior.
- Windows system-call/API compatibility services and selected application-class APIs.
- Correct image permission transitions and W^X enforcement.

### 4.3 macOS .app / Mach-O path

Still required:
- Filesystem bundle traversal, binary Info.plist support, architecture selection and resource handling.
- Mach-O relocation, dyld rebasing/binding, chained fixups, dependency resolution and code-signature policy.
- macOS entry/runtime conventions, process services, frameworks and selected application-class APIs.
- Correct segment mapping, memory permissions and W^X enforcement.

### 4.4 Compatibility boundary

Instruction execution is not Windows/macOS compatibility. Each supported application class requires the relevant process/thread model, ABI, syscalls, dynamic libraries, filesystem semantics, synchronization, graphics, input, audio and networking services. No general Windows or macOS application compatibility is claimed.

## 5. Security and validation

All executable files, bundle metadata and dependencies are untrusted. Every offset, length, address, relocation, mapping and permission must be checked. Resource limits must be enforced before allocation. Writable/executable mappings must be prevented unless a narrowly defined, reviewed policy requires them. Filesystem, network and device access must be brokered through explicit policy.

The following fuzz targets have been added but have NOT been run:
- pe32_plus
- macho64
- app_bundle_plist

All tests, fuzz campaigns, builds, benchmarks and QEMU validation remain deferred until Phase 2 implementation is complete.

## 6. Stage G completion criteria

Stage G is complete only when all of the following exist and are validated:

1. PE32+ and Mach-O loaders support the declared image features, relocations, dependencies and entry-point semantics.
2. A guest process/address-space backend provides isolation, W^X, resource limits and fault containment.
3. A bounded x86-64 decoder and direct native execution boundary are integrated without application translation.
4. Windows and macOS ABI, syscall, runtime and framework layers support explicitly declared application classes.
5. Launcher/file-manager dispatch opens original files and reports exact compatibility blockers.
6. Loader, decoder, dependency and compatibility fuzz campaigns complete with no unresolved security findings.
7. Differential, malformed-input, process-isolation and end-to-end launch tests pass on the declared supported TRION configuration.
8. A compatibility matrix identifies tested architectures, imports, APIs and application classes.

**Current result:** the format-inspection foundation and fuzz-target source are implemented. The process executor, decoder, dynamic linkers and platform compatibility layers are not. Therefore Stage G is not complete, and no .exe/.app launch support is claimed. This is the only planned application execution engine in TRION.
