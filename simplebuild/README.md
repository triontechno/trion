# SimpleBuild

SimpleBuild is TRION's command-line front end for the canonical Redox-derived
build system in `build/`. It does not duplicate Cookbook, package resolution,
filesystem installation, or image-generation logic.

## Prerequisite validation

The `SimpleBuild Prerequisite Validation` workflow prepares the x86_64 desktop
build environment on Ubuntu 24.04:

1. Restores a versioned cache of compiler prefixes, Cookbook outputs, recipe
   build targets, filesystem tools, and the Podman environment.
2. Uses Redox's published binary prefixes for GCC and Rust.
3. Builds the Clang/LLVM prefix from the canonical Cookbook recipes. This is a
   deliberate fallback because the upstream `clang-install.tar.gz` download
   returned HTTP 403 from `static.redox-os.org` in GitHub Actions. The workflow
   no longer depends on that blocked download.
4. Assembles the target sysroot, including the relibc stage, using the prepared
   prefixes.
5. Verifies the compiler prefixes, sysroot, Cookbook executable, installer,
   and filesystem utilities.
6. Runs `bash simplebuild/test.sh` for CLI, configuration, and regression
   checks.
7. Uploads a checksummed prerequisite artifact for a separate ISO workflow.

The source-built Clang/LLVM outputs and relevant recipe target directories are
cached so subsequent runs can reuse completed compilation. The workflow does
not build an ISO. GCC and Rust continue to use Redox's published binary-prefix
packages; they are not rebuilt from source on every run.

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

Run `bash simplebuild/test.sh` for fast CLI/configuration checks. The GitHub
workflow additionally prepares and verifies reusable build prerequisites
before running those checks.
