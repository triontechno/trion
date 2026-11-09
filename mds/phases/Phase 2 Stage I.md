# TRION Phase 2 — Stage I: Windows Compatibility Runtime

**Status: compatibility-boundary foundation implemented; the selected minimal PE32+ profile is incomplete.** This is not a general Windows runtime.

## Selected compatibility boundary

Target x86-64 PE32+ Windows console-subsystem images with direct entry and no arguments/environment, and no dependence on TLS, SEH/unwind metadata, a Windows CRT, GUI subsystems or undocumented NT internals. Imports are limited to `ExitProcess`, `GetCurrentProcessId` and `GetCurrentThreadId` from `kernel32.dll` or `kernelbase.dll`; all other imports fail closed. This is a deliberately constrained console-runtime profile, not a claim of general Windows application compatibility.

Entry setup follows the Microsoft x64 register calling convention, supplies the 32-byte home area and expected entry stack alignment, and sets x86-64 `GS` to the guest TEB. `ExitProcess` and the identity imports are narrow runtime services, not evidence that the broader Win32/NT ABI is available.

## Implemented source
- Typed import symbols for names and ordinals.
- Bounded export table with duplicate and null-address rejection.
- Explicit classification of selected process, thread, virtual-memory, file, synchronization, time and console APIs.
- Unsupported and unimplemented APIs return explicit diagnostics. No API is falsely marked operational.
- COSMIC Files and Redox exec dispatch launch the restricted PE32+ profile through the real Redox process/address-space path.
- The PE32+ loader validates bounded image/section layouts, maps the image with W^X protections, prefers the requested image base, and applies `IMAGE_REL_BASED_DIR64` relocations when it must use another base. Unsupported relocation types and malformed relocation blocks are rejected.
- Bounded import descriptors for the selected profile are resolved into executable thunks for `ExitProcess`, `GetCurrentProcessId` and `GetCurrentThreadId`. Unsupported libraries, names, ordinals and malformed tables are rejected; when the PE declares an IAT directory, every resolved slot must be contained in it.
- The import thunks expose the three declared process/thread lifecycle APIs: `ExitProcess` uses authenticated guest termination, and the identity queries return the kernel-owned process/thread IDs through argument-free restricted syscalls.
- The selected direct-entry profile now initializes a minimal x64 PEB, TEB and empty process-parameters block, including the image base, stack bounds, TEB self pointer and PEB pointer in GS. This does not implement the loader module lists, heaps, TLS, argument/environment strings or other NT process services.

## Remaining
- Versioned DLL catalog, dependency search policy, forwarder resolution and a broader operational platform export table. Import resolution is wired only for the three declared APIs in the native loader.
- Full Windows process parameters, PEB loader data/module lists, richer thread/runtime services, TLS, exception/unwind handling.
- Kernel-backed Win32/NT service implementations for a narrowly declared profile.
- Filesystem, synchronization, process/thread, console and timing semantics.
- Integration of the full Windows runtime with the isolated guest process backend.
- Compatibility matrix based on actual post-Phase-2 validation.

No general Windows application compatibility is claimed.
