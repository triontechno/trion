# Trion binary-to-ternary translator

The tool has three explicit modes:

- `inspect` performs ELF/x86-64 instruction analysis.
- `translate` emits validated TIR v1 intermediate representation for a small exact instruction subset.
- `package` validates canonical TIR v1 and wraps it in a TRIONEX v1 (`.trx`) interpreted-program container.

## Supported input and translation subset

Inputs to `inspect` and `translate` must be ELF64 for x86-64 and contain at least one nonempty executable section. The current translator accepts only:

- `NOP`
- `MOV r64, imm64` using the `mov r64, imm64` encoding
- near `RET` with no immediate stack adjustment

Any invalid or unsupported instruction rejects the whole input with the source address. The tool does not silently skip code and does not leave a partial output.

ELF files containing `SHT_REL` or `SHT_RELA` sections are rejected. Relocated code bytes cannot be translated faithfully until relocation semantics and targets are implemented.

## TIR v1 and TRIONEX v1

TIR v1 is canonical UTF-8/ASCII text, newline-terminated, with this header:

```text
TRION-TIR 1 x86_64
```

TRIONEX v1 is a versioned 36-byte-header container holding validated TIR v1. It is an interpreted program container, not native machine code and not a kernel-loadable Redox executable. The bounded runtime currently supports only `NOP`, `MOV_IMM64`, and `RET`.

## Commands

From the repository root:

```sh
cargo run --manifest-path ternary/translator/Cargo.toml -- inspect path/to/program
cargo run --manifest-path ternary/translator/Cargo.toml -- translate path/to/program output.tir
cargo run --manifest-path ternary/translator/Cargo.toml -- package output.tir program.trx
cargo test --manifest-path ternary/translator/Cargo.toml
```

Both `translate` and `package` use create-new output semantics and refuse to overwrite an existing file. Packaging validates the entire TIR stream before writing the container.

Candidate mnemonic counts from `inspect` are heuristic triage only; they are not translation support. Dynamic dependencies, relocation processing, general control flow, syscalls, and ABI integration are not implemented. Unsupported input is rejected rather than approximated.
