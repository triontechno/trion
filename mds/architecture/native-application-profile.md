# TRION Version 1 Native Application Profile

**Status: implementation profile; runtime validation deferred.**

This document defines the native executable and runtime contract for TRION Version 1. It describes the Redox-native ABI used by the system. It does not define or claim Linux, Windows, or macOS application compatibility.

## Executable format

The native loader accepts little-endian ELF images matching the running kernel's architecture and pointer width:

| Architecture | ELF class | ELF machine |
|---|---:|---:|
| x86 | ELF32 | EM_386 (3) |
| x86-64 | ELF64 | EM_X86_64 (62) |
| AArch64 | ELF64 | EM_AARCH64 (183) |
| RISC-V 64 | ELF64 | EM_RISCV (243) |

Supported image types are `ET_EXEC` and `ET_DYN`. Either may request a `PT_INTERP` handoff; the interpreter image itself must be `ET_DYN`. The loader requires the ELF header and program-header dimensions to match the native ABI.

The loader rejects malformed or unsupported images, including:

- Program-header tables outside the backing file or larger than the loader's 1 MiB header-table limit.
- Load segments with zero memory size, file size larger than memory size, invalid alignment/congruence, or file ranges outside the image.
- Unknown segment permission bits and writable-plus-executable load segments.
- Load segments whose mapped page ranges overlap. Shared-page segment layouts are not yet supported.
- Images whose entry point is not inside an executable load segment.
- Invalid, duplicate, misplaced, oversized, non-absolute, or non-NUL-terminated `PT_INTERP` paths.

The ELF loader maps segments with W^X permissions and uses checked address arithmetic. The interpreter must be reachable through the native scheme-rooted filesystem namespace.

## Dynamic linking and TLS

The native C runtime is relibc. The dynamic linker is installed as `/lib/ld.so.1`; the shared C runtime is `/lib/libc.so.6`. The linker and library search paths are configured by the native system image and process environment.

The initial process vector supplies `AT_PHDR`, `AT_PHENT`, `AT_PHNUM`, `AT_ENTRY`, and, during interpreter handoff, `AT_BASE`. The runtime resolves PIE load bias from `PT_PHDR` and `AT_PHDR` when initializing static TLS. `PT_TLS` templates are checked for size, alignment and address overflow before copying into the TCB.

The runtime supports the architecture-specific thread pointer and TLS activation paths implemented by relibc/redox-rt. Dynamic TLS masters are tracked in the TCB and copied with bounds checks. Executables that need initial TLS but do not provide a valid mapped program-header description are rejected during runtime initialization.

## Initial process state

The loader constructs the initial stack with:

- `argc`, a NULL-terminated `argv`, a NULL-terminated `envp`, and an auxiliary vector terminated by `AT_NULL`.
- A 16-byte-aligned initial stack pointer and a bounded stack region.
- Bounded argument, environment and working-directory strings. The aggregate startup string payload is limited to 2 MiB and the argument/environment count is limited to 65,536 entries.
- Native auxiliary entries for executable/interpreter metadata, page size, real/effective user and group IDs, secure-execution state, executable path, inherited signal masks, umask, working directory and process/thread/namespace/filetable descriptors.
- An optional `AT_RANDOM` pointer to 16 bytes read from `/scheme/rand`; it is omitted if the entropy scheme is not available during early boot.

The argument and environment byte strings must not contain embedded NUL bytes. Oversized or malformed startup data returns an execution error instead of overflowing the stack or allocation arithmetic.

## Process, thread and signal runtime

Process creation and image replacement use the native Redox process/thread descriptors and address-space interfaces. A successful in-place exec stages the new image and startup state before switching address spaces. Close-on-exec handling and TLS deactivation are performed on the final transition path; process naming and process-group restrictions are applied only after image construction succeeds.

Native thread creation uses the relibc pthread layer and redox-rt clone/thread descriptors. Each thread receives a TCB, architecture-specific TLS activation, copied static TLS masters, and signal-control state. Thread exit is routed through the native thread status interface. Signal entry, architecture register save/restore, signal masks and return are implemented by redox-rt's architecture-specific signal runtime.

## Native system interfaces

Native applications use the Redox syscall ABI and scheme interfaces. These are not Linux syscall numbers or Linux kernel semantics.

| Area | Native interface |
|---|---|
| Filesystem and namespaces | scheme-backed file descriptors, namespace descriptors, open/read/write/stat and directory operations |
| Process and threads | process/thread descriptors, fork/exec, wait, pthread clone and thread exit |
| Signals | native signal-control descriptors, relibc signal API and redox-rt signal trampoline |
| Memory | address-space descriptors, mmap/fmap, mprotect and munmap |
| IPC | scheme sockets, pipes and descriptor-based IPC |
| Time | time scheme and native clock/timer interfaces |
| Devices | device schemes and explicitly granted device descriptors |

Applications must use the native runtime and services shipped in the TRION system image. Ordinary native ELF launches use native Redox namespace and descriptor authority; they do not automatically activate the restricted Stage H guest policy. The Stage H guest backend remains a separate, supervisor-authorized profile with bounded pre-opened descriptor grants. Neither profile provides arbitrary hardware access outside the existing device and scheme interfaces.

## Explicitly unsupported

- Linux ELF execution as a Linux ABI, including Linux syscall compatibility.
- Windows PE application compatibility and Win32/NT services.
- macOS Mach-O application compatibility and Darwin services.
- ELF load layouts that require overlapping PT_LOAD page mappings.
- Arbitrary foreign ABI interpreters, foreign shared-library formats, or platform API translation.
- Scheduler policy interfaces not implemented by the current native runtime.
- AArch64 SIMD and RISC-V vector register state is not yet preserved by the signal context implementation; applications that require those register sets to survive signal delivery are outside the supported runtime profile.

These exclusions are deliberate Version 1 boundaries, not claims that the corresponding compatibility layers exist.
