# TRION-H Processor

TRION-H is implemented here as a binary, 128-bit datapath processor. The RTL in
this directory is the Phase 2 processor implementation; this is not an FPGA
bitstream, a physical-hardware validation, or an operating-system integration.

## Architecture

- 32-bit fixed-width instructions.
- 128-bit binary datapath.
- 32 general-purpose registers, each 128 bits wide.
- Register R0 reads as zero and ignores writes.
- 64-bit byte-addressed program counter.
- Single-cycle, non-pipelined execution core.
- Separate instruction and data memory interfaces.
- 128-bit aligned data transfers.
- Sticky terminal fault state with a fault code, fault PC, and faulting instruction.
- Retired-instruction pulse and 64-bit retirement counter.

The instruction encoding and the documented architectural limitations are in
ISA.md.

## Source files

| File | Purpose |
| --- | --- |
| design.v | Decoder, binary ALU, register file, processor core, simulation memory, and Makerchip top wrapper. |
| testbench.v | Self-checking processor and ALU verification suite. |
| Makefile | Explicit Icarus Verilog and Verilator commands. |
| ISA.md | Instruction formats, opcode map, memory semantics, and fault model. |

## Simulation memory

trion_h_sim_system provides 256 instruction words and 256 data words by
default. Instruction memory is initialized to HALT and data memory to zero,
so the simulation does not fetch unknown values by default.

The wrapper accepts IMEM_INIT_FILE and DMEM_INIT_FILE parameters for
$readmemh initialization. The reusable smoke program is in
programs/smoke.asm and its pre-encoded image is programs/smoke.hex. For
example, set IMEM_INIT_FILE to "src/processor/programs/smoke.hex" when
instantiating the simulation wrapper from the repository root. Its
instruction/data arrays can also be loaded directly by a testbench. The
memory wrapper is for simulation; the processor core itself uses external
memory ports intended for later FPGA or SoC integration.

The simulation data memory is 4 KiB (256 x 16 bytes); instruction memory is
1 KiB (256 x 4 bytes). Out-of-range accesses raise processor faults.

## Makerchip

The file defines a top wrapper with the interface expected by the
TRION-H Makerchip setup:

- clk
- reset_async
- passed
- failed

The wrapper runs a fixed four-instruction smoke program. passed means the
program reached HALT without a processor fault; it is only a progress
indicator and does not replace the self-checking testbench. failed asserts
on a processor fault.

## Commands

The test suite is intentionally manual. Run these only when Phase 2 testing is
authorized.

From the repository root:

```sh
make -C src/processor test-icarus
make -C src/processor lint-verilator
make -C src/processor test-verilator
```

test-icarus compiles and runs the testbench. lint-verilator performs lint
only. test-verilator builds and runs the same testbench with Verilator.
Generated files are placed under .build/processor/.

No ISO, desktop image, live image, or installation image is required for
processor verification. No release-image target is provided by this Makefile.

## Current limitations

- The core is single-cycle and does not implement stalls, wait states, or
  backpressure.
- The instruction and data memory interfaces assume combinational reads and
  edge-triggered writes; a future bus adapter or FPGA memory controller will
  be needed for memories with different latency.
- Data memory operations currently transfer one aligned 128-bit word.
  Byte/halfword operations and endianness-visible subword behavior are not
  defined.
- Faults are sticky until reset. There is no exception vector, interrupt
  controller, privilege-mode transition, or software trap-return instruction.
- There is no cache, MMU, virtual memory, debug module, or OS/ABI integration.
- The RTL is written in synthesizable Verilog for the decoder, ALU, register
  file, and core. The simulation memory and Makerchip demo wrapper contain
  simulation-oriented initialization and should be excluded or replaced in a
  board-specific synthesis top.
- No FPGA part, clock target, pinout, or board is selected, so board-specific
  timing and pin constraints are intentionally not supplied.

No physical FPGA or silicon validation is claimed.
