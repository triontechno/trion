# TRION direct executable engine — Stage G foundation

This crate inspects original PE32+ x86-64 and little-endian Mach-O 64 x86-64 images. It does not translate applications, emit TIR, create .trx files, or generate intermediate executables. Native launch support is implemented separately in Redox `redox-rt`; its current Windows and macOS boundaries are documented in [Stage I](../../phases/Phase%202%20Stage%20I.md) and [Stage J](../../phases/Phase%202%20Stage%20J.md).

Implemented:
- Bounded PE32+ header, section and data-directory parsing; section overlap/range validation; import-library enumeration; inert mapping at the preferred base.
- Bounded Mach-O 64 header, load-command, segment, dependency and LC_MAIN validation.
- Safe XML Info.plist CFBundleExecutable selection against a caller-supplied bundle member listing.
- Explicit launch assessment that refuses to claim execution while required OS services are absent.

Not implemented by this metadata crate:
- Relocation application and runtime import/IAT resolution. The separate Windows runtime supports bounded DIR64 rebasing and only `ExitProcess`, `GetCurrentProcessId` and `GetCurrentThreadId` imports from `kernel32.dll` or `kernelbase.dll`.
- Mach-O dyld rebasing/binding, chained fixups, code-signature enforcement and dependency loading.
- Binary plist parsing and general filesystem semantics for `.app` bundles. The current launcher accepts bounded XML metadata.
- Full Windows/macOS ABIs, operating-system services, frameworks, graphics, audio, input or networking APIs. See the stage specifications for the limited launch profiles and remaining work.
- Linux ELF/Linux-ABI compatibility. The native ELF/Redox process path does not implement the Linux ABI.
- Foreign-ISA emulation. Only x86-64 format metadata is accepted.

This crate's mapping plans are inert metadata, not executable memory. Parsing alone is not an application runtime. All tests, fuzz campaigns, builds, benchmarks and QEMU validation remain deferred until Phase 2 implementation is complete.
