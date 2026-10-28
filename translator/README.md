# Trion binary-to-ternary translator

This directory is the translator front-end for Trion. The first target is **x86-64 ELF**, matching the current QEMU/release direction. The first milestone is deliberately an inspector: parse ELF metadata, decode executable sections, and report which instructions a future translator backend must implement.

## Why inspect before translating?

A binary translator cannot safely rewrite arbitrary machine code by replacing each instruction independently. It must account for control flow, flags, memory addressing, calling conventions, relocations, dynamic linking, syscalls, exceptions, and self-modifying/JIT code. This front-end makes the input ISA explicit and provides a safe foundation.

## Current status

- [x] Detect and parse x86-64 ELF inputs.
- [x] Decode executable sections with an x86-64 decoder.
- [x] Report instruction counts and instructions that need backend support.
- [ ] Build a control-flow graph and basic blocks.
- [ ] Translate a documented, tested x86-64 subset into Trion Ternary IR.
- [ ] Define a stable serialized ternary bytecode format.
- [ ] Implement the ternary VM/runtime and ABI/syscall compatibility layer.
- [ ] Add differential tests against native x86-64 execution.
- [ ] Add a bootable Trion integration target.

**This is not yet an executable translator.** The inspector must not claim that an ELF was converted or runnable. It runs on ordinary binary hardware and does not create physical ternary hardware.

## Build and run

From the repository root:

```sh
cargo run --manifest-path translator/Cargo.toml -- inspect path/to/program
cargo test --manifest-path translator/Cargo.toml
```

Static executables and dynamically linked executables are both ELF files, but this inspector does not resolve dynamic dependencies or translate system calls yet.
