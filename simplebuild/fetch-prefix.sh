#!/usr/bin/env bash
# Fetch Redox prebuilt toolchain archives (GCC, Rust, Clang/LLD).
# Tries GitHub release assets first (reliable from Actions runners), then
# falls back to the official static.redox-os.org endpoint. Verifies each
# archive against pinned SHA-256 hashes to guarantee integrity.
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
EXPECTED="$ROOT/simplebuild/prefix-archives.expected.sha256"
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
  fingerprint="$(printf '%s\n' "$(sha256sum "$MODE_FILE" | cut -d' ' -f1)" "$(sha256sum "$MANIFEST" | cut -d' ' -f1)" | sha256sum | cut -d' ' -f1)"
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

# Load expected SHA-256 hashes from tracked file.
declare -A EXPECTED_SHA256
if [[ ! -f "$EXPECTED" ]]; then
  echo "Missing expected checksums file: $EXPECTED" >&2
  exit 1
fi
while IFS=' ' read -r hash name || [[ -n "${hash:-}${name:-}" ]]; do
  # read returns non-zero for a final line without a newline. Process it too;
  # otherwise the last pinned archive checksum is silently lost.
  # Strip potential carriage returns from Windows line endings.
  name="${name%$'\r'}"
  [[ -n "${hash:-}" && -n "${name:-}" ]] || continue
  if [[ ! "$hash" =~ ^[0-9a-f]{64}$ ]]; then
    echo "Invalid SHA-256 in $EXPECTED for $name" >&2
    exit 1
  fi
  EXPECTED_SHA256["$name"]="$hash"
done < "$EXPECTED"

# Base URLs for the GitHub release assets (primary) and official endpoint (fallback).
RELEASE_TAG="redox-toolchain-x86_64-unknown-redox-2026-11-21"
GITHUB_BASE="https://github.com/triontechno/trion/releases/download/$RELEASE_TAG"
OFFICIAL_BASES=(
  "https://static.redox-os.org/toolchain/$HOST_TARGET/$TARGET"
  "https://static.redox-os.org/toolchain/$TARGET"
)

for archive_name in gcc-install.tar.gz rust-install.tar.gz clang-install.tar.gz; do
  archive="$PREFIX/$archive_name"
  downloaded=0
  saw_forbidden=0

  # Try GitHub release first (fast, reliable from Actions runners).
  url="$GITHUB_BASE/$archive_name"
  echo "Fetching $url"
  if wget --tries=2 --timeout=60 --user-agent="Wget/1.21.3" \
      --output-document="$archive.partial" "$url" 2>"$response_log"; then
    downloaded=1
  else
    cat "$response_log"
    if grep -Eq 'HTTP request sent, awaiting response\.\.\. (403|404)' "$response_log"; then
      saw_forbidden=1
    else
      echo "Network or server error while fetching $archive_name from GitHub; trying official endpoint." >&2
    fi
    rm -f "$archive.partial"
  fi

  # Fall back to official Redox endpoint if GitHub failed with 403/404.
  if (( downloaded == 0 )); then
    for base in "${OFFICIAL_BASES[@]}"; do
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
  fi

  if (( downloaded == 0 )); then
    if (( saw_forbidden == 1 )); then
      use_source_fallback
    fi
    echo "No Redox prefix archive found for $TARGET/$archive_name." >&2
    exit 1
  fi

  # Verify gzip integrity and archive contents.
  gzip -t "$archive.partial"
  listing="$archive.partial.list"
  tar -tzf "$archive.partial" > "$listing"
  test -s "$listing"
  if grep -Eq '(^/|(^|/)\.\.(/|$))' "$listing"; then
    echo "Unsafe path in $archive_name" >&2
    exit 1
  fi

  # Verify SHA-256 against pinned expected value.
  actual="$(sha256sum "$archive.partial" | cut -d' ' -f1)"
  expected="${EXPECTED_SHA256[$archive_name]:-}"
  if [[ -z "$expected" ]]; then
    echo "No expected SHA-256 for $archive_name in $EXPECTED" >&2
    exit 1
  fi
  if [[ "$actual" != "$expected" ]]; then
    echo "SHA-256 mismatch for $archive_name:" >&2
    echo "  expected: $expected" >&2
    echo "  actual:   $actual" >&2
    exit 1
  fi
  echo "  SHA-256 verified: $actual"

  mv "$archive.partial" "$archive"
  rm -f "$listing"
  printf '%s  %s\n' "$actual" "build/prefix/$TARGET/$archive_name" >> "$manifest_tmp"
done

mv "$manifest_tmp" "$MANIFEST"
printf 'binary\n' > "$MODE_FILE"
(
  cd "$ROOT"
  sha256sum --check build/simplebuild/prefix-archives.sha256
)
fingerprint="$(printf '%s\n' "$(sha256sum "$MODE_FILE" | cut -d' ' -f1)" "$(sha256sum "$MANIFEST" | cut -d' ' -f1)" | sha256sum | cut -d' ' -f1)"
echo "Prefix mode: prebuilt binary (PREFIX_BINARY=1)"
echo "Redox prefix archive SHA-256 fingerprint: $fingerprint"
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  printf 'prefix_binary=1\n' >> "$GITHUB_OUTPUT"
  printf 'archive_sha256=%s\n' "$fingerprint" >> "$GITHUB_OUTPUT"
  printf 'target=%s\n' "$TARGET" >> "$GITHUB_OUTPUT"
  printf 'host_target=%s\n' "$HOST_TARGET" >> "$GITHUB_OUTPUT"
fi\r'}"
  [[ -n "${hash:-}" && -n "${name:-}" ]] || continue
  if [[ ! "$hash" =~ ^[0-9a-f]{64}$ ]]; then
    echo "Invalid SHA-256 in $EXPECTED for $name" >&2
    exit 1
  fi
  EXPECTED_SHA256["$name"]="$hash"
done < "$EXPECTED"

# Base URLs for the GitHub release assets (primary) and official endpoint (fallback).
RELEASE_TAG="redox-toolchain-x86_64-unknown-redox-2026-11-21"
GITHUB_BASE="https://github.com/triontechno/trion/releases/download/$RELEASE_TAG"
OFFICIAL_BASES=(
  "https://static.redox-os.org/toolchain/$HOST_TARGET/$TARGET"
  "https://static.redox-os.org/toolchain/$TARGET"
)

for archive_name in gcc-install.tar.gz rust-install.tar.gz clang-install.tar.gz; do
  archive="$PREFIX/$archive_name"
  downloaded=0
  saw_forbidden=0

  # Try GitHub release first (fast, reliable from Actions runners).
  url="$GITHUB_BASE/$archive_name"
  echo "Fetching $url"
  if wget --tries=2 --timeout=60 --user-agent="Wget/1.21.3" \
      --output-document="$archive.partial" "$url" 2>"$response_log"; then
    downloaded=1
  else
    cat "$response_log"
    if grep -Eq 'HTTP request sent, awaiting response\.\.\. (403|404)' "$response_log"; then
      saw_forbidden=1
    else
      echo "Network or server error while fetching $archive_name from GitHub; trying official endpoint." >&2
    fi
    rm -f "$archive.partial"
  fi

  # Fall back to official Redox endpoint if GitHub failed with 403/404.
  if (( downloaded == 0 )); then
    for base in "${OFFICIAL_BASES[@]}"; do
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
  fi

  if (( downloaded == 0 )); then
    if (( saw_forbidden == 1 )); then
      use_source_fallback
    fi
    echo "No Redox prefix archive found for $TARGET/$archive_name." >&2
    exit 1
  fi

  # Verify gzip integrity and archive contents.
  gzip -t "$archive.partial"
  listing="$archive.partial.list"
  tar -tzf "$archive.partial" > "$listing"
  test -s "$listing"
  if grep -Eq '(^/|(^|/)\.\.(/|$))' "$listing"; then
    echo "Unsafe path in $archive_name" >&2
    exit 1
  fi

  # Verify SHA-256 against pinned expected value.
  actual="$(sha256sum "$archive.partial" | cut -d' ' -f1)"
  expected="${EXPECTED_SHA256[$archive_name]:-}"
  if [[ -z "$expected" ]]; then
    echo "No expected SHA-256 for $archive_name in $EXPECTED" >&2
    exit 1
  fi
  if [[ "$actual" != "$expected" ]]; then
    echo "SHA-256 mismatch for $archive_name:" >&2
    echo "  expected: $expected" >&2
    echo "  actual:   $actual" >&2
    exit 1
  fi
  echo "  SHA-256 verified: $actual"

  mv "$archive.partial" "$archive"
  rm -f "$listing"
  printf '%s  %s\n' "$actual" "build/prefix/$TARGET/$archive_name" >> "$manifest_tmp"
done

mv "$manifest_tmp" "$MANIFEST"
printf 'binary\n' > "$MODE_FILE"
(
  cd "$ROOT"
  sha256sum --check build/simplebuild/prefix-archives.sha256
)
fingerprint="$(printf '%s\n' "$(sha256sum "$MODE_FILE" | cut -d' ' -f1)" "$(sha256sum "$MANIFEST" | cut -d' ' -f1)" | sha256sum | cut -d' ' -f1)"
echo "Prefix mode: prebuilt binary (PREFIX_BINARY=1)"
echo "Redox prefix archive SHA-256 fingerprint: $fingerprint"
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  printf 'prefix_binary=1\n' >> "$GITHUB_OUTPUT"
  printf 'archive_sha256=%s\n' "$fingerprint" >> "$GITHUB_OUTPUT"
  printf 'target=%s\n' "$TARGET" >> "$GITHUB_OUTPUT"
  printf 'host_target=%s\n' "$HOST_TARGET" >> "$GITHUB_OUTPUT"
fi