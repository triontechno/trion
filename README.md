# TRION

TRION is an operating-system project exploring balanced-ternary computing and a polished desktop experience.

## Repository layout

- `build/` — build tools and cookbook (the existing Redox build system)
- `src/` — actual OS component source trees imported into this monorepo as ordinary files, not submodules
- `ternary/` — balanced-ternary core
- `translator/` — binary/ternary interoperability tools
- `scripts/` — repository-level maintenance scripts

See [MONOREPO.md](MONOREPO.md) for the migration plan and current limitations. Imported upstream components retain their own licenses and copyright notices.
