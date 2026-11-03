# TRION-H Binary ISA — Phase 2

## Architectural state

| Item | Definition |
| --- | --- |
| Instruction width | 32 bits, fixed length |
| Datapath width | 128-bit unsigned bit-vector |
| General-purpose registers | 32 x 128-bit |
| Register zero | R0 always reads zero; writes are discarded |
| Program counter | 64-bit byte address |
| Instruction alignment | 4 bytes |
| Data transfer width | 128 bits |
| Data alignment | 16 bytes |

All 128-bit integer operations wrap modulo 2^128. There is no signed
arithmetic instruction in this revision; signed interpretation is used only
for sign-extending immediates and calculating relative branch/jump targets.

Registers R1 through R31 are general-purpose. No stack-pointer, return
address, callee-saved, or caller-saved ABI convention is frozen by this RTL.
JAL writes its link value to the destination register specified by the
instruction.

## Instruction encoding

All instructions use the same 32-bit field layout:

| Bits | Width | Field |
| --- | ---: | --- |
| [31:25] | 7 | opcode |
| [24:20] | 5 | rd |
| [19:15] | 5 | rs1 |
| [14:10] | 5 | rs2 |
| [9:0] | 10 | imm10 |

imm10 is a signed two's-complement immediate in the range -512 to +511.
Instructions that do not use a field ignore it; assemblers should encode
unused fields as zero.

### Opcode map

| Opcode | Mnemonic | Operation |
| --- | --- | --- |
| 0x01 | ADD | rd = rs1 + rs2 |
| 0x02 | SUB | rd = rs1 - rs2 |
| 0x03 | AND | rd = rs1 & rs2 |
| 0x04 | OR | rd = rs1 OR rs2 |
| 0x05 | XOR | rd = rs1 XOR rs2 |
| 0x06 | SLL | rd = rs1 << rs2[6:0] |
| 0x07 | SRL | rd = rs1 >> rs2[6:0] |
| 0x08 | ADDI | rd = rs1 + sign_extend(imm10) |
| 0x09 | LD | rd = memory128[rs1 + sign_extend(imm10)] |
| 0x0A | ST | memory128[rs1 + sign_extend(imm10)] = rs2 |
| 0x0B | BEQ | If rs1 == rs2, branch relative to current PC |
| 0x0C | BNE | If rs1 != rs2, branch relative to current PC |
| 0x0D | JAL | rd = PC + 4; branch relative to current PC |
| 0x7E | HALT | Stop execution and retire the halt instruction |

All unlisted opcodes are illegal and fault. No packed or multi-valued
arithmetic operations are part of this ISA.

## Execution semantics

### Binary ALU

ADD, SUB, AND, OR, and XOR operate over all 128 bits. Addition and
subtraction wrap at 128 bits. SLL and SRL are logical shifts; the shift
amount is the low seven bits of rs2.

### Immediate arithmetic

ADDI adds the sign-extended 10-bit immediate to the 128-bit value in rs1.
The result is written to rd, except that writes to R0 are discarded.

### Loads and stores

The effective byte address is the low 64 bits of rs1 plus the sign-extended
10-bit immediate. LD loads one 128-bit word into rd. ST stores the full
128-bit value of rs2.

Effective addresses must be divisible by 16. The current interface does not
support byte, halfword, or narrower loads/stores. The memory bus uses byte
addresses; the memory adapter selects the 128-bit word at address [...:4].

### Branches and jumps

For BEQ, BNE, and JAL, the signed immediate is a displacement in
instruction words. The byte target is:

target = PC + (sign_extend(imm10) * 4)

BEQ and BNE compare the full 128-bit register values. A not-taken branch
uses PC + 4. JAL writes PC + 4 to rd and updates the PC to the
relative target. Branch targets remain four-byte aligned because the
displacement is scaled by four.

### HALT and retirement

A valid non-HALT instruction retires on its execution clock edge, updates the
PC, and increments retired_count. HALT also retires and increments the
counter, then asserts halted; the PC remains at the HALT instruction.
An instruction that faults is not retired. instruction_retired is a
one-cycle pulse.

## Fault model

Faults are terminal and sticky until reset. The core records fault_pc and
fault_instruction; fault_code is:

| Code | Name | Meaning |
| --- | --- | --- |
| 0 | FAULT_NONE | No fault |
| 1 | FAULT_PC_ALIGNMENT | PC is not four-byte aligned |
| 2 | FAULT_INSTRUCTION_RANGE | Instruction fetch is outside configured memory |
| 3 | FAULT_ILLEGAL_OPCODE | Opcode is not implemented |
| 4 | FAULT_DATA_ALIGNMENT | Load/store address is not 16-byte aligned |
| 5 | FAULT_DATA_RANGE | Load/store address is outside configured memory |

A faulting instruction does not update the PC, write a register, commit a
store, or increment the retirement counter. Reset clears terminal status and
architectural state.

## Decisions not frozen by this RTL

The following are explicitly left for a later architecture revision:
interrupts, exception vectors and return, privilege modes, CSR/system
instructions, ABI register roles, byte ordering for subword access, signed
comparison/branch instructions, arithmetic right shift, atomic operations,
memory wait states, and the final external bus protocol.
