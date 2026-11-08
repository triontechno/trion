# TRION

TRION is an operating-system project exploring balanced-ternary computing and a polished desktop experience.

The repository has three primary directories — TRION = 3:

- build/ — Redox-derived build system, recipes, toolchain setup, and image generation. The build tool's own source belongs in build/code/.
- src/ — actual OS source trees, vendored as ordinary tracked files rather than Git submodules.
- mds/ternary/ — TRION's balanced-ternary core and adaptive binary/ternary runtime components.

Automation lives in .github/ and helper scripts live in scripts/. See mds/architecture/MONOREPO.md for the target layout and migration status. Imported upstream components retain their own licenses and copyright notices.

## Adaptive binary/ternary architecture

- [TRION OS architecture specification](mds/architecture/TRION-ARCHITECTURE.md)
- [Phase 2 implementation plan](mds/phases/Phase%202.md)
- [Phase 2 Stage A specification](mds/phases/Phase%202%20Stage%20A.md)
- [Phase 2 Stage E — Runtime and Platform Integration](mds/phases/Phase%202%20Stage%20E.md)
- [Adaptive representation policy crate](mds/ternary/adaptive/README.md)
- [Platform capability and runtime integration crate](mds/ternary/platform/README.md)
- [Redox boot-time capability service](mds/components/trion-capd/README.md)
- [Boot-time capability integration checker](mds/components/trion-cap-check/README.md)

## Binary/ternary computing

- [Phase 2 Stage B](mds/phases/Phase%202%20Stage%20B.md)
- [Phase 2 Stage C](mds/phases/Phase%202%20Stage%20C.md)
- [Phase 2 Stage D](mds/phases/Phase%202%20Stage%20D.md)
- [Phase 2 Stage G — Direct Executable Engine](mds/phases/Phase%202%20Stage%20G.md)
- [Direct executable engine foundation](mds/ternary/executable/README.md)

## Final project phase

- [Phase H — UI / UX / UC](mds/phases/Phase%20H.md)
- [Packed ternary memory crate](mds/ternary/memory/README.md)
