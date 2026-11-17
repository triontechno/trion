# TRION SimpleBuild

Artifact-first builder for the existing TRION/Redox build tree. It generates
both `harddrive.img` and `redox-live.iso` without invoking the full
Cookbook/toolchain/Podman build by default.

## Commands

```sh
bash simplebuild/build.sh --list-configs
bash simplebuild/build.sh --arch x86_64 --config desktop --check
bash simplebuild/build.sh --arch x86_64 --config desktop
bash simplebuild/build.sh --arch x86_64 --config desktop --qemu
```

Use `--board <board>` for board-specific configurations. Output paths match
the traditional Makefile's `BUILD=build/<ARCH>/<CONFIG_NAME>`:
`build/build/<ARCH>/<CONFIG_NAME>/`.

## Reuse, downloads and rebuilds

- Uses `build/config/`, `build/fstools/bin/redox_installer`,
  `build/repo/`, and `build/redox.ipxe` (the actual paths from
  `build/mk/config.mk` and `build/mk/disk.mk`).
- If the installer or compiled repository is missing, attempts download only
  when `simplebuild/downloads.json` has a corresponding HTTPS URL and
  matching SHA-256. A repository archive must be a tar archive containing
  repository files at its root, or a single top-level `repo/` directory.
- Changed recipe source files are detected using recipe metadata, fetched
  source-repository revisions and working-tree changes, plus the main Git
  revision. The first run seeds this fingerprint baseline. Only changed recipe targets are
  rebuilt with `PODMAN_BUILD=0`; the full OS is never rebuilt implicitly.
- `--no-rebuild-changed` stops instead of rebuilding changed recipe targets.
- `--check` reports missing prerequisites without starting a build.

## Image backend

Backend C (the existing Redox installer) is used initially because the
traditional `build/mk/disk.mk` relies on its RedoxFS and live bootloader
semantics. An independent assembler (B) should be selected only after it
exists and benchmarks faster while producing equivalent bootable images.
The backend boundary is isolated in the image-generation function.

## Cache and verification

Cache reuse requires matching SHA-256 fingerprints for the filesystem
configuration, installer, complete compiled package repository and iPXE
script, plus matching SHA-256 checksums for both output images. Successful
builds write `SHA256SUMS` and `build/simplebuild/cache/manifest.json`.

## QEMU

`--qemu` starts an interactive x86_64 UEFI/NVMe boot attempt using QEMU and
OVMF. Other architectures can build images, but their QEMU launch profiles
are not yet implemented. A QEMU process starting is not proof that the guest
reached userspace; inspect the VM boot screen/serial output.
