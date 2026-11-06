# TRION Phase 2 — Stage E: Runtime and Platform Integration

**Implementation status:** User-space capability-report foundation added; kernel integration remains gated.

## Delivered in this stage slice

- `ternary/platform` exposes a versioned capability report.
- The report distinguishes ordinary binary execution from the software TIR runtime.
- The current TIR operation set is explicit: NOP, MOV_IMM64, RET.
- Native ternary hardware is reported as unavailable unless a future verified platform adapter explicitly establishes it; the current host report never guesses.
- Conventional device, DMA, MMIO, network, and persistent-format boundaries remain binary.
- CI tests the report and its formatting/linting.

## Integration contract

1. Capability reports are evidence, not assumptions. A platform adapter must identify its source and validation method before reporting a hardware feature.
2. The adaptive selector consumes measured candidate costs and semantic-validation flags. It must not infer ternary capability from the CPU architecture name.
3. TIR remains a user-space bounded interpreter. No untrusted TIR is executed in the kernel.
4. Physical page allocation, page tables, DMA descriptors, MMIO registers, driver ABIs, syscalls, and network frames retain their established binary representation.
5. Packed ternary memory is an application/runtime data structure over ordinary binary-backed memory. It is not wired into the kernel physical-memory allocator.

## Remaining integration gate

The repository's current Stage E foundation does not yet provide a verified Redox kernel integration, device-driver adapter, hardware capability probe, DMA/MMIO bridge, syscall ABI, or boot-time capability service. These require architecture-specific design and QEMU/hardware validation. They must not be marked complete merely because the host capability-report crate passes tests.
