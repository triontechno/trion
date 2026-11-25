#!/usr/bin/env bash
# Prepare the reusable Redox cross-toolchain, sysroot, Cookbook, and host tools.
# This is a prerequisite-only stage: it never creates an OS image or ISO.
set -Eeuo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
MAKE_BIN="${MAKE:-make}"
ARCH="${TRION_ARCH:-x86_64}"
CONFIG_NAME="${TRION_CONFIG_NAME:-desktop}"
MODE="build"

case "${1:-}" in
  "") ;;
  --dry-run) MODE="dry-run"; shift ;;
  -h|--help)
    cat <<'EOF'
Usage: simplebuild/prepare-prerequisites.sh [--dry-run]

Builds the reusable x86_64 desktop prerequisites through the canonical
Redox Makefile. The compiler prefix is built from source because the
published Redox binary-prefix endpoint returns HTTP 403. The official
upstream Rust toolchain is used to avoid rebuilding Rust itself.
--dry-run validates the canonical Make target graph without compiling.
EOF
    exit 0
    ;;
  *) printf 'prepare-prerequisites: unknown option: %s\n' "$1" >&2; exit 2 ;;
esac

if [[ "$ARCH" != "x86_64" || "$CONFIG_NAME" != "desktop" ]]; then
  printf 'prepare-prerequisites: CI currently supports x86_64/desktop only\n' >&2
  exit 2
fi

command -v "$MAKE_BIN" >/dev/null 2>&1 || {
  printf 'prepare-prerequisites: GNU make is required\n' >&2
  exit 1
}

make_args=(
  -C "$ROOT/build"
  "ARCH=$ARCH"
  "CONFIG_NAME=$CONFIG_NAME"
  "PREFIX_BINARY=0"
  "PREFIX_USE_UPSTREAM_RUST_COMPILER=1"
  "PODMAN_BUILD=1"
  "FSTOOLS_IN_PODMAN=1"
  prefix
  fstools
)

if [[ "$MODE" == "dry-run" ]]; then
  "$MAKE_BIN" -n "${make_args[@]}" >/dev/null
  printf 'prepare-prerequisites: canonical source-build target graph is valid\n'
  exit 0
fi

printf 'prepare-prerequisites: building the GCC/Clang prefixes and relibc sysroot from source\n'
printf 'prepare-prerequisites: using the official upstream Rust compiler binaries\n'
"$MAKE_BIN" "${make_args[@]}"

prefix_dir="$ROOT/build/prefix/$ARCH-unknown-redox"
for output in gcc-install rust-install clang-install sysroot; do
  if [[ ! -d "$prefix_dir/$output" ]] || ! find "$prefix_dir/$output" -mindepth 1 -print -quit | grep -q .; then
    printf 'prepare-prerequisites: missing or empty prerequisite: %s\n' "$prefix_dir/$output" >&2
    exit 1
  fi
done

test -x "$ROOT/build/target/release/repo"
test -s "$ROOT/build/build/fstools.tag"
test -s "$ROOT/build/build/container.tag"
test -x "$ROOT/build/build/fstools/bin/redox_installer"
test -x "$ROOT/build/build/fstools/bin/redoxfs"
test -x "$ROOT/build/build/fstools/bin/redoxfs-mkfs"

printf 'prepare-prerequisites: all compiler prefixes, sysroot, Cookbook, installer, and filesystem tools verified\n'
