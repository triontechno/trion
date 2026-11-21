#!/usr/bin/env bash
# Verify the Redox prefix archives recorded by fetch-prefix.sh.
#
# The manifest lists paths relative to the repository root
# (build/prefix/<target>/<archive>), so verification MUST run from the root.
# Every workflow and local run goes through this one script so the check cannot
# drift out of sync again.
#
# Usage: simplebuild/verify-prefix.sh [expected-fingerprint]
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MODE_FILE="build/simplebuild/prefix-mode"
MANIFEST="build/simplebuild/prefix-archives.sha256"
EXPECTED_FINGERPRINT="${1:-}"

[[ -f "$MODE_FILE" ]] || { echo "Missing $MODE_FILE; run simplebuild/fetch-prefix.sh first." >&2; exit 1; }
[[ -f "$MANIFEST" ]] || { echo "Missing $MANIFEST; run simplebuild/fetch-prefix.sh first." >&2; exit 1; }

mode="$(cat "$MODE_FILE")"
case "$mode" in
  binary) sha256sum --check "$MANIFEST" ;;
  source) test ! -s "$MANIFEST" ;;
  *) echo "Invalid prefix mode: $mode" >&2; exit 1 ;;
esac

fingerprint="$(printf '%s\n' \
  "$(sha256sum "$MODE_FILE" | cut -d' ' -f1)" \
  "$(sha256sum "$MANIFEST" | cut -d' ' -f1)" | sha256sum | cut -d' ' -f1)"
echo "Prefix mode: $mode"
echo "Prefix fingerprint: $fingerprint"

if [[ -n "$EXPECTED_FINGERPRINT" && "$fingerprint" != "$EXPECTED_FINGERPRINT" ]]; then
  echo "Prefix fingerprint mismatch (expected $EXPECTED_FINGERPRINT)." >&2
  exit 1
fi
