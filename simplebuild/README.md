# SimpleBuild

SimpleBuild is TRION's command-line front end for the canonical Redox-derived
build system in `build/`. It does not duplicate Cookbook, package resolution,
filesystem installation, or image-generation logic.

## Prerequisite validation

The `SimpleBuild Prerequisite Validation` workflow prepares the x86_64 desktop
build environment on Ubuntu 24.04 through the canonical Redox Make targets
`prefix` and `fstools`.

The published Redox toolchain-prefix endpoint returned HTTP 403 for both the
Clang and GCC archives in GitHub Actions. The workflow therefore does not use
Redox's prebuilt prefix archives:

1. Sets `PREFIX_BINARY=0`, which selects the canonical source-build path for
   the Redox GCC/binutils/libtool and Clang/LLVM prefixes.
2. Sets `PREFIX_USE_UPSTREAM_RUST_COMPILER=1`, which uses the official upstream
   Rust compiler binaries instead of compiling the Redox Rust fork. Redox's
   own build guide documents this option for x86_64 and notes that building
   Rust itself can take two hours or more.
3. Builds the relibc sysroot and Cookbook/installer/RedoxFS host tools through
   the canonical `make prefix fstools` targets.
4. Verifies all compiler prefixes, sysroot, Cookbook executable, installer,
   and filesystem utilities.
5. Runs `bash simplebuild/test.sh` and uploads a checksummed prerequisite
   artifact for the separate ISO workflow.

The `simplebuild/prepare-prerequisites.sh` script owns the exact Make
invocation and output checks. Its `--dry-run` mode validates the target graph
without compiling. The workflow caches compiler prefixes, Cookbook output,
recipe build targets, filesystem tools, and the Podman environment so later
runs can reuse completed work.

The workflow does not build an ISO. The uploaded artifact is retained for
30 days. The future ISO workflow should download the artifact from the
successful prerequisite workflow run, verify its SHA-256 manifest, extract
the prepared build outputs into the repository workspace, and invoke the
image targets without rerunning prerequisite compilation.

## Local usage

```sh
bash simplebuild/build.sh --help
bash simplebuild/build.sh --arch x86_64 --config desktop --check
bash simplebuild/build.sh --arch x86_64 --config desktop
bash simplebuild/build.sh --arch x86_64 --config desktop --qemu
```

The image build remains delegated to `build/Makefile`. QEMU is only needed
when explicitly launching the generated image with `--qemu`; it is not
required to build an ISO.

## Validation

Run `bash simplebuild/test.sh` for CLI/configuration tests, prerequisite
Make-target dry-run validation, and regression checks. The GitHub workflow
additionally compiles and verifies reusable build prerequisites before
packaging the artifact.
