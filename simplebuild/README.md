# SimpleBuild

SimpleBuild is TRION's command-line front end for the canonical Redox-derived
build system in `build/`. It does not duplicate Cookbook, package resolution,
filesystem installation, or image-generation logic.

## Prerequisite validation

The `SimpleBuild Prerequisite Validation` workflow prepares the x86_64 desktop
build environment on Ubuntu 24.04 using the canonical Make targets `prefix`
and `fstools`.

1. Restores a versioned cache of toolchain prefixes, Cookbook outputs, recipe
   build targets, host filesystem tools, and the Podman environment.
2. Uses Redox's published binary prefixes for GCC and Rust.
3. Builds only the Clang/LLVM prefix from the canonical Cookbook recipes.
   The published `clang-install.tar.gz` endpoint returned HTTP 403 in GitHub
   Actions, so the workflow sets `PREFIX_CLANG_BINARY=0` while leaving
   `PREFIX_BINARY=1` for the GCC and Rust prefixes.
4. Assembles the target sysroot, including the relibc stage, from the prepared
   prefixes.
5. Builds the Cookbook, installer, and RedoxFS host tools.
6. Verifies all compiler prefixes, the sysroot, Cookbook executable, installer,
   and filesystem utilities.
7. Runs `bash simplebuild/test.sh` and uploads a checksummed prerequisite
   artifact for the separate ISO workflow.

The independent `PREFIX_CLANG_BINARY` setting is forwarded into nested Podman
Make invocations. This prevents the Clang fallback from accidentally changing
GCC/Rust build modes or triggering a complete source rebuild of the toolchain.
The source-built Clang/LLVM outputs and recipe target directories are cached.
GCC and Rust continue to use Redox's published binary-prefix packages.

The uploaded artifact is retained for 30 days. The future ISO workflow should
download the artifact from the successful prerequisite workflow run, verify
its SHA-256 manifest, extract the prepared build outputs into the repository
workspace, and invoke the image targets without rerunning prerequisite
compilation. GitHub Actions cache entries accelerate later prerequisite runs;
the uploaded artifact is the explicit handoff between workflows.

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

Run `bash simplebuild/test.sh` for CLI/configuration checks, prerequisite
Make-target dry-run validation, and regression checks. The GitHub workflow
additionally prepares and verifies reusable build prerequisites before
packaging the artifact.
