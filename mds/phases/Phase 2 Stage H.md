# TRION Phase 2 — Stage H: Isolated Guest Process Backend

**Implementation status: partially implemented; not complete.** Stage G's restricted freestanding native images use Redox's real process/address-space interfaces. The kernel owns scheduling, user-mode entry, exception termination, process-manager wait status and teardown. The Stage H-specific guest sandbox and capability broker are still incomplete.

**Validation:** deferred until all Phase 2 implementation stages are complete. No tests, builds, fuzz campaigns, benchmarks or QEMU runs have been executed.

## Implemented source

- Native image entry uses a fresh Redox address space and kernel page mappings; original PE32+/Mach-O64 bytes are mapped with final W^X permissions.
- Native entry and stack registers are installed through the process scheme; the existing Redox scheduler and architecture context switch enter user mode.
- The native exit trampoline submits `ProcCall::Exit` to the process manager. Existing process-manager lifecycle waits for all threads to terminate; kernel teardown closes file handles and drops the address space.
- Before native entry, inherited user and exception signal handlers are now cleared as a required operation. Failure to reset signal state aborts the launch rather than entering guest code with stale handlers.
- Kernel unhandled processor exceptions are retained in `CtxtStsBuf`. The process manager now maps x86-64 exception vectors to POSIX wait signals (SIGFPE, SIGTRAP, SIGSEGV, SIGILL, SIGBUS), instead of reporting the invalid signal number 1.
- The portable guest model provides bounded address-space mappings, checked memory access, W^X enforcement, memory/mapping quotas, process states, instruction budgets, brokered syscall events, breakpoint suspension and fault transitions.

## Remaining implementation

- A kernel-enforced per-process guest policy that mediates or denies direct syscalls; native guest code currently executes with the normal Redox syscall surface.
- Explicit per-process capability allowlists for filesystem, network, device and IPC scheme handles, with a broker that enforces the allowlists.
- Kernel-enforced guest memory/CPU/file-descriptor quotas and accounting; the portable model's memory quota is not a substitute for kernel enforcement.
- Safe cancellation and thread lifecycle policy specific to guest processes, including policy for descendants and newly created threads.
- Structured fault details delivered to the launcher (the wait status now carries the mapped signal, but exception address/code and crash reporting are not yet exposed through a guest-specific report).
- Audited cleanup/recovery for every partial-load and process-failure path.
- Isolation and runtime integration for broader Windows/macOS profiles as Stages I and J implement those services.

## Security boundary

The bounded Rust interpreter model is not a hardware address space. Native freestanding images use actual Redox address spaces and user-mode contexts, but they are **not yet a complete sandbox**. Until the kernel-enforced syscall/capability policy is implemented, do not treat arbitrary PE/Mach-O code as safely isolated or run untrusted applications under this profile.
