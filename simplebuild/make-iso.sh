#!/usr/bin/env bash
# One-command TRION ISO build.
#
#   bash simplebuild/make-iso.sh                 # x86_64 desktop
#   bash simplebuild/make-iso.sh --config server # other config
#   bash simplebuild/make-iso.sh --qemu-test     # also boot-test the result
#
# Steps: fetch + verify the prebuilt prefix (when missing), check
# prerequisites, generate harddrive.img and redox-live.iso, verify SHA256SUMS.
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

ARCH="${ARCH:-x86_64}"
CONFIG="${CONFIG_NAME:-desktop}"
QEMU_TEST=0
FORCE=0
while (( $# )); do
  case "$1" in
    --arch) ARCH="$2"; shift 2 ;;
    --config) CONFIG="$2"; shift 2 ;;
    --qemu-test) QEMU_TEST=1; shift ;;
    --force) FORCE=1; shift ;;
    -h|--help) sed -n '2,10p' "$0"; exit 0 ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
  esac
done

export ARCH
if [[ ! -f build/simplebuild/prefix-mode ]]; then
  echo "==> Fetching Redox prefix"
  bash simplebuild/fetch-prefix.sh
fi
echo "==> Verifying prefix"
bash simplebuild/verify-prefix.sh

args=(--arch "$ARCH" --config "$CONFIG")
echo "==> Checking prerequisites"
bash simplebuild/build.sh "${args[@]}" --check

echo "==> Building images"
(( FORCE )) && args+=(--force)
bash simplebuild/build.sh "${args[@]}"

out="build/build/$ARCH/$CONFIG"
echo "==> Verifying checksums"
(cd "$out" && test -s harddrive.img && test -s redox-live.iso && sha256sum -c SHA256SUMS)

if (( QEMU_TEST )); then
  echo "==> Boot-testing in QEMU"
  bash simplebuild/qemu-test.sh "$out/harddrive.img" "$out/redox-live.iso"
fi

echo "ISO ready: $out/redox-live.iso"
