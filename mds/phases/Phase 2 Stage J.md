# TRION Phase 2 — Stage J: Native System Integration and Completion

**Status: implementation complete for the declared Version 1 native system scope; validation deferred.**

Stage J integrates the Version 1 native executable path, initfs boot graph, service manager, native Redox ABI, hardware/device boundaries, Stage H guest security and process lifecycle. It does not add a foreign operating-system compatibility layer.

## Implemented

### 1. Unified executable dispatch

The authoritative process-image entry, src/relibc/redox-rt/src/proc.rs::fexec_impl, classifies the original image signature once before entering a format-specific loader:

- ELF selects the native TRION/Redox loader.
- MZ selects the restricted x86-64 freestanding PE32+ loader.
- Little-endian Mach-O64 selects the restricted x86-64 freestanding Mach-O loader.
- Unknown signatures return ENOEXEC before format-specific parsing.
- Non-x86-64 targets accept only native ELF.

No translation, extraction, TIR fallback or generated executable is introduced. The PE32+/Mach-O64 entry profiles remain freestanding and do not provide Win32/NT or Darwin/dyld APIs.

### 2. Initfs and boot integration

- src/base/Makefile includes trion-capd and trion-cap-check in the initfs binary set and copies them into the generated initfs.
- 40_drivers.target orders the capability daemon/checker with the hardware manager and initfs PCI spawner.
- 50_rootfs.service follows the drivers target using the existing weak-dependency model.
- Weak dependencies preserve boot when optional hardware discovery is unavailable; the capability checker reports integration failures without making rootfs mounting depend on optional devices.

### 3. Bounded service startup and runtime supervision

The init service manager now supports opt-in bounded restart policy:

- restart_on_failure, max_restarts and restart_delay_ms are parsed from service configuration.
- Retry count is capped at 10; delay is capped at 30 seconds.
- Startup failures before readiness are retried within the configured budget.
- Long-running services opted into restart policy are registered by PID with the scheduler.
- PID 1's single central waitpid loop reports reaped child PIDs/statuses to the scheduler. Failed supervised services are restarted within a finite budget; successful exits are not restarted.
- Child reaping remains centralized; service restart logic does not introduce competing Child::wait callers.
- Scheme readiness now propagates namespace lookup and scheme-registration failures rather than panicking through expect.

The hardware manager and TRION capability daemon opt into three bounded restarts with short delays. Other services retain the default no-restart behavior unless explicitly configured.

### 4. Native kernel/runtime ABI integration

The native ABI remains the Redox syscall and scheme ABI. The integrated profile covers:

| Area | Integrated boundary |
|---|---|
| Executable/process | ELF loader, interpreter handoff, exec transition, process/thread descriptors |
| Memory | Address-space descriptors, grants, mmap/fmap, protection and unmap operations |
| Threads/TLS | relibc pthread and redox-rt architecture-specific TCB/TLS activation |
| Signals | Native signal-control descriptors, signal trampoline and return paths |
| Filesystem/namespaces | Scheme-backed descriptors and namespace authority |
| IPC | Scheme sockets, pipes and descriptor-mediated IPC |
| Time | Native time scheme and timer interfaces |
| Devices | Redox device schemes and mediated descriptors |

Supported native ELF architecture profiles are x86, x86-64, AArch64 and RISC-V64 as documented in Native Application Profile. No syscall numbers or kernel ABI layouts are changed by Stage J. No Linux syscall translation is added.

### 5. Hardware and ternary boundaries

Hardware services continue to use Redox device and scheme interfaces. The capability service reports platform facts and adaptive policy; it does not grant applications direct MMIO or DMA authority. MMIO layouts, DMA descriptors, driver ABIs, syscall structures, network frames and persistent formats remain binary. Packed ternary memory remains a logical representation backed by conventional binary memory.

### 6. Security and resource lifecycle

- Ordinary native ELF launches retain native Redox namespace and descriptor authority.
- Stage H guest policy is activated only by an explicit trusted-supervisor guest launch.
- Guest syscall filtering, delegated descriptor rights, memory/descriptor/CPU quotas, cancellation, crash reporting and revocation remain kernel-enforced.
- Native process/thread lifetime, address-space ownership and scheme descriptors remain kernel-managed.
- PID 1 continuously reaps children, tolerates interrupted/transient wait errors without panicking, and routes supervised service failures to the bounded restart scheduler.
- Failed image preparation returns before the new image becomes active; failed service readiness kills and reaps the unsuccessful child before retry.

Ordinary native processes continue under the existing Redox kernel resource policy. Stage J does not invent a system-wide quota or claim that Stage H's guest-only limits apply to all native processes.

## Explicit exclusions

- Windows, macOS or Linux application compatibility.
- Win32/NT, Darwin, dyld or Linux syscall emulation.
- TIR or translator fallback for application execution.
- Direct application MMIO/DMA access.
- Physical ternary CPU, RAM or peripheral assumptions.
- Unbounded automatic service restart.

## Validation gate

**Validation remains deferred by project instruction.** No tests, builds, benchmarks, fuzzing or QEMU runs have been performed for Stage J. After all Phase 2 implementation stages are frozen, the consolidated validation pass must verify:

- Native ELF, interpreter and exec behavior on supported architectures.
- PE32+/Mach-O64 restricted-profile behavior and rejection of unsupported formats.
- Initfs packaging and boot/service dependency ordering.
- Startup retry limits, runtime restart budgets, readiness failure cleanup and PID 1 child reaping.
- Native process/thread/TLS/signal, descriptor, namespace and address-space lifecycle.
- Stage H syscall isolation, capability revocation, quotas, cancellation and crash reporting.
- Full supported system-image build and QEMU boot.

Implementation completion does not imply compilation, runtime or hardware validation.
