# TRION SimpleBuild

Artifact-first builder for the existing TRION/Redox build tree. It generates
both `harddrive.img` and `redox-live.iso` without invoking the full
Cookbook/toolchain/Podman build by default.

## Commands

```sh
bash simplebuild/build.sh --list-configs
bash simplebuild/build.sh --arch x86_64 --config desktop --check
bash simplebuild/build.sh --arch x86_64 --config desktop
# Force a fresh image build instead of accepting cached images:
bash simplebuild/build.sh --arch x86_64 --config desktop --force
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
- The download entries are optional fallbacks for deployments that publish
  immutable artifacts externally. The GitHub Actions pipeline instead restores
  the installer and repository from its prerequisite cache and verifies them
  against `build/simplebuild/prerequisites.sha256`.
- Changed recipe source files are detected using recipe metadata, fetched
  source-repository revisions and working-tree changes, plus the main Git
  revision. The first run seeds this fingerprint baseline. Only changed recipe targets are
  rebuilt with `PODMAN_BUILD=0`; the full OS is never rebuilt implicitly.
- `--no-rebuild-changed` stops instead of rebuilding changed recipe targets. CI always uses this option, so changed recipes cannot silently invoke Make/Cookbook during an image job.
- `--force` regenerates both images even when cached image files and fingerprints are valid. CI uses it so the 30-minute timing measures real image generation, not a cache hit.
- `--check` reports missing prerequisites without starting a build.

## CI setup and execution

1. Run **Actions → Prepare SimpleBuild Prerequisites → Run workflow** once,
   and repeat it when the compiled installer or package repository must be
   refreshed. This isolated bootstrap workflow is the only SimpleBuild-related
   workflow that invokes the traditional Cookbook `make repo fstools` build.
2. The **SimpleBuild OS Images** workflow runs for relevant pull requests
   targeting `main`, relevant pushes to `main`, or manual dispatch. It does
   not invoke the full Cookbook build; it requires an exact prerequisite-cache
   match for the current build inputs, verifies the SHA-256 manifest, and
   builds both images through SimpleBuild. It fails early with instructions
   instead of silently using a stale prerequisite cache.
3. The image workflow restores compiled prerequisites, but forces both images to be regenerated. Its timing summary therefore measures real image generation rather than a cached-image no-op.
4. QEMU validation runs `simplebuild/qemu-test.sh` against the two completed image files directly. It never calls `make -C build qemu`, `make images`, or Cookbook. QEMU is a consumer/test runner for images, not an image builder.
5. The workflow records the fresh combined image-generation duration and reports whether it met the 30-minute target.

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
builds write `SHA256SUMS` and `build/simplebuild/cache/<ARCH>/<BOARD>/<CONFIG>.json`.

## QEMU

`simplebuild/qemu-test.sh <harddrive.img> <redox-live.iso>` boots both completed x86_64 artifacts directly with QEMU and checks the Redox startup and TRION capability markers over serial. It does not invoke Make or Cookbook. The workflow installs QEMU separately from the image-build dependencies. The interactive `--qemu` option remains available for manual inspection.
