#!/usr/bin/env bash
# SimpleBuild is a small, deterministic front-end to the repository's canonical
# Redox Make build system. It does not duplicate Cookbook or image-generation logic.
set -Eeuo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="$ROOT/build"
MAKE_BIN="${MAKE:-make}"
ARCH="${ARCH:-x86_64}"
CONFIG_NAME="${CONFIG_NAME:-desktop}"
BOARD="${BOARD:-}"
CHECK_ONLY=0
RUN_QEMU=0

usage() {
  cat <<'EOF'
SimpleBuild — TRION OS image builder

Usage:
  simplebuild/build.sh [options]

Options:
  --arch ARCH       Target architecture (x86_64, aarch64, i586, riscv64gc)
  --config NAME     Filesystem configuration (default: desktop)
  --board NAME      Optional board variant
  --check           Validate configuration and host build tools only
  --qemu            Launch the generated live ISO in QEMU after validation
  -h, --help        Show this help

The image build is delegated to build/Makefile. SimpleBuild does not maintain
a second compiler, package manager, or image-generation implementation.
EOF
}

fail() { printf 'SimpleBuild: ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf 'SimpleBuild: %s\n' "$*"; }

while (($#)); do
  case "$1" in
    --arch) (($# >= 2)) || fail "--arch requires a value"; ARCH="$2"; shift 2 ;;
    --config) (($# >= 2)) || fail "--config requires a value"; CONFIG_NAME="$2"; shift 2 ;;
    --board) (($# >= 2)) || fail "--board requires a value"; BOARD="$2"; shift 2 ;;
    --check) CHECK_ONLY=1; shift ;;
    --qemu) RUN_QEMU=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fail "unknown option: $1 (use --help)" ;;
  esac
done

case "$ARCH" in
  x86_64|aarch64|i586|riscv64gc) ;;
  *) fail "unsupported architecture '$ARCH'" ;;
esac
[[ "$CONFIG_NAME" =~ ^[A-Za-z0-9._-]+$ ]] || fail "invalid config name"
[[ -z "$BOARD" || "$BOARD" =~ ^[A-Za-z0-9._-]+$ ]] || fail "invalid board name"
[[ -f "$BUILD_DIR/Makefile" ]] || fail "build/Makefile is missing"
command -v "$MAKE_BIN" >/dev/null 2>&1 || fail "GNU make is required"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"

CONFIG_ROOT="$BUILD_DIR/config"
CONFIG_FILE=""
if [[ -n "$BOARD" && -f "$CONFIG_ROOT/$ARCH/$BOARD/$CONFIG_NAME.toml" ]]; then
  CONFIG_FILE="$CONFIG_ROOT/$ARCH/$BOARD/$CONFIG_NAME.toml"
elif [[ -f "$CONFIG_ROOT/$ARCH/$CONFIG_NAME.toml" ]]; then
  CONFIG_FILE="$CONFIG_ROOT/$ARCH/$CONFIG_NAME.toml"
elif [[ -f "$CONFIG_ROOT/$CONFIG_NAME.toml" ]]; then
  CONFIG_FILE="$CONFIG_ROOT/$CONFIG_NAME.toml"
else
  fail "no filesystem config found for ARCH=$ARCH BOARD=$BOARD CONFIG_NAME=$CONFIG_NAME"
fi

OUTPUT_DIR="$BUILD_DIR/build/$ARCH/$CONFIG_NAME"
IMAGE="$OUTPUT_DIR/harddrive.img"
ISO="$OUTPUT_DIR/redox-live.iso"

log "configuration: ${CONFIG_FILE#"$ROOT/"}"
log "backend: build/Makefile (canonical Redox build system)"

if ((CHECK_ONLY)); then
  log "configuration and required host tools are valid"
  exit 0
fi

make_args=(-C "$BUILD_DIR" "ARCH=$ARCH" "CONFIG_NAME=$CONFIG_NAME")
if [[ -n "$BOARD" ]]; then make_args+=("BOARD=$BOARD"); fi
log "building disk image and live ISO"

# Redox's upstream Rust recipes may emit this forward-compatibility lint for
# public async traits. Keep other Rust warnings enabled; suppress only this lint.
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-A async_fn_in_trait"

"$MAKE_BIN" "${make_args[@]}" images

[[ -s "$IMAGE" ]] || fail "disk image missing or empty: $IMAGE"
[[ -s "$ISO" ]] || fail "live ISO missing or empty: $ISO"
(
  cd "$OUTPUT_DIR"
  sha256sum harddrive.img redox-live.iso > SHA256SUMS
  sha256sum --check SHA256SUMS
)

log "both images generated and SHA-256 verified"
log "disk image: $IMAGE"
log "live ISO: $ISO"

if ((RUN_QEMU)); then
  command -v qemu-system-x86_64 >/dev/null 2>&1 || fail "qemu-system-x86_64 is required for --qemu"
  [[ "$ARCH" == "x86_64" ]] || fail "--qemu currently supports x86_64 only"
  "$MAKE_BIN" "${make_args[@]}" qemu live=yes
fi
