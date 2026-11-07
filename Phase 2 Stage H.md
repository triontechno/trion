# TRION Phase 2 — Stage H: Isolated Guest Process Backend

**Implementation status: partially implemented.** Native execution now uses Redox's real process/address-space interfaces for the restricted freestanding Stage G profiles. Full guest isolation policy and broker containment remain incomplete.  
**Validation:** deferred until all Phase 2 implementation stages are complete.

## Implemented source

- Bounded in-crate guest address-space model with page alignment, overlap rejection, checked arithmetic, memory/mapping quotas, and W^X policy.
- CPU register state, process lifecycle states, stack/entry validation, instruction budget, bounded instruction execution, brokered syscall events, breakpoint suspension, and explicit fault-state transitions.
- Native loader adapter in src/relibc/redox-rt/src/proc/trion.rs creates a fresh Redox address space, maps original PE32+/Mach-O64 segments, applies final page permissions, writes initial RIP/RSP through the process scheme, and lets the normal Redox scheduler/context switch enter user mode.
- Process exit is handed to the existing process manager through the OS-owned return trampoline. Unhandled processor exceptions follow the existing kernel exception path.

## Remaining

- Explicit guest exception-to-signal policy and reliable fault reporting to the launcher.
- Per-process capability policy and brokered filesystem, network, device and IPC access.
- Resource accounting, memory/CPU limits tied to kernel process objects, thread lifecycle and safe cancellation.
- Audited cleanup/recovery for every partial-load and process-failure path.
- A native backend for broader Windows/macOS runtime profiles as those services are implemented in Stages I and J.

**Important:** The bounded Rust interpreter model is not a hardware address space. Native freestanding images use actual Redox address spaces and user-mode contexts, but this does not yet provide the complete sandbox, resource broker or compatibility runtime required for general applications.
