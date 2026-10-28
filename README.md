# TRION

TRION is an operating-system project exploring balanced-ternary computing and a polished desktop experience.

The repository has **three primary directories** — TRION = 3:

- `build/` — Redox-derived build system, recipes, toolchain setup, and image generation. The build tool's own source belongs in `build/code/`.
- `src/` — actual OS source trees, vendored as ordinary tracked files rather than Git submodules.
- `ternary/` — TRION's balanced-ternary core and binary/ternary translator.

Automation lives in `.github/` and helper scripts live in `scripts/`. See [MONOREPO.md](MONOREPO.md) for the target layout and migration status. Imported upstream components retain their own licenses and copyright notices.
