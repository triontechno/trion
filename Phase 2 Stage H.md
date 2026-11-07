# TRION Phase 2 — Stage H: Isolated Guest Process Backend

**Status: bounded guest-memory model and interpreted instruction subset implemented; kernel isolation and native execution not implemented.**

## Implemented source
- Guest address-space mapping model with page alignment, overlap rejection, checked arithmetic and bounded memory/mapping quotas.
- Read/write permission enforcement and W^X rejection.
- Explicit mapping protection and unmapping operations.
- CPU register-state structure, process lifecycle states, entry/stack validation and instruction-budget metadata.\n- Bounded execution of the decoder subset against original instruction bytes, explicit syscall/breakpoint events, and fault-state transitions. This is an in-crate interpreter, not native CPU execution.

## Remaining
- Integrate address-space ownership with Redox kernel process structures and scheduler.
- Implement page-table mappings and hardware permission transitions.
- Implement architecture-specific context switching and direct native instruction execution.
- Add syscall, exception, signal, fault and thread transitions.
- Enforce process capabilities, filesystem/network/device brokers and resource accounting.
- Implement safe teardown and recovery.

**Important:** The current model is a Rust address-space/interpreter model. It is not a hardware address space and is not wired to Redox process scheduling, page tables, native context switching, or the syscall/signal/fault lifecycle. It does not enable native guest execution.
