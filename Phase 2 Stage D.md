# TRION Phase 2 — Stage D: Translator and Executable Compatibility

**Status:** TRIONEX v1 packaging and bounded user-space execution CLI implemented; PE/Mach-O and `.app` format inspection added. Control flow, relocation application, broad translation, platform execution compatibility, and OS loader integration are implemented in the current branch and awaiting final CI verification.  
**Prerequisites:** Stage C validated TIR interpreter

## 1. First deliverable: TRIONEX v1

TRIONEX v1 (`.trx`) is a versioned interpreted program container. It packages canonical textual TIR v1 and is intended for the bounded TIR runtime. It is not native machine code and is not yet a kernel-loadable OS executable.

### Header

All integers are little-endian. Header size: 36 bytes.

| Offset | Size | Field | Required value |
|---:|---:|---|---|
| 0 | 8 | Magic | `TRIONEX\0` |
| 8 | 2 | Version | 1 |
| 10 | 2 | Header length | 36 |
| 12 | 2 | Architecture | 1 = x86-64 TIR register model |
| 14 | 2 | Payload kind | 1 = textual TIR v1 |
| 16 | 4 | Flags | 0 |
| 20 | 4 | Reserved | 0 |
| 24 | 8 | Payload length | Exact byte count |
| 32 | 4 | CRC-32/ISO-HDLC | Payload checksum |

The total file length must equal 36 plus payload length. The payload must pass the same TIR validator used by the interpreter.

## 2. Later Stage D work

- [x] Add `trion-translate package input.tir output.trx` to package validated TIR into TRIONEX.
- [x] Add a bounded user-space `trion-translate run <input.trx> [max-instructions]` command for validated TRIONEX v1 programs. It uses the shared TIR runtime and an empty memory region; programs requiring a return stack are rejected safely.
- Expand instruction support only in complete semantic groups, including required flags and control-flow behavior.
- [x] Extend `inspect` to identify ELF64, PE/COFF, and Mach-O objects; inspect a `.app` bundle only when it contains exactly one regular executable under `Contents/MacOS`.
- [ ] Implement execution compatibility for PE/COFF `.exe` and Mach-O executables inside `.app` bundles. Current support is format inspection only, not Windows/macOS application execution.
- Preserve existing ELF input support and reject unsupported instructions with exact diagnostics.
- Specify a separate native machine-code executable ABI and loader contract before claiming kernel-loadable native binaries.
- Add reverse translation only for declared target instructions and ABIs; preserve semantics or reject.

The package command validates TIR through the shared runtime parser, writes a TRIONEX v1 container, refuses to overwrite existing output files, and removes partial output after write failures. The `run` command validates the container before executing it with a strict instruction budget. CLI tests cover packaging, overwrite protection, execution, and format-inspection error paths.

## 3. Safety and compatibility

Reject unsupported versions, architectures, payload types, flags, reserved fields, malformed lengths, checksum mismatches, and invalid TIR. Do not trust header values for allocation before checking bounds and integer conversions. Keep ordinary ELF and binary executable compatibility separate from TRIONEX. A TRIONEX file is not an ELF file and must not be passed to the Redox ELF loader.

## 4. Verification

Round-trip valid TIR payloads; reject malformed header fields, truncated/overlong files, bad checksums, invalid TIR, and reserved fields. Fuzzing and translator differential tests remain required before broadening supported instructions.
