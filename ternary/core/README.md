# TRION balanced-ternary core

This directory is reserved for TRION's native balanced-ternary implementation.

Design goals:
- Represent each trit as -1, 0, or +1.
- Define balanced-ternary arithmetic and serialization independently of binary machine integers where practical.
- Add unit tests for normalization, overflow policy, arithmetic, and conversion boundaries.

Status: scaffold only; no production implementation has been added yet.
