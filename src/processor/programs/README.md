# TRION-H sample programs

- smoke.asm is the readable four-instruction example.
- smoke.hex is the matching 32-bit machine-code image for $readmemh.
- The demo calculates R3 = 3 and halts.

Load the image by setting IMEM_INIT_FILE to
src/processor/programs/smoke.hex when instantiating trion_h_sim_system from
the repository root. The processor testbench contains broader instruction
sequences and is the authoritative regression suite.
