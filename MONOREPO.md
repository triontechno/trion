# TRION source monorepo

TRION is the project name. This repository is being reorganized into a single monorepo: `build/` contains the existing Redox build system and cookbook, while `src/` contains actual checked-in copies of the operating-system component repositories. These are ordinary tracked files, not Git submodules.

## Target layout

- `build/` — the current build tools, recipes, configuration, Rust build-system source, and build scripts
- `src/kernel/`, `src/base/`, `src/redoxfs/`, `src/relibc/` — core OS, filesystem, and compatibility library
- `src/ion/`, `src/pkgutils/` — shell and package tools
- `src/orbital/`, `src/orbclient/` — display system and client library
- `src/cosmic-*/` — desktop applications/libraries
- `ternary/` — TRION balanced-ternary core
- `translator/` — binary/ternary interoperability tools
- `scripts/` — repository-level migration and maintenance scripts

## Migration status

The workflow imports the listed 17 upstream source trees and records their exact commit IDs in `src/UPSTREAMS.lock`. It preserves upstream license and copyright files and excludes each checkout's `.git` directory, so imported projects become ordinary files in the TRION Git history.

This is not yet a fully self-contained, build-tested monorepo. After import, the cookbook recipes under `build/recipes/` must be migrated to local source paths (including preserving recipe patches and build scripts), and all remaining recipe dependencies must be audited and vendored where appropriate. Then the complete build and boot tests must run before claiming the monorepo is complete.
