#!/usr/bin/env bash
# Fingerprint every input that can affect the reusable Cookbook prefix,
# compiled repository, host installer, or the binary archives they consume.
#
# Two keys are produced:
#   toolchain_key - GCC/Rust/Clang/LLVM and the sysroot. This deliberately
#                   excludes TRION/package sources so a source-only change can
#                   restore the compiled toolchain instead of rebuilding it.
#   key           - the complete prerequisite bundle (toolchain + compiled
#                   package repository + host installer).
set -Eeuo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
: "${GITHUB_OUTPUT:?GITHUB_OUTPUT must be set by GitHub Actions}"

ARCH="${ARCH:-x86_64}"
HOST_TARGET="${HOST_TARGET:-x86_64-unknown-linux-gnu}"
case "$ARCH" in
  riscv64gc) TARGET="riscv64gc-unknown-redox" ;;
  x86_64|aarch64|i586) TARGET="$ARCH-unknown-redox" ;;
  *) echo "Unsupported Redox target architecture: $ARCH" >&2; exit 2 ;;
esac

manifest="build/simplebuild/prefix-archives.sha256"
mode_file="build/simplebuild/prefix-mode"
if [[ ! -s "$mode_file" ]]; then
  echo "Missing Redox prefix mode marker: $mode_file" >&2
  exit 1
fi
prefix_mode="$(cat "$mode_file")"
case "$prefix_mode" in
  binary)
    if [[ ! -s "$manifest" ]]; then
      echo "Prebuilt prefix mode selected but archive checksums are missing." >&2
      exit 1
    fi
    sha256sum --check "$manifest"
    ;;
  source)
    if [[ -s "$manifest" ]]; then
      echo "Source fallback selected but stale binary archive hashes remain." >&2
      exit 1
    fi
    ;;
  *)
    echo "Unknown Redox prefix mode: $prefix_mode" >&2
    exit 1
    ;;
esac

# The relibc recipe follows the upstream Git repository. Include its resolved
# revision so a moving upstream HEAD can never reuse a sysroot built from older
# libc sources.
relibc_revision="$(timeout 45s git ls-remote --exit-code \
  https://gitlab.redox-os.org/redox-os/relibc.git HEAD | awk 'NR == 1 {print $1}')"
llvm_revision="$(timeout 45s git ls-remote --exit-code \
  https://gitlab.redox-os.org/redox-os/llvm-project.git refs/heads/redox-2025-10-03 | awk 'NR == 1 {print $1}')"
if [[ ! "$relibc_revision" =~ ^[0-9a-f]{40}$ || ! "$llvm_revision" =~ ^[0-9a-f]{40}$ ]]; then
  echo "Could not resolve the upstream relibc or Redox LLVM revision; refusing a stale cache key." >&2
  exit 1
fi

# A path belongs to the compiled toolchain/sysroot when changing it can alter
# GCC, Rust, Clang, LLD, LLVM, relibc or the container they build in.
is_toolchain_path() {
  case "$1" in
    build/Makefile|build/mk/*|build/podman/*|build/podman_bootstrap.sh) return 0 ;;
    build/Cargo.toml|build/Cargo.lock|build/rust-toolchain.toml|build/code/*) return 0 ;;
    build/recipes/dev/*|build/recipes/core/relibc/*) return 0 ;;
    build/recipes/libs/libgmp/*|build/recipes/libs/libmpfr/*|build/recipes/libs/mpc/*) return 0 ;;
    build/recipes/libs/libstdcxx/*|build/recipes/libs/libstdcxx-v3/*) return 0 ;;
    build/recipes/libs/zstd/*|build/recipes/libs/zlib/*|build/recipes/libs/xz/*) return 0 ;;
    build/recipes/libs/libarchive/*|build/recipes/libs/openssl3/*|build/recipes/libs/nghttp2/*) return 0 ;;
    build/recipes/net/download/curl/*) return 0 ;;
  esac
  return 1
}

# Everything that can change the compiled package repository or installer.
is_prerequisite_input() {
  case "$1" in
    build/*) return 0 ;;
    config/*|filesystem/*|kernel/*|src/*|ternary/*|recipes/*|podman/*) return 0 ;;
    cookbook.toml|Cargo.lock|*/Cargo.lock|Cargo.toml|*/Cargo.toml) return 0 ;;
    simplebuild/*|.github/workflows/simplebuild-prerequisites.yml|.github/workflows/simplebuild-images.yml|.github/workflows/phase2-validation.yml) return 0 ;;
  esac
  return 1
}

mapfile -d '' tracked_files < <(git ls-files -z)
toolchain_inputs=()
inputs=()
for path in "${tracked_files[@]}"; do
  [[ -f "$path" ]] || continue
  if is_prerequisite_input "$path"; then
    inputs+=("$path")
  fi
  if is_toolchain_path "$path"; then
    toolchain_inputs+=("$path")
  fi
done

if [[ ${#inputs[@]} -eq 0 || ${#toolchain_inputs[@]} -eq 0 ]]; then
  echo "No tracked prerequisite inputs were found; refusing to create a cache key." >&2
  exit 1
fi

hash_paths() {
  local path
  for path in "$@"; do
    printf '%s\0' "$path"
    sha256sum -- "$path" | cut -d' ' -f1
  done | sha256sum | cut -d' ' -f1
}

toolchain_input_hash="$(hash_paths "${toolchain_inputs[@]}")"
input_hash="$(hash_paths "${inputs[@]}")"
archive_hash="$(printf '%s\n' "$(sha256sum "$mode_file" | cut -d' ' -f1)" "$(sha256sum "$manifest" | cut -d' ' -f1)" | sha256sum | cut -d' ' -f1)"

toolchain_key_material="$RUNNER_OS|$HOST_TARGET|$TARGET|$prefix_mode|$relibc_revision|$llvm_revision|$archive_hash|$toolchain_input_hash"
toolchain_digest="$(printf '%s' "$toolchain_key_material" | sha256sum | cut -d' ' -f1)"
toolchain_key="simplebuild-toolchain-${RUNNER_OS:-Linux}-$TARGET-$toolchain_digest"

# Fold the toolchain digest into the full key so a toolchain change also
# invalidates the compiled repository and installer.
key_material="$toolchain_digest|$input_hash"
key_digest="$(printf '%s' "$key_material" | sha256sum | cut -d' ' -f1)"
key="simplebuild-prerequisites-${RUNNER_OS:-Linux}-$TARGET-$key_digest"

{
  printf 'key=%s\n' "$key"
  printf 'toolchain_key=%s\n' "$toolchain_key"
  printf 'relibc_revision=%s\n' "$relibc_revision"
  printf 'llvm_revision=%s\n' "$llvm_revision"
  printf 'prefix_mode=%s\n' "$prefix_mode"
  printf 'archive_sha256=%s\n' "$archive_hash"
  printf 'target=%s\n' "$TARGET"
  printf 'host_target=%s\n' "$HOST_TARGET"
} >> "$GITHUB_OUTPUT"
echo "Prerequisite cache key: $key"
echo "Toolchain cache key: $toolchain_key"
echo "Redox target: $TARGET (host: $HOST_TARGET)"
echo "relibc source revision: $relibc_revision"
echo "Redox LLVM source revision: $llvm_revision"
echo "Prefix mode: $prefix_mode"
echo "Prefix cache fingerprint: $archive_hash"
