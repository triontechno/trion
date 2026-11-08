# TRION source monorepo

TRION has three primary project directories (TRION = 3):

- `build/` — the Redox-derived build system, recipes, configuration, toolchain setup, and image-generation machinery. The build tool's Rust source belongs in `build/code/`.
- `src/` — OS component source trees (kernel, base system, filesystem, libc, shell, package tools, display system, and desktop apps), stored as ordinary tracked files rather than submodules.
- `ternary/` — TRION-specific balanced-ternary technology: `ternary/core/` and `ternary/translator/`.

Repository automation and migration helpers live in `.github/` and `scripts/`; they are supporting directories, not additional pillars.

## Target layout

```text
trion/
├── build/
│   ├── code/             # Build-tool Rust sources
│   ├── recipes/
│   ├── config/
│   ├── mk/
│   └── ...
├── src/                  # Vendored OS component trees
└── ternary/
    ├── core/             # Balanced-ternary arithmetic crate
    └── translator/       # x86-64 ELF inspection frontend
```

## Build-system import and status

The canonical upstream build-system mirror is [redox-os/redox](https://github.com/redox-os/redox). The importer copies upstream files into `build/`, puts upstream Rust sources in `build/code/`, and records the exact imported revision in `build/UPSTREAM.lock`. It does not add a submodule.

The importer has not yet been run and the full OS image pipeline has not been validated. After import, recipe source paths and dependencies must be audited, the image build must run in CI, and the resulting ISO/IMG must be boot-tested before claiming a working release.
