# TRION Phase 2 — Stage H: Isolated Guest Process Backend

**Implementation status: partially implemented; not complete.** Stage G's restricted freestanding native images use Redox's real process/address-space interfaces. The kernel owns scheduling, user-mode entry, exception termination, process-manager wait status and teardown. The Stage H-specific guest sandbox and capability broker are still incomplete.

**Validation:** deferred until all Phase 2 implementation stages are complete. No tests, builds, fuzz campaigns, benchmarks or QEMU runs have been executed.

## Implemented source

- Native image entry uses a fresh Redox address space and kernel page mappings; original PE32+/Mach-O64 bytes are mapped with final W^X permissions.
- Native entry and stack registers are installed through the process scheme; the existing Redox scheduler and architecture context switch enter user mode.
- The native exit trampoline submits a dedicated `SYS_GUEST_EXIT` syscall carrying an 8-bit process status. The kernel records that status and transitions the restricted context to termination; existing process-manager lifecycle waits for all threads to terminate and kernel teardown releases file handles and address-space grants.
- Before native entry, inherited user and exception signal handlers are now cleared as a required operation. Failure to reset signal state aborts the launch rather than entering guest code with stale handlers.
- Kernel unhandled processor exceptions are retained in `CtxtStsBuf`. The process manager now maps x86-64 exception vectors to POSIX wait signals (SIGFPE, SIGTRAP, SIGSEGV, SIGILL, SIGBUS), instead of reporting the invalid signal number 1.
- The portable guest model provides bounded address-space mappings, checked memory access, W^X enforcement, memory/mapping quotas, process states, instruction budgets, brokered syscall events, breakpoint suspension and fault transitions.
- **Kernel-enforced syscall isolation and guest exit are implemented as a restricted baseline:** the native loader prepares policy for one address space, and the kernel activates it atomically when the pending address-space switch is committed. An uncommitted preflight is canceled when its control descriptor closes; after selection is staged, the pending switch retains the activation intent. The common dispatcher permits only `SYS_CLOSE` and a dedicated `SYS_GUEST_EXIT` carrying an 8-bit process status; every other syscall is rejected before scheme dispatch. Guest exit cannot address an inherited descriptor or select a scheme operation. Policy activation is one-shot.
- Separate-context native launches clear the inherited target file table before policy activation. In-place native exec still retains non-close-on-exec descriptors, but the deny-all syscall gate makes them inaccessible other than through close; this is not yet a capability allowlist.
- Kernel quotas are applied when the guest policy is activated: the authenticated guest address space is bound to the context and limited to 512 MiB of mapped grants (including fixed replacement and address-space move paths), the descriptor table is limited to 256 active descriptors/indices, and scheduled guest CPU time is limited to 60 seconds from policy activation. Address-space changes after activation are rejected unless they select the same bound space. These are conservative defaults for the current freestanding profile, not per-user configuration.
- Guest exit status is retained in the kernel context status report and forwarded to the process manager as a normal exit code. Existing force-kill and processor-exception reporting remains distinct.
- This narrow exit exception does not provide a general syscall broker or make the guest runtime usable for ordinary I/O.

## Remaining implementation

- Explicit per-process capability allowlists for filesystem, network, device and IPC scheme handles, with a broker that enforces the allowlists.
- Safe cancellation and thread lifecycle policy specific to guest processes, including policy for descendants and newly created threads.
- A guest-specific crash-report API is not exposed. Unhandled processor exceptions are retained in the kernel status record and the process manager logs the exception vector, code and instruction address while translating the wait signal.
- Audited cleanup/recovery for every partial-load and process-failure path.
- Isolation and runtime integration for broader Windows/macOS profiles as Stages I and J implement those services.
- Complete capability-based descriptor inheritance and a syscall broker/allowlist. Do not loosen the deny-all policy until the broker validates the target capability and operation.

## Security boundary

The bounded Rust interpreter model is not a hardware address space. Native freestanding images use actual Redox address spaces and user-mode contexts. The deny-all syscall baseline, isolated exit path, and initial kernel resource quotas are enforced, but the backend does not yet provide a functional filesystem/network/device/IPC capability broker, complete descriptor inheritance policy, or comprehensive cleanup/recovery audit. Do not treat Stage H as complete or claim general-purpose safe execution of arbitrary untrusted PE/Mach-O applications.
