#!/usr/bin/env bash
# Boot the already-built images directly. This script must never invoke Make or Cookbook.
set -Eeuo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <harddrive.img> <redox-live.iso>" >&2
  exit 2
fi

disk_image=$1
live_iso=$2
qemu=${QEMU_SYSTEM_X86_64:-qemu-system-x86_64}

command -v "$qemu" >/dev/null 2>&1 || {
  echo "QEMU executable not found: $qemu" >&2
  exit 1
}
[[ -s "$disk_image" ]] || { echo "Missing or empty disk image: $disk_image" >&2; exit 1; }
[[ -s "$live_iso" ]] || { echo "Missing or empty live ISO: $live_iso" >&2; exit 1; }

run_guest() {
  local label=$1
  shift
  local log
  local status
  log=$(mktemp)
  echo "Boot-testing $label directly with QEMU..."
  set +e
  timeout --signal=TERM --kill-after=10s 120s "$qemu" \
    -machine q35 -cpu core2duo -smp 4 -m 2048 \
    -display none -vga none -net none \
    -serial stdio -monitor none -no-reboot \
    "$@" >"$log" 2>&1
  status=$?
  set -e

  if ! grep -Fq 'Redox OS starting...' "$log"; then
    echo "QEMU did not reach the Redox kernel startup marker for $label." >&2
    cat "$log" >&2
    rm -f "$log"
    return 1
  fi
  if [[ "$status" -ne 0 && "$status" -ne 124 ]]; then
    echo "QEMU exited unexpectedly for $label (status $status)." >&2
    cat "$log" >&2
    rm -f "$log"
    return "$status"
  fi
  for marker in \
    'capability scheme registered at /scheme/trion' \
    'capability reports validated from /scheme/trion'; do
    if ! grep -Fq "$marker" "$log"; then
      echo "Missing guest validation marker for $label: $marker" >&2
      cat "$log" >&2
      rm -f "$log"
      return 1
    fi
  done
  echo "$label: kernel startup and TRION capability checks passed."
  rm -f "$log"
}

run_guest "harddrive.img" \
  -drive "file=$disk_image,format=raw,if=none,id=triondisk" \
  -device nvme,drive=triondisk,serial=TRION_DISK \
  -boot order=c

ovmf_code=$(find /usr/share/OVMF/OVMF_CODE.fd /usr/share/ovmf/OVMF_CODE.fd /usr/share/qemu/OVMF_CODE.fd -type f -print -quit 2>/dev/null || true)
ovmf_vars=$(find /usr/share/OVMF/OVMF_VARS.fd /usr/share/ovmf/OVMF_VARS.fd /usr/share/qemu/OVMF_VARS.fd -type f -print -quit 2>/dev/null || true)
if [[ -z "$ovmf_code" || -z "$ovmf_vars" ]]; then
  echo "OVMF UEFI firmware is required to boot-test the live ISO (install the ovmf package)." >&2
  exit 1
fi
vars_copy=$(mktemp)
trap 'rm -f "$vars_copy"' EXIT
cp "$ovmf_vars" "$vars_copy"

run_guest "redox-live.iso" \
  -drive "if=pflash,format=raw,unit=0,file=$ovmf_code,readonly=on" \
  -drive "if=pflash,format=raw,unit=1,file=$vars_copy" \
  -device ich9-ahci,id=ahci \
  -drive "file=$live_iso,format=raw,media=cdrom,readonly=on,if=none,id=trioncd" \
  -device ide-cd,drive=trioncd,bus=ahci.0 \
  -boot order=d

echo "Both generated images passed direct QEMU boot validation."
