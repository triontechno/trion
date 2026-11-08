# TRION Phase 2 — Stage I: Windows Compatibility Runtime

**Status: compatibility-boundary foundation implemented; general Windows runtime services remain incomplete.** The Stage G native launcher accepts only the freestanding PE32+ profile.

## Implemented source
- Typed import symbols for names and ordinals.
- Bounded export table with duplicate and null-address rejection.
- Explicit classification of selected process, thread, virtual-memory, file, synchronization, time and console APIs.
- Unsupported and unimplemented APIs return explicit diagnostics. No API is falsely marked operational.
- COSMIC Files and Redox exec dispatch can launch the restricted no-import PE32+ profile through the real Redox process/address-space path.

## Remaining
- Versioned DLL catalog, dependency search policy, forwarder resolution and operational platform export table. The bounded PE import-by-name/ordinal parser and IAT population primitive now exist.
- Windows process parameters, PEB/TEB, x64 startup ABI, TLS, exception/unwind handling.
- Kernel-backed Win32/NT service implementations for a narrowly declared profile.
- Filesystem, synchronization, process/thread, console and timing semantics.
- Integration of the full Windows runtime with the isolated guest process backend.
- Compatibility matrix based on actual post-Phase-2 validation.

No general Windows application compatibility is claimed.
