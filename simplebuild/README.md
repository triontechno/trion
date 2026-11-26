# SimpleBuild

SimpleBuild is TRION's command-line front end for the canonical Redox-derived
build system in the build/ directory. It does not duplicate Cookbook, package
resolution, filesystem installation, or image-generation logic.

## Two-stage CI architecture

SimpleBuild separates expensive toolchain preparation from image generation.

### Stage 1: prerequisite validation

The SimpleBuild Prerequisite Validation workflow prepares the x86_64 desktop
build environment on Ubuntu 24.04 through the canonical Redox Make targets
prefix and fstools.

The published Redox toolchain-prefix endpoint returned HTTP 403 for both the
Clang and GCC archives in GitHub Actions. The workflow therefore does not use
Redox's prebuilt prefix archives:

1. Sets PREFIX_BINARY=0, selecting the canonical source-build path for the
   Redox GCC/binutils/libtool and Clang/LLVM prefixes.
2. Sets PREFIX_USE_UPSTREAM_RUST_COMPILER=1, using official upstream Rust
   compiler binaries instead of compiling the Redox Rust fork.
3. Builds the relibc sysroot and Cookbook/installer/RedoxFS host tools through
   the canonical make prefix fstools targets.
4. Verifies all compiler prefixes, sysroot, Cookbook executable, installer,
   and filesystem utilities.
5. Runs bash simplebuild/test.sh after prerequisite validation.

The simplebuild/prepare-prerequisites.sh script owns the Make invocation and
output checks. Its --dry-run mode validates the target graph without compiling.

### Incremental preservation

Successful compilation is preserved even if a later build or validation step
fails. The workflow restores a compatible prerequisite cache, packages all
existing compiler prefixes, Cookbook outputs, recipe target directories and
partial build directories, uploads the snapshot as a 30-day artifact, and
saves the available build directories to a run-specific cache key.

A partial snapshot is not considered a validated complete prerequisite
bundle. The prerequisite workflow must finish successfully before the ISO
workflow consumes its artifact.

### Stage 2: ISO generation

The separate SimpleBuild ISO Generation workflow starts automatically when
SimpleBuild Prerequisite Validation on main completes successfully. It also
supports manual dispatch, optionally selecting a successful prerequisite run
ID.

The ISO workflow:

1. Checks out the exact source commit used by the successful prerequisite run.
2. Downloads that run's artifact and verifies its SHA-256 manifest.
3. Restores compiler prefixes, relibc sysroot, Cookbook outputs, recipe target
   directories, host tools, and the Podman image.
4. Checks required prerequisite outputs and Make sentinel files.
5. Runs the SimpleBuild image builder, delegating generation to build/Makefile.
6. Verifies the generated disk image and live ISO, then uploads the desktop
   ISO, checksums, and build metadata as a separate artifact.

The ISO stage does not invoke make prefix or make fstools: it restores those
outputs from the prerequisite artifact. Package/image work still required by
the selected desktop configuration is performed during ISO generation.
QEMU is not installed or launched; it is not required to build the ISO.

## Local usage

    bash simplebuild/build.sh --help
    bash simplebuild/build.sh --arch x86_64 --config desktop --check
    bash simplebuild/build.sh --arch x86_64 --config desktop
    bash simplebuild/build.sh --arch x86_64 --config desktop --qemu

The image build remains delegated to build/Makefile. QEMU is only needed when
explicitly launching the generated image with --qemu.

## Validation

Run bash simplebuild/test.sh for CLI/configuration tests, prerequisite
Make-target dry-run validation, and regression checks. The prerequisite
workflow compiles and verifies reusable build prerequisites; the ISO workflow
validates the restored artifact and the generated images.
