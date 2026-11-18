#!/usr/bin/env bash
# Stable cache fingerprint for compiled Cookbook prerequisites.
# Only tracked source/configuration inputs are hashed; generated build outputs,
# cache archives and manifests can never change the key between restore/save.
set -Eeuo pipefail

if [[ -z "${GITHUB_OUTPUT:-}" ]]; then
  echo "GITHUB_OUTPUT must be set by the workflow." >&2
  exit 2
fi

mapfile -d '' tracked_files < <(git ls-files -z)
inputs=()
for path in "${tracked_files[@]}"; do
  case "$path" in
    build/Makefile|build/mk/*|build/podman/*|build/podman_bootstrap.sh|build/recipes/*|build/config/*|build/redox.ipxe|config/*|filesystem/*|kernel/*|src/*|recipes/*|podman/*|cookbook.toml|Cargo.lock|*/Cargo.lock|Cargo.toml|*/Cargo.toml)
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
    sha256sum < "$path" | cut -d' ' -f1
  done | sha256sum | cut -d' ' -f1
)
key="simplebuild-prerequisites-${RUNNER_OS:-Linux}-$input_hash"
printf 'key=%s\n' "$key" >> "$GITHUB_OUTPUT"
echo "Prerequisite cache key: $key"
