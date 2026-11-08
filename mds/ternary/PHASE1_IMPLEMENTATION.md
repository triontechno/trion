# TRION Phase 1 implementation

Phase 1 now has checked ternary representations, a binary integer bridge, a versioned balanced-value format, and an x86-64 ELF-to-TIR subset translator. This is a host-side software implementation; it does not add ternary hardware, execute TIR, or establish that the OS image boots.

## Core API (`ternary/core`)

- `Ternary` stores canonical balanced B3 integers least-significant trit first. `from_i128`, `from_u128`, `to_i128`, and `to_u128` provide exact checked bridges; signed values outside the target integer range and negative-to-unsigned conversion return errors.
- `from_balanced_trits_lsf` validates B3 digits and canonicalizes high zero trits.
- `try_add`, `try_sub`, `try_negated`, and `try_mul` report storage-size/allocation failures. They use arbitrary-precision balanced digits and never wrap or saturate. The original `add`, `sub`, `negated`, and `mul` convenience APIs remain.
- `pack_u3_u8/u16/u32/u64` and `unpack_u3_u8/u16/u32/u64` implement least-significant-first U3 positional encoding. Maximum counts are 5, 10, 20, and 40. Decoders reject impossible counts and values outside the declared range. `compare_u3_u32/u64` validates same-count operands. `u3_to_balanced` and `balanced_to_u3` convert a single digit only.
- `serialize_balanced` and `deserialize_balanced` implement TRN3 v1. The format is `TRN3` magic, version byte 1, balanced-domain byte 1, little-endian `u32` trit count, and one least-significant-first payload byte per digit (`0` = -1, `1` = 0, `2` = +1). Zero is one encoded zero trit. The decoder rejects unknown versions/domains, invalid digits, noncanonical leading zeros, malformed lengths, and zero count. Incompatible future layouts require a new version.

The signed multi-trit representation is the unique balanced positional integer `sum(digit[i] * 3^i)`. For n trits the representable range is `[-(3^n-1)/2, +(3^n-1)/2]`; `Ternary` stores an unbounded canonical value, while fixed-width binary conversion is checked.

## Translator (`ternary/translator`)

`inspect` remains ELF metadata and x86-64 decoder analysis. Its candidate mnemonic counts are explicitly heuristic and do not imply translation support.

`translate <input-elf> <new-output.tir>` translates all instructions in nonempty executable ELF64 x86-64 sections only if each instruction is in the supported subset: NOP, `MOV r64, imm64` (all GPRs), and near RET with no immediate stack adjustment. ELF files with REL/RELA relocation sections, invalid instructions, or unsupported instructions fail the entire translation; no partial TIR is returned.

TIR v1 is canonical UTF-8/ASCII text, headed `TRION-TIR 1 x86_64`, with source addresses and one record per NOP, MOV_IMM64, or RET. Register ids are r0..r15 with the documented x86-64 mapping. MOV immediates are canonical balanced-ternary text with a checked u64 range. Generated TIR is passed through `validate_tir` before output, and CLI output creation refuses to overwrite an existing file.

The translator does **not** emit a native executable, implement a TIR interpreter, resolve dynamic dependencies or relocations, support branches other than RET, implement ABI/syscalls/exceptions, or prove semantic equivalence by executing translated programs. The strict byte sweep may reject executable sections containing inline data. TIR records are a typed intermediate representation, not a runnable translation.

## Tests and verification

Tests cover i128/u128 bridge boundaries, balanced arithmetic, exhaustive small U3 round trips, packed width boundaries, invalid counts/digits/noncanonical packed values, TRN3 round trips and malformed values, ELF architecture/malformed-input and relocation rejection, supported translation, unsupported/invalid instructions, TIR validation, and CLI output/no-overwrite behavior.

The commands required for verification are:

```sh
cargo test --manifest-path ternary/core/Cargo.toml
cargo check --manifest-path ternary/translator/Cargo.toml
cargo test --manifest-path ternary/translator/Cargo.toml
cargo fmt --manifest-path ternary/core/Cargo.toml -- --check
cargo fmt --manifest-path ternary/translator/Cargo.toml -- --check
cargo clippy --manifest-path ternary/core/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path ternary/translator/Cargo.toml --all-targets -- -D warnings
```

**Validation in this editing session:** not run locally yet. The available Windows host does not have `cargo` or `rustc` installed. CI workflows on `main` have been configured to run the core and translator tests, formatter, and clippy checks; their results must be recorded after the push. No full OS build, boot, executable translation, or TIR execution is claimed.

## Remaining work outside this implementation

- TIR interpreter/native code generation, ABI/syscall integration, branch and memory instruction semantics, dynamic linking/relocations, and differential execution tests.
- Fuzzing of packed decoding, TRN3 parsing, ELF/TIR parsers.
- Benchmarks measuring total storage, conversion overhead, performance, memory, and energy on specified workloads before enabling ternary paths by default.
- Compiler annotation integration, hardware-aware interfaces, and bootable OS integration.
