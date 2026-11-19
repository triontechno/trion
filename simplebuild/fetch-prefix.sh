#!/usr/bin/env bash
# Fetch the official Redox prebuilt cross-toolchain archives and fingerprint
# their exact bytes. Extraction and execution happen only inside Debian Trixie.
set -Eeuo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
ARCH="${ARCH:-x86_64}"
HOST_TARGET="${HOST_TARGET:-x86_64-unknown-linux-gnu}"

case "$ARCH" in
  x86_64|aarch64|i586|riscv64gc) TARGET="$ARCH-unknown-redox" ;;
  *) echo "Unsupported Redox target architecture: $ARCH" >&2; exit 2 ;;
esac

# Redox documents the prebuilt Linux prefix for x86_64 Linux hosts. Do not
# silently fall back to compiling LLVM/GCC/Rust on unsupported hosts.
if [[ "$HOST_TARGET" != "x86_64-unknown-linux-gnu" ]]; then
  echo "Prebuilt Redox prefix is not verified for host $HOST_TARGET." >&2
  exit 2
fi

PREFIX="$ROOT/build/prefix/$TARGET"
MANIFEST="$ROOT/build/simplebuild/prefix-archives.sha256"
mkdir -p "$PREFIX" "$(dirname "$MANIFEST")"
manifest_tmp="$MANIFEST.tmp"
: > "$manifest_tmp"

cleanup() {
  rm -f "$PREFIX"/*.tar.gz.partial "$manifest_tmp"
}
trap cleanup EXIT

for archive_name in gcc-install.tar.gz rust-install.tar.gz clang-install.tar.gz; do
  archive="$PREFIX/$archive_name"
  url="https://static.redox-os.org/toolchain/$HOST_TARGET/$TARGET/$archive_name"
  echo "Fetching $url"
  wget --tries=4 --timeout=60 --user-agent="Wget/1.21.3" \
    --output-document="$archive.partial" "$url"

  # Reject HTML/error pages, truncated downloads and unsafe archive paths.
  gzip -t "$archive.partial"
  listing="$archive.partial.list"
  tar -tzf "$archive.partial" > "$listing"
  test -s "$listing"
  if grep -Eq '(^/|(^|/)\.\.(/|$))' "$listing"; then
    echo "Unsafe path in $archive_name" >&2
    exit 1
  fi
  mv "$archive.partial" "$archive"
  rm -f "$listing"
  printf '%s  %s\n' "$(sha256sum "$archive" | cut -d ' ' -f1)" \
    "build/prefix/$TARGET/$archive_name" >> "$manifest_tmp"
done

mv "$manifest_tmp" "$MANIFEST"
(
  cd "$ROOT"
  sha256sum --check build/simplebuild/prefix-archives.sha256
)
archive_fingerprint="$(sha256sum "$MANIFEST" | cut -d ' ' -f1)"
echo "Redox prefix archive SHA-256 manifest: $archive_fingerprint"
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  printf 'archive_sha256=%s\n' "$archive_fingerprint" >> "$GITHUB_OUTPUT"
  printf 'target=%s\n' "$TARGET" >> "$GITHUB_OUTPUT"
  printf 'host_target=%s\n' "$HOST_TARGET" >> "$GITHUB_OUTPUT"
fi
