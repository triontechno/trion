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
echo "SimpleBuild validation passed."
