# TRION

TRION is an operating-system project exploring balanced-ternary computing and a polished desktop experience.

The repository has **three primary directories** — TRION = 3:

- `build/` — Redox-derived build system, recipes, toolchain setup, and image generation. The build tool's own source belongs in `build/code/`.
- `src/` — actual OS source trees, vendored as ordinary tracked files rather than Git submodules.
- `ternary/` — TRION's balanced-ternary core and adaptive binary/ternary runtime components.

Automation lives in `.github/` and helper scripts live in `scripts/`. See [MONOREPO.md](MONOREPO.md) for the target layout and migration status. Imported upstream components retain their own licenses and copyright notices.

## Adaptive binary/ternary architecture

- [TRION OS architecture specification](TRION-ARCHITECTURE.md)
- [Phase 2 implementation plan](Phase%202.md)
- [Phase 2 Stage A specification](Phase%202%20Stage%20A.md)
- [Phase 2 Stage E — Runtime and Platform Integration](Phase%202%20Stage%20E.md)
- [Adaptive representation policy crate](ternary/adaptive/README.md)
- [Platform capability and runtime integration crate](ternary/platform/README.md)
- [Redox boot-time capability service](src/base/trion-capd/README.md)
- [Boot-time capability integration checker](src/base/trion-cap-check/README.md)

## Binary/ternary computing

- [Phase 2 Stage B](Phase%202%20Stage%20B.md)
- [Phase 2 Stage C](Phase%202%20Stage%20C.md)
- [Phase 2 Stage D](Phase%202%20Stage%20D.md)
- [Phase 2 Stage G — Direct Executable Engine](Phase%202%20Stage%20G.md)

## Final project phase

- [Phase H — UI / UX / UC](Phase%20H.md)
- [Packed ternary memory crate](ternary/memory/README.md)
