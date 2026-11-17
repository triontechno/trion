#!/usr/bin/env bash
# Reliable entry point for producing TRION's bootable disk image and live ISO.
set -Eeuo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="$ROOT/build"
ARCH="${ARCH:-x86_64}"
CONFIG_NAME="${CONFIG_NAME:-desktop}"
OUTPUT_DIR="$BUILD_DIR/build/$ARCH/$CONFIG_NAME"
fail() { printf 'TRION image build failed: %s\n' "$*" >&2; exit 1; }
command -v make >/dev/null 2>&1 || fail "GNU make is required"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"
[[ -f "$BUILD_DIR/Makefile" ]] || fail "build/Makefile not found"
[[ -f "$BUILD_DIR/config/$ARCH/$CONFIG_NAME.toml" || -f "$BUILD_DIR/config/$CONFIG_NAME.toml" ]] || fail "No filesystem configuration found for ARCH=$ARCH CONFIG_NAME=$CONFIG_NAME"
printf '==> Building TRION artifacts (ARCH=%s, CONFIG_NAME=%s)\n' "$ARCH" "$CONFIG_NAME"
make -C "$BUILD_DIR" ARCH="$ARCH" CONFIG_NAME="$CONFIG_NAME" images
IMG="$OUTPUT_DIR/harddrive.img"
ISO="$OUTPUT_DIR/redox-live.iso"
[[ -s "$IMG" ]] || fail "disk image missing or empty: $IMG"
[[ -s "$ISO" ]] || fail "live ISO missing or empty: $ISO"
if command -v file >/dev/null 2>&1; then file "$IMG" "$ISO"; fi
printf '\n==> SHA-256 checksums\n'
(cd "$OUTPUT_DIR" && sha256sum harddrive.img redox-live.iso | tee SHA256SUMS)
printf '\nTRION image build and output validation succeeded.\n'
printf 'Disk image: %s\nLive ISO:   %s\n' "$IMG" "$ISO"
