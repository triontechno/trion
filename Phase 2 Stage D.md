# TRION Phase 2 — Stage D: Translator and Executable Compatibility

**Status:** TRIONEX v1 container library and validated TIR packaging CLI implemented; instruction expansion and executable/runtime integration remain in progress.  
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
- Add a runtime command to inspect and execute TRIONEX with an explicitly supplied test stack.
- Expand instruction support only in complete semantic groups, including required flags and control-flow behavior.
- Preserve existing ELF input support and reject unsupported instructions with exact diagnostics.
- Specify a separate native machine-code executable ABI and loader contract before claiming kernel-loadable native binaries.
- Add reverse translation only for declared target instructions and ABIs; preserve semantics or reject.

The package command validates TIR through the shared runtime parser, writes a TRIONEX v1 container, refuses to overwrite existing output files, and removes partial output after write failures. CLI tests cover valid packaging and overwrite protection.

## 3. Safety and compatibility

Reject unsupported versions, architectures, payload types, flags, reserved fields, malformed lengths, checksum mismatches, and invalid TIR. Do not trust header values for allocation before checking bounds and integer conversions. Keep ordinary ELF and binary executable compatibility separate from TRIONEX. A TRIONEX file is not an ELF file and must not be passed to the Redox ELF loader.

## 4. Verification

Round-trip valid TIR payloads; reject malformed header fields, truncated/overlong files, bad checksums, invalid TIR, and reserved fields. Fuzzing and translator differential tests remain required before broadening supported instructions.
