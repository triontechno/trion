# TRION source monorepo

TRION is organized around three primary project directories ("TRION = 3"):

- `build/` — the Redox-derived build system, recipes, configuration, toolchain setup, and image-generation machinery. The build tool's own Rust source lives in `build/code/`, not `build/src/`.
- `src/` — actual OS component source trees (kernel, base system, filesystem, libc, shell, package tools, display system, and desktop apps). These are ordinary tracked files, not Git submodules.
- `ternary/` — TRION-specific balanced-ternary technology, including `ternary/core/` and `ternary/translator/` for binary/ternary interoperability.

Repository-level automation and migration helpers live in `.github/` and `scripts/`; they are supporting directories, not additional pillars.

## Target layout

```text
trion/
├── build/
│   ├── code/             # Cookbook/build-tool Rust sources (renamed from upstream src/)
│   ├── recipes/
│   ├── config/
│   ├── mk/
│   └── ...
├── src/
│   ├── kernel/
│   ├── base/
│   ├── redoxfs/
│   ├── relibc/
│   ├── ion/
│   ├── pkgutils/
│   ├── orbital/
│   ├── orbclient/
│   └── cosmic-*/
└── ternary/
    ├── core/
    └── translator/
```

## Build-system import and path convention

The canonical upstream build-system mirror is [redox-os/redox](https://github.com/redox-os/redox), which mirrors the Redox GitLab build-system repository. Its build files must be copied as ordinary files into `build/`; do not add the upstream repository as a submodule.

The upstream Rust crate conventionally stores source under `src/`. TRION intentionally uses `build/code/` to distinguish build-tool code from the OS source at root `src/`. When importing upstream files, update the path entries in `build/Cargo.toml` to point to `code/`. Do not blindly move the root `src/` directory: in TRION it already means OS component source.

## Imported OS sources

The source-import workflow records exact upstream commit IDs in `src/UPSTREAMS.lock`, preserves upstream license and copyright files, and excludes each checkout's `.git` metadata. The current list includes 17 upstream component trees.

## Migration status

This layout is a target, not a claim that the current checkout is already build-complete. The build system must be vendored into `build/`, recipes must be audited and adapted to consume local `src/` trees where applicable, and all recipe dependencies must be accounted for. A successful full build must produce both disk-image and live-ISO artifacts, and the resulting image must be boot-tested before TRION can be called self-contained.
