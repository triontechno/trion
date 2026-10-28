#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"
mkdir -p build

# Preserve the existing build-system README before the root README becomes TRION's overview.
if [[ -f README.md && ! -f build/README.md ]]; then
  cp README.md build/README.md
fi

# Move the current repository/build-system contents under build/.
# Keep root orchestration scripts, Git metadata, the root overview, and root license in place.
items=(
  ".cargo"
  ".gitlab"
  ".gitlab-ci.yml"
  "Cargo.lock"
  "Cargo.toml"
  "CONTRIBUTING.md"
  "HARDWARE.md"
  "Makefile"
  "TRADEMARK.md"
  "bin"
  "build.sh"
  "config"
  "flake.lock"
  "flake.nix"
  "mk"
  "native_bootstrap.sh"
  "podman"
  "podman_bootstrap.sh"
  "recipes"
  "redox.ipxe"
  "rust-toolchain.toml"
  "src"
)

for item in "${items[@]}"; do
  if [[ -e "$item" ]]; then
    if [[ -e "build/$item" ]]; then
      echo "Refusing to overwrite existing build/$item while moving $item" >&2
      exit 1
    fi
    mkdir -p "build/$(dirname "$item")"
    mv "$item" "build/$item"
  fi
done

echo "Build system moved under build/. Root scripts and .github remain at the repository root."
