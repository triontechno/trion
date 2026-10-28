#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

UPSTREAM_URL="${TRION_BUILD_UPSTREAM_URL:-https://github.com/redox-os/redox.git}"
UPSTREAM_REF="${TRION_BUILD_UPSTREAM_REF:-master}"
DEST="$ROOT/build"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if ! command -v git >/dev/null 2>&1; then
  echo "git is required to vendor the Redox build system." >&2
  exit 1
fi

echo "Cloning build-system mirror: $UPSTREAM_URL ($UPSTREAM_REF)"
git clone --depth 1 --branch "$UPSTREAM_REF" "$UPSTREAM_URL" "$TMP/upstream"

# Copy the upstream build system as normal files. Do not copy Git metadata,
# GitLab metadata, or the upstream Rust source directory under its old name.
mkdir -p "$DEST"
while IFS= read -r -d '' entry; do
  name="$(basename "$entry")"
  case "$name" in
    .git|.gitmodules|src) continue ;;
  esac
  cp -a "$entry" "$DEST/"
done < <(find "$TMP/upstream" -mindepth 1 -maxdepth 1 -print0)

if [[ ! -d "$TMP/upstream/src" ]]; then
  echo "Upstream checkout has no src/ directory; refusing to continue." >&2
  exit 1
fi
mkdir -p "$DEST/code"
cp -a "$TMP/upstream/src/." "$DEST/code/"

if [[ ! -f "$DEST/Cargo.toml" ]]; then
  echo "Upstream checkout has no Cargo.toml; refusing to continue." >&2
  exit 1
fi

# The upstream crate uses src/ paths in explicit target declarations.
# In TRION, that crate source is deliberately located at build/code/.
sed -i 's#src/#code/#g' "$DEST/Cargo.toml"

cat > "$DEST/UPSTREAM.lock" <<EOF
repository=$UPSTREAM_URL
ref=$UPSTREAM_REF
commit=$(git -C "$TMP/upstream" rev-parse HEAD)
source_directory=code
EOF

echo "Vendored build system into build/; Rust crate source is in build/code/."
echo "Review the diff, then run the build and image tests before treating it as ready."
