# Trion binary-to-ternary translator

The tool has two explicit modes: `inspect` performs ELF/x86-64 instruction analysis, while `translate` emits a validated **TIR v1 intermediate representation** for a small exact instruction subset. TIR is not a native executable and is not executed by this project.

## Supported input and translation subset

Inputs must be ELF64 for x86-64 and contain at least one nonempty executable section. The translator decodes every byte of those sections and accepts only:

- `NOP` (all x86 encodings decoded by iced-x86 as NOP)
- `MOV r64, imm64` using the `mov r64, imm64` encoding; all sixteen general-purpose 64-bit registers are supported
- near `RET` with no immediate stack adjustment

Any invalid or unsupported instruction rejects the whole input with the source address. The tool does not silently skip code and does not leave a partial output. This strict section sweep can reject executables that mix inline data into executable sections.

ELF files containing `SHT_REL` or `SHT_RELA` sections are rejected. Relocated code bytes cannot be translated faithfully until relocation semantics and targets are implemented; the translator does not guess or silently ignore them.

## TIR v1 format and semantics

TIR v1 is canonical UTF-8/ASCII text, newline-terminated, with a fixed header:

```text
TRION-TIR 1 x86_64
```

Each instruction record begins with its 16-digit lowercase hexadecimal source address:

```text
0000000000400000 NOP
0000000000400001 MOV_IMM64 r0 1
000000000040000b RET
```

Register IDs are fixed: `r0..r7` mean RAX, RCX, RDX, RBX, RSP, RBP, RSI, RDI; `r8..r15` mean R8..R15. `MOV_IMM64` writes the unsigned 64-bit immediate to the entire register; its immediate is canonical balanced-ternary text (`T`, `0`, `1`) and must fit `u64`. `NOP` changes no state. `RET` pops an eight-byte return address from the stack and transfers control to it, matching near x86-64 RET. The IR currently records addresses but does not implement branch relocation, an interpreter, native code generation, ABI/syscall integration, or execution-equivalence testing.

The translator validates the complete generated TIR before writing it. The CLI uses `create_new`: it will not overwrite an existing output path. `validate_tir` checks version, canonical record syntax, operations, registers, and immediate ranges. Unit and CLI tests cover supported translation, unsupported instructions, malformed input, relocation rejection, output validation, and no-overwrite behavior.

## Inspect, translate, test

From the repository root:

```sh
cargo run --manifest-path ternary/translator/Cargo.toml -- inspect path/to/program
cargo run --manifest-path ternary/translator/Cargo.toml -- translate path/to/program output.tir
cargo test --manifest-path ternary/translator/Cargo.toml
```

The inspector's candidate counts remain heuristic triage only; they are not translation support. `translate` reports only the exact subset above. Static and dynamic ELF inputs are accepted only to the extent their executable section bytes fit the supported subset; dynamic dependencies are not resolved, and files with relocation sections are rejected. TIR is an analysis/translation artifact, not proof of an executable conversion.
