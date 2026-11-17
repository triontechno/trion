# TRION SimpleBuild

Artifact-first image generation for TRION/Redox. From the repository root:

```sh
./simplebuild/build.sh --list-configs
./simplebuild/build.sh --arch x86_64 --config desktop
./simplebuild/build.sh --arch x86_64 --config desktop --qemu
```

Creates `build/build/<ARCH>/<CONFIG>/harddrive.img`, `redox-live.iso`,
and `SHA256SUMS`.

- Reuses `repo/` and `build/fstools/bin/redox_installer`.
- Does not invoke the traditional `make images`, Cookbook, cross-toolchain,
  or Podman bootstrap implicitly.
- Detects tracked changes under `build/recipes/` and rebuilds only affected
  recipe targets with `PODMAN_BUILD=0`.
- If the host installer is absent, checks `downloads.json`; downloads require
  HTTPS and a matching SHA-256. No guessed binary URL is used.
- Stops with an explicit prerequisite error rather than starting a full build.

## Image backend

Option C (existing Redox installer) is the initial backend. It creates the
native RedoxFS filesystem and bootloader layout. A generic ISO assembler is
not assumed compatible. Replace it with Option B only after implementation
against the actual image format and a benchmark proves it faster.

Successful builds write a cache manifest containing config and artifact
checksums under `build/simplebuild/cache/manifest.json`. Pass `--qemu` for
an interactive boot attempt after integrity validation. QEMU boot is not an
automated guest-userspace assertion. Run under Linux/WSL with Python 3.
