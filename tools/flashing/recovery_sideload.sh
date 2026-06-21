#!/usr/bin/env bash
# recovery_sideload.sh -- Push an OTA package to a device in MonoOS recovery
# mode via "adb sideload", mirroring the SideloadManager::AdbSideload path
# implemented in boot/recovery/sideload_manager.rs.
#
# Usage:
#   ./recovery_sideload.sh <ota_package.zip>
#   ./recovery_sideload.sh --wait <ota_package.zip>   Wait for device first
#
# Requires: adb in PATH, device already booted into recovery with
#           "Apply update from ADB" selected (or boot command set to
#           apply-update via the BootStateBlock recovery_arg).

set -euo pipefail

WAIT=0
PACKAGE=""

for arg in "$@"; do
  case "$arg" in
    --wait) WAIT=1 ;;
    *)      PACKAGE="$arg" ;;
  esac
done

if [[ -z "$PACKAGE" ]]; then
  echo "Usage: $(basename "$0") [--wait] <ota_package.zip>" >&2
  exit 1
fi

if [[ ! -f "$PACKAGE" ]]; then
  echo "ERROR: package not found: $PACKAGE" >&2
  exit 1
fi

if ! command -v adb &>/dev/null; then
  echo "ERROR: adb not found in PATH. Install Android platform-tools." >&2
  exit 1
fi

SIZE_MB=$(du -m "$PACKAGE" | cut -f1)
echo "OTA package : $PACKAGE (${SIZE_MB} MB)"

if [[ "$WAIT" -eq 1 ]]; then
  echo "Waiting for device in recovery sideload mode..."
  adb wait-for-sideload
fi

# Verify the device reports a sideload-capable adbd (recovery's adbd binds
# the "sideload:<size>" service rather than the normal one).
STATE=$(adb get-state 2>/dev/null || echo "unknown")
echo "adb device state: $STATE"

if [[ "$STATE" != "sideload" && "$STATE" != "recovery" ]]; then
  echo "WARNING: device does not report 'sideload' or 'recovery' state."
  echo "         Ensure the device shows 'Apply update from ADB' in the"
  echo "         recovery menu before continuing."
fi

echo "Starting sideload transfer..."
START_TS=$(date +%s)

if adb sideload "$PACKAGE"; then
  END_TS=$(date +%s)
  ELAPSED=$(( END_TS - START_TS ))
  echo ""
  echo "Sideload completed in ${ELAPSED}s."
  echo "The device will apply the update and reboot automatically."
  exit 0
else
  echo ""
  echo "Sideload FAILED. Check the device screen for the recovery error message." >&2
  exit 1
fi
