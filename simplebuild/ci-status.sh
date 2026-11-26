#!/usr/bin/env bash
# Run a CI command quietly and report only plain-English warnings or results.
set -Eeuo pipefail

usage() {
  echo "Usage: ci-status.sh --success 'Complete: ...' -- command [args...]" >&2
  exit 2
}
[[ "${1:-}" == "--success" && -n "${2:-}" && "${3:-}" == "--" && ${4-} ]] || usage
success_message="$2"
shift 3
log_file="$(mktemp)"
trap 'rm -f "$log_file"' EXIT

if "$@" >"$log_file" 2>&1; then
  command_status=0
else
  command_status=$?
fi

# Report warnings even if the command later fails.
grep -Ei '(^|[[:space:]])(warning:|warning\[|WARN:)' "$log_file" |
  sed -E 's/^[[:space:]]+//' |
  sort -u |
  while IFS= read -r line; do
    printf 'WARNING: %s\n' "$line"
    printf 'WARNING: %s\n' "$line" >> "${GITHUB_STEP_SUMMARY:-/dev/null}"
  done || true

if [[ "$command_status" -eq 0 ]]; then
  printf '%s\n' "$success_message"
  printf '%s\n' "$success_message" >> "${GITHUB_STEP_SUMMARY:-/dev/null}"
else
  reason="$(grep -Ei -m 1 '(^|[[:space:]])(error:|error\[|fatal error:|failed:|could not|cannot |unable to )' "$log_file" || true)"
  if [[ -z "$reason" ]]; then
    reason="$(tail -n 1 "$log_file" | tr -d '\r' | cut -c1-240)"
  fi
  [[ -n "$reason" ]] || reason="the command exited with code $command_status and gave no further details"
  printf 'ERROR: Could not complete the task because %s\n' "$reason"
  {
    printf 'ERROR: Could not complete the task because %s\n' "$reason"
    printf '\n<details><summary>Relevant diagnostic details</summary>\n\n'
    grep -Ei 'error:|error\[|fatal error:|failed:|could not|cannot |unable to ' "$log_file" | tail -n 12 || tail -n 12 "$log_file"
    printf '\n</details>\n'
  } >> "${GITHUB_STEP_SUMMARY:-/dev/null}"
  exit "$command_status"
fi
