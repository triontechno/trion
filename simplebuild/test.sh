#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

bash -n simplebuild/build.sh
bash -n simplebuild/test.sh
bash simplebuild/build.sh --help >/dev/null
bash simplebuild/build.sh --arch x86_64 --config desktop --check
if bash simplebuild/build.sh --arch invalid --check >/dev/null 2>&1; then
  echo "invalid architecture was accepted" >&2
  exit 1
fi
if bash simplebuild/build.sh --unknown-option >/dev/null 2>&1; then
  echo "unknown option was accepted" >&2
  exit 1
fi


# Keep the Clang fallback independently selectable inside the nested Podman make.
grep -Fq 'PREFIX_CLANG_BINARY?=$(PREFIX_BINARY)' build/mk/config.mk
grep -Fq -- '--env PREFIX_CLANG_BINARY=$(PREFIX_CLANG_BINARY)' build/mk/podman.mk
grep -Fq 'PREFIX_CLANG_BINARY=0' .github/workflows/simplebuild.yml

# Parse the exact aggregate targets used by CI. This catches invalid leaf-target
# names before an expensive toolchain build starts.
make -n -C build \
  ARCH=x86_64 \
  CONFIG_NAME=desktop \
  PREFIX_BINARY=1 \
  PREFIX_CLANG_BINARY=0 \
  PODMAN_BUILD=1 \
  FSTOOLS_IN_PODMAN=1 \
  prefix fstools >/dev/null

# Host tools live under build/build because the imported Makefile sets
# FSTOOLS=build/fstools relative to its own build/ working directory.
grep -Fq 'build/build/fstools/bin/redox_installer' .github/workflows/simplebuild.yml

# The image build compiles redox-rt; keep the vec! macro in scope there.
if ! grep -Fq 'use alloc::{boxed::Box, vec, vec::Vec};' src/relibc/redox-rt/src/proc.rs; then
  echo "redox-rt proc.rs must import alloc::vec for vec! macro usage" >&2
  exit 1
fi
echo "SimpleBuild validation passed."
