# TRION Phase 2 — Stage H: Isolated Guest Process Backend

**Status: safe foundational model implemented; OS integration and native execution not implemented.**

## Implemented source
- Guest address-space mapping model with page alignment, overlap rejection, checked arithmetic and bounded memory/mapping quotas.
- Read/write permission enforcement and W^X rejection.
- Explicit mapping protection and unmapping operations.
- CPU register-state structure, process lifecycle states, entry/stack validation and instruction-budget metadata.

## Remaining
- Integrate address-space ownership with Redox kernel process structures and scheduler.
- Implement page-table mappings and hardware permission transitions.
- Implement architecture-specific context switching and direct native instruction execution.
- Add syscall, exception, signal, fault and thread transitions.
- Enforce process capabilities, filesystem/network/device brokers and resource accounting.
- Implement safe teardown and recovery.

**Important:** The current model is a Rust policy/data structure. It is not a hardware address space and does not execute guest instructions. No guest execution is enabled.
