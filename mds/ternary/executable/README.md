# TRION direct executable engine — Stage G foundation

This crate inspects original PE32+ x86-64 and little-endian Mach-O 64 x86-64 images. It does not translate applications, emit TIR, create .trx files, or generate intermediate executables.

Implemented:
- Bounded PE32+ header, section and data-directory parsing; section overlap/range validation; import-library enumeration; inert mapping at the preferred base.
- Bounded Mach-O 64 header, load-command, segment, dependency and LC_MAIN validation.
- Safe XML Info.plist CFBundleExecutable selection against a caller-supplied bundle member listing.
- Explicit launch assessment that refuses to claim execution while required OS services are absent.

Not implemented:
- PE relocation application and symbol-level import/IAT resolution.
- Mach-O dyld rebasing, binding, chained fixups, code-signature enforcement and dependency loading.
- Binary plist parsing and filesystem traversal of .app bundles.
- Guest process/address-space isolation, instruction decoding/execution, fault handling, dynamic linking, Windows/macOS syscalls, runtimes, frameworks, graphics, audio, input or networking APIs.
- Foreign-ISA emulation. Only x86-64 format metadata is accepted.

The mapped PE image is inert data, not executable memory. A parser is not an application runtime; no .exe or .app launch is currently enabled. All tests, fuzz campaigns, builds, benchmarks and QEMU validation remain deferred until Phase 2 implementation is complete.
