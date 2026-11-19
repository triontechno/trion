#!/usr/bin/env bash
# Prefer the official Redox binary prefix. If the public toolchain endpoint
# denies access (403/404), explicitly fall back to a one-time source bootstrap;
# the completed prefix/sysroot is then cached by the prerequisite workflow.
set -Eeuo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
ARCH="${ARCH:-x86_64}"
HOST_TARGET="${HOST_TARGET:-x86_64-unknown-linux-gnu}"

case "$ARCH" in
  x86_64|aarch64|i586|riscv64gc) TARGET="$ARCH-unknown-redox" ;;
  *) echo "Unsupported Redox target architecture: $ARCH" >&2; exit 2 ;;
esac

PREFIX="$ROOT/build/prefix/$TARGET"
MANIFEST="$ROOT/build/simplebuild/prefix-archives.sha256"
MODE_FILE="$ROOT/build/simplebuild/prefix-mode"
mkdir -p "$PREFIX" "$(dirname "$MANIFEST")"
manifest_tmp="$MANIFEST.tmp"
response_log="$PREFIX/.prefix-download-response"
: > "$manifest_tmp"

cleanup() {
  rm -f "$PREFIX"/*.tar.gz.partial "$PREFIX"/*.partial.list "$manifest_tmp" "$response_log"
}
trap cleanup EXIT

use_source_fallback() {
  echo "::warning::Redox prebuilt prefix archives are not publicly downloadable (HTTP 403/404). Using PREFIX_BINARY=0 for this one-time bootstrap; the completed prefix and sysroot will be cached."
  rm -f "$PREFIX"/*.tar.gz "$PREFIX"/*.partial "$PREFIX"/*.partial.list
  : > "$MANIFEST"
  printf 'source\n' > "$MODE_FILE"
  fingerprint="$(sha256sum "$MODE_FILE" "$MANIFEST" | sha256sum | cut -d' ' -f1)"
  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    printf 'prefix_binary=0\n' >> "$GITHUB_OUTPUT"
    printf 'archive_sha256=%s\n' "$fingerprint" >> "$GITHUB_OUTPUT"
    printf 'target=%s\n' "$TARGET" >> "$GITHUB_OUTPUT"
    printf 'host_target=%s\n' "$HOST_TARGET" >> "$GITHUB_OUTPUT"
  fi
  echo "Prefix mode: source fallback (PREFIX_BINARY=0)"
  echo "Prefix cache fingerprint: $fingerprint"
  exit 0
}

if [[ "$HOST_TARGET" != "x86_64-unknown-linux-gnu" ]]; then
  echo "Prebuilt Redox prefix is not verified for host $HOST_TARGET." >&2
  exit 2
fi

for archive_name in gcc-install.tar.gz rust-install.tar.gz clang-install.tar.gz; do
  archive="$PREFIX/$archive_name"
  downloaded=0
  saw_forbidden=0

  for base in \
    "https://static.redox-os.org/toolchain/$HOST_TARGET/$TARGET" \
    "https://static.redox-os.org/toolchain/$TARGET"; do
    url="$base/$archive_name"
    echo "Fetching $url"
    if wget --tries=2 --timeout=60 --user-agent="Wget/1.21.3" \
      --output-document="$archive.partial" "$url" 2>"$response_log"; then
      downloaded=1
      break
    fi

    cat "$response_log"
    if grep -Eq 'HTTP request sent, awaiting response\.\.\. (403|404)' "$response_log"; then
      saw_forbidden=1
      rm -f "$archive.partial"
      continue
    fi
    echo "Network or server error while fetching $archive_name; refusing to silently fall back." >&2
    exit 1
  done

  if (( downloaded == 0 )); then
    if (( saw_forbidden == 1 )); then
      use_source_fallback
    fi
    echo "No Redox prefix archive found for $TARGET/$archive_name." >&2
    exit 1
  fi

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
printf 'binary\n' > "$MODE_FILE"
(
  cd "$ROOT"
  sha256sum --check build/simplebuild/prefix-archives.sha256
)
fingerprint="$(sha256sum "$MODE_FILE" "$MANIFEST" | sha256sum | cut -d' ' -f1)"
echo "Prefix mode: prebuilt binary (PREFIX_BINARY=1)"
echo "Redox prefix archive SHA-256 fingerprint: $fingerprint"
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  printf 'prefix_binary=1\n' >> "$GITHUB_OUTPUT"
  printf 'archive_sha256=%s\n' "$fingerprint" >> "$GITHUB_OUTPUT"
  printf 'target=%s\n' "$TARGET" >> "$GITHUB_OUTPUT"
  printf 'host_target=%s\n' "$HOST_TARGET" >> "$GITHUB_OUTPUT"
fi
