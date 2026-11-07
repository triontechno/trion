# TRION Phase 2 — Stage I: Windows Compatibility Runtime

**Status: compatibility-boundary foundation implemented; runtime services not implemented.**

## Implemented source
- Typed import symbols for names and ordinals.
- Bounded export table with duplicate and null-address rejection.
- Explicit classification of selected process, thread, virtual-memory, file, synchronization, time and console APIs.
- Unsupported and unimplemented APIs return explicit diagnostics. No API is falsely marked operational.

## Remaining
- PE import-by-name/ordinal parsing and IAT population.
- Versioned dependency catalog and DLL search policy.
- Windows process startup, x64 calling convention, TLS, exception/unwind handling.
- Kernel-backed Win32/NT service implementations for a narrowly declared profile.
- Filesystem, synchronization, process/thread, console and timing semantics.
- Integration with the isolated guest process backend and launcher.
- Compatibility matrix based on actual post-Phase-2 validation.

No general Windows application compatibility is claimed.
