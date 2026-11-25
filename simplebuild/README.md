# SimpleBuild

SimpleBuild is TRION's command-line front end for the canonical Redox-derived
build system in `build/`. It does not duplicate Cookbook, package resolution,
filesystem installation, or image-generation logic.

## Prerequisite validation

The `SimpleBuild Prerequisite Validation` workflow performs these steps on
Ubuntu 24.04 for the x86_64 desktop target:

1. Restores a versioned GitHub Actions cache of the Redox toolchain outputs.
2. Runs the canonical `make -C build ... prefix` target with
   `PREFIX_BINARY=1` and Podman enabled. The build system obtains and unpacks
   the GCC, Rust, and Clang binary-prefix packages and constructs the target
   sysroot, including the relibc stage.
3. Verifies that the compiler prefixes, sysroot, and Cookbook executable
   exist and are non-empty.
4. Runs `bash simplebuild/test.sh` for CLI, configuration, and regression
   checks.
5. Uploads a checksummed `simplebuild-prerequisites-<commit>` artifact for a
   separate ISO workflow to consume.

The binary-prefix mode intentionally uses Redox's published compiler packages;
it does not rebuild GCC, Rust, or LLVM from source on every run. The relibc
sysroot and Cookbook tooling are prepared by the canonical build system.
The cache key includes the build definitions and relevant recipe/toolchain
sources so changes invalidate stale outputs.

The uploaded artifact is retained for 30 days. The future ISO workflow should
download the artifact from the successful prerequisite workflow run, verify
its SHA-256 manifest, extract it into the repository workspace, and invoke
SimpleBuild without rerunning the prerequisite stage. GitHub Actions cache
entries are an acceleration layer; the uploaded artifact is the explicit
handoff between workflows.

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
