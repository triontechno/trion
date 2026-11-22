# SimpleBuild

SimpleBuild is TRION's small command-line interface for producing OS images.
It intentionally delegates compilation, package resolution, filesystem
installation, and image creation to the canonical Redox-derived build system
in `build/`.

This avoids maintaining two independent implementations of the same build
pipeline. SimpleBuild owns argument validation, configuration selection,
build invocation, artifact verification, and optional QEMU launch.

## Requirements

- Bash 4+
- GNU Make
- SHA-256 utilities (`sha256sum`)
- The host dependencies and Podman configuration required by `build/`
- QEMU x86_64 and UEFI firmware only when using `--qemu`

## Usage

```sh
bash simplebuild/build.sh --help
bash simplebuild/build.sh --arch x86_64 --config desktop --check
bash simplebuild/build.sh --arch x86_64 --config desktop
bash simplebuild/build.sh --arch x86_64 --config desktop --qemu
```

Supported target architectures are `x86_64`, `aarch64`, `i586`, and
`riscv64gc`. Configuration lookup follows the same precedence as
`build/mk/config.mk`: board-specific, architecture-specific, then shared.

The generated files are placed at
`build/build/<arch>/<config>/harddrive.img` and
`build/build/<arch>/<config>/redox-live.iso`. A `SHA256SUMS` manifest is
written and checked after both artifacts are verified as non-empty.

## Validation

Run `bash simplebuild/test.sh` for fast CLI/configuration checks. Pull requests
also run the SimpleBuild CI workflow, which validates the CLI and performs a
full image build through the canonical build system.
