#!/usr/bin/env bash
# flash_device.sh – Flash OneOS images to a connected device via fastboot
#
# Usage:
#   ./build/scripts/flash_device.sh [--slot a|b] [--wipe-userdata]
#
# Requires:
#   - fastboot (from Android SDK platform-tools)
#   - A device in fastboot / bootloader mode

set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IMAGES="$ROOT/build/release"
FASTBOOT="${FASTBOOT:-fastboot}"

SLOT="a"
WIPE_USERDATA=0

for arg in "$@"; do
  case "$arg" in
    --slot=a|--slot=A) SLOT="a" ;;
    --slot=b|--slot=B) SLOT="b" ;;
    --wipe-userdata)   WIPE_USERDATA=1 ;;
  esac
done

echo "[flash] Waiting for device in fastboot mode..."
$FASTBOOT wait-for-device

echo "[flash] Flashing bootloader..."
$FASTBOOT flash bootloader "$IMAGES/bootloader.bin"

echo "[flash] Rebooting into bootloader to apply new bootloader..."
$FASTBOOT reboot-bootloader
sleep 3

echo "[flash] Flashing slot ${SLOT}..."
$FASTBOOT --set-active="$SLOT"
$FASTBOOT flash "boot_${SLOT}"   "$IMAGES/boot.img"   2>/dev/null || true
$FASTBOOT flash "system_${SLOT}" "$IMAGES/system.img" 2>/dev/null || true
$FASTBOOT flash "vendor_${SLOT}" "$IMAGES/vendor.img" 2>/dev/null || true

if [[ "$WIPE_USERDATA" -eq 1 ]]; then
  echo "[flash] Wiping userdata..."
  $FASTBOOT -w
fi

echo "[flash] Rebooting device..."
$FASTBOOT reboot

echo "[flash] Done."
