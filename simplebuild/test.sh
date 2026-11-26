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


# Validate the source-built prerequisite entrypoint, its help, and the exact
# canonical Make target graph used by GitHub Actions. No compiler is built here.
bash -n simplebuild/prepare-prerequisites.sh
bash simplebuild/prepare-prerequisites.sh --help >/dev/null
bash simplebuild/prepare-prerequisites.sh --dry-run

grep -Fq '"PREFIX_BINARY=0"' simplebuild/prepare-prerequisites.sh
grep -Fq '"PREFIX_USE_UPSTREAM_RUST_COMPILER=1"' simplebuild/prepare-prerequisites.sh
grep -Fq 'prefix' simplebuild/prepare-prerequisites.sh
grep -Fq 'fstools' simplebuild/prepare-prerequisites.sh
grep -Fq 'build/build/fstools/bin/redox_installer' simplebuild/prepare-prerequisites.sh

# The image build compiles redox-rt; keep the vec! macro in scope there.
if ! grep -Fq 'use alloc::{boxed::Box, vec, vec::Vec};' src/relibc/redox-rt/src/proc.rs; then
  echo "redox-rt proc.rs must import alloc::vec for vec! macro usage" >&2
  exit 1
fi
echo "SimpleBuild validation passed."
