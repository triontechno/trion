#!/usr/bin/env bash
# Fingerprint every input that can affect the reusable Cookbook prefix,
# compiled repository, host installer, or the binary archives they consume.
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
if [[ ! -s "$manifest" ]]; then
  echo "Missing pre-fetched Redox prefix checksum manifest: $manifest" >&2
  exit 1
fi
sha256sum --check "$manifest"

# The relibc recipe follows the upstream Git repository. Include its resolved
# revision so a moving upstream HEAD can never reuse a sysroot built from older
# libc sources.
relibc_revision="$(timeout 45s git ls-remote --exit-code \
  https://gitlab.redox-os.org/redox-os/relibc.git HEAD | awk 'NR == 1 {print $1}')"
if [[ ! "$relibc_revision" =~ ^[0-9a-f]{40}$ ]]; then
  echo "Could not resolve the upstream relibc revision; refusing a stale cache key." >&2
  exit 1
fi

mapfile -d '' tracked_files < <(git ls-files -z)
inputs=()
for path in "${tracked_files[@]}"; do
  case "$path" in
    build/Makefile|build/mk/*|build/podman/*|build/podman_bootstrap.sh|build/recipes/*|build/config/*|build/redox.ipxe|config/*|filesystem/*|kernel/*|src/*|ternary/*|recipes/*|podman/*|cookbook.toml|Cargo.lock|*/Cargo.lock|Cargo.toml|*/Cargo.toml|simplebuild/*|.github/workflows/simplebuild-prerequisites.yml|.github/workflows/simplebuild-images.yml)
      [[ -f "$path" ]] && inputs+=("$path")
      ;;
  esac
done

if [[ ${#inputs[@]} -eq 0 ]]; then
  echo "No tracked prerequisite inputs were found; refusing to create a cache key." >&2
  exit 1
fi

input_hash=$(
  for path in "${inputs[@]}"; do
    printf '%s\0' "$path"
    sha256sum -- "$path" | cut -d' ' -f1
  done | sha256sum | cut -d' ' -f1
)
archive_hash="$(sha256sum "$manifest" | cut -d' ' -f1)"
key_material="$RUNNER_OS|$HOST_TARGET|$TARGET|$relibc_revision|$archive_hash|$input_hash"
key_digest="$(printf '%s' "$key_material" | sha256sum | cut -d' ' -f1)"
key="simplebuild-prerequisites-${RUNNER_OS:-Linux}-$TARGET-$key_digest"

{
  printf 'key=%s\n' "$key"
  printf 'relibc_revision=%s\n' "$relibc_revision"
  printf 'archive_sha256=%s\n' "$archive_hash"
  printf 'target=%s\n' "$TARGET"
  printf 'host_target=%s\n' "$HOST_TARGET"
} >> "$GITHUB_OUTPUT"
echo "Prerequisite cache key: $key"
echo "Redox target: $TARGET (host: $HOST_TARGET)"
echo "relibc source revision: $relibc_revision"
echo "Prefix archive manifest SHA-256: $archive_hash"
