# TRION source monorepo

TRION is the project name (not “TRION³”). This repository currently contains the Redox build system, its cookbook recipes, and TRION-specific Rust prototypes. Most operating-system component code is still fetched by the recipes from separate upstream repositories.

## Target layout

- `source/kernel/` — kernel
- `source/base/` — base system and drivers
- `source/redoxfs/` — filesystem
- `source/relibc/` — C/POSIX compatibility library
- `source/ion/` and `source/pkgutils/` — shell and package manager
- `source/orbital/` and `source/orbclient/` — display server/window manager and client library
- `source/cosmic-*/` — desktop applications
- `ternary/` — balanced-ternary core
- `translator/` — binary inspection/translation frontend
- existing root files, `recipes/`, `config/`, and `src/` remain the build system

## Current state and migration stages

The build system and recipes are already in this repository, but that does **not** mean the component sources are already vendored. The import script below copies the selected upstream source trees into this repository and writes exact upstream commit IDs to `source/UPSTREAMS.lock`. It deliberately does not rewrite cookbook recipes yet: recipe paths, recipe patches, submodules, and reproducible pins must be migrated and tested together rather than silently changing the build.

1. Run `bash scripts/import-core-sources.sh` from the repository root to import the listed first-party OS and desktop components.
2. Update the matching recipes to use local `path = "source/…"` sources, preserving patches and build scripts.
3. Expand the source manifest to every remaining recipe dependency that TRION intends to vendor; third-party archives and their licenses need separate treatment.
4. Run the full build and boot tests before calling the repository a fully self-contained monorepo.

Imported source trees keep their upstream license and copyright files. The import script does not rewrite or remove those notices. The root repository's license does not override the licenses of vendored projects.
