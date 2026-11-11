# TRION Phase 2 — Stage G: Direct Executable Engine

**Implementation status: complete for the declared freestanding x86-64 profile.**  
**Validation:** deferred until all Phase 2 implementation stages are complete.  
**Branch:** main

## 1. Objective

Provide one direct execution path for original Windows PE32+ .exe files and macOS Mach-O64 .app executables. Stage G establishes the executable engine and native process hand-off. It does not claim general Windows or macOS application compatibility; those platform runtimes belong to Stages I and J.

## 2. Non-negotiable architecture

- Original executable bytes are loaded directly; no application translation, TIR conversion, .trx, generated intermediate executable, or translation cache.
- Guest code never runs in kernel mode.
- Native execution uses a real Redox process address space and user-mode context.
- Writable/executable mappings are prohibited.
- Unsupported imports, runtime features, rebase requirements, ABI requirements, and architectures fail closed.
- Original .exe files and .app bundles remain unchanged.

## 3. Implemented source

### Format and metadata handling
- Bounded PE32+ x86-64 header, COFF, optional-header, section-table, and selected data-directory parsing.
- PE section range/overlap validation, import-library enumeration, inert mapping, DIR64 relocation application, import-by-name/ordinal parsing, export-table resolution, and deferred IAT population in the research loader.
- Bounded Mach-O64 x86-64 header, load-command, segment, dependency, and LC_MAIN inspection.
- XML Info.plist CFBundleExecutable selection with brokered member validation.

### Native Redox execution
- src/relibc/redox-rt/src/proc.rs dispatches original PE32+ and little-endian Mach-O64 images into proc/trion.rs from the standard process execution path.
- src/relibc/redox-rt/src/proc/trion.rs creates a fresh address space through Redox's existing process/address-space scheme, maps original image segments, enforces W^X, creates the user stack, sets the initial instruction/stack registers through current-addrspace, and hands execution to the normal Redox scheduler and x86-64 context-switch path.
- The PE function-entry profile's OS-owned return trampoline invokes ProcCall::Exit through the process-manager scheme and retries interrupted/temporarily unavailable exit requests. It is not translated application code. The restricted Mach-O profile uses an OS-owned x86-64 adapter to build the Darwin-style initial stack, call LC_MAIN, and report its return status through authenticated guest exit.
- Native PE32+ profile: x86-64 freestanding images with no imports/IAT, TLS callbacks, delay imports, exception/unwind directory, load-config/CFG metadata, managed-runtime directory, argv or environment block; loaded at the preferred image base. Rebase-required images fail closed. Windows PEB/TEB, Win32/NT APIs, dynamic linking and unwind services are not provided.
- Native Mach-O64 profile: freestanding MH_EXECUTE images with LC_MAIN, no dyld, dylib dependencies, chained fixups or code signature, with only argv[0] and an empty environment; mapped at their declared segment addresses. The initial stack has minimal argc/argv0 and empty envp/Apple vectors; an OS-owned adapter calls LC_MAIN using the x86-64 C calling convention and returns through authenticated guest exit. macOS dyld, CRT and framework services are not provided.
- The process runs in user mode under Redox's actual address-space and scheduler machinery. Unhandled processor faults reach the existing kernel exception path; Stage H still owns the explicit guest fault, signal, capability and containment policy.

### Instruction semantics
- The declared TRION-X64-INTERPRETED-CORE-0 subset has bounded decoding and execution for NOP, RET, INT3, SYSCALL, HLT, PUSH/POP r64 (including REX-extended registers), MOV r64 immediate, CALL/JMP rel32, and near Jcc.
- Implemented semantics include stack/register effects, branch conditions, SYSCALL RCX/R11 and flag-mask transitions, entry-return handling, breakpoint suspension, and user-mode HLT/decode/memory fault states.
- This is a deliberately small interpreter profile, not a claim of complete x86-64 emulation. Native PE/Mach-O execution uses the CPU directly.

### OS launcher
- Redox relibc dispatches original PE32+ and Mach-O64 files through the standard execve/posix_spawn path, so any OS component using the normal process API reaches the native loader.
- COSMIC Files routes .exe files into that process path.
- COSMIC Files resolves .app/Contents/Info.plist, rejects symlink/special-file bundle members, selects the declared Contents/MacOS executable, and launches that original Mach-O file through the same process path.
- The current bundle resolver supports XML Info.plist; binary plist and general macOS bundle semantics remain unsupported.

### Fuzzing and tests
- Fuzz targets exist for PE32+, Mach-O64, app bundle metadata, packed ternary memory, TRN3 deserialization, and the x86-64 decoder.
- Decoder and guest semantic regression tests are authored.
- No tests, builds, fuzz campaigns, benchmarks, or QEMU validation have been run.

## 4. Work assigned to later stages

- Stage H: explicit guest fault/signal policy, capability isolation, resource accounting, brokered filesystem/network/device access, thread lifecycle, and containment.
- Stage I: Windows process parameters, PEB/TEB, DLL catalog/linking, TLS, unwind/exception handling, Win32/NT services, and documented Windows application classes.
- Stage J: binary plist, dyld, Mach-O rebasing/binding/chained fixups, code-signature policy, frameworks, macOS ABI/startup and application services.

These are not prerequisites for the freestanding Stage G profile; they remain required for broader application compatibility.

## 5. Completion criteria

Stage G implementation is complete when:
1. The declared freestanding PE32+ and Mach-O64 profiles load original image bytes into a real Redox user process.
2. Address-space permissions, W^X, entry/stack setup, and process exit are connected to the existing Redox process lifecycle.
3. Every instruction in the declared interpreter subset has defined, bounded semantics and unsupported instructions fail closed.
4. COSMIC Files routes .exe and supported .app bundles into the direct execution path.
5. Regression and fuzz test sources cover the pure format parsers, decoder, guest semantics, and bundle metadata boundaries. Native kernel process-path validation is reserved for the post-Phase-2 validation pass.

**Current result:** the Stage G implementation criteria are present in source on main. Runtime validation remains deliberately deferred. This does not claim general Windows or macOS application compatibility.
