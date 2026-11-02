# TRION ternary technology

This pillar contains balanced-ternary arithmetic, checked binary bridges, and x86-64 ELF analysis with a limited TIR translation backend.

- `core/` — canonical balanced signed integers, checked arithmetic, U3 packing, TRN3 serialization, integer conversion APIs, and CLI.
- `translator/` — x86-64 ELF inspection plus validated TIR v1 output for a documented instruction subset. It does not emit an executable binary.

Both crates are software implementations for conventional binary hardware. See `PHASE1_IMPLEMENTATION.md` for precise capabilities, validation status, and limitations.
