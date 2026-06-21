#!/usr/bin/env bash
# device_info.sh -- Print hardware and software information from a connected device
# Usage: ./device_info.sh [device_serial]
set -euo pipefail
DEVICE="${1:-}"
ADB_CMD="adb${DEVICE:+ -s $DEVICE}"

shell() { $ADB_CMD shell "$@" 2>/dev/null || echo "(unavailable)"; }

echo "==================================================="
echo "  MonoOS Device Information"
echo "==================================================="
echo ""
echo "-- Software --"
printf "  MonoOS version  : "; shell getprop ro.monoos.version
printf "  Build number   : "; shell getprop ro.monoos.build.id
printf "  Kernel         : "; shell uname -r
printf "  Security patch : "; shell getprop ro.build.version.security_patch
echo ""
echo "-- Hardware --"
printf "  Model          : "; shell getprop ro.product.model
printf "  Manufacturer   : "; shell getprop ro.product.manufacturer
printf "  CPU ABI        : "; shell getprop ro.product.cpu.abi
printf "  CPU cores      : "; shell nproc
echo ""
echo "-- Storage --"
$ADB_CMD shell df -h /data 2>/dev/null | tail -1 | \
  awk '{printf "  /data: total=%s used=%s free=%s\n", $2, $3, $4}' || echo "  (unavailable)"
echo ""
echo "-- Battery --"
$ADB_CMD shell dumpsys battery 2>/dev/null | \
  grep -E "level|status|health" | sed "s/^/  /" || echo "  (unavailable)"
echo ""
echo "-- Memory --"
$ADB_CMD shell cat /proc/meminfo 2>/dev/null | \
  grep -E "MemTotal|MemAvailable|SwapTotal" | sed "s/^/  /" || echo "  (unavailable)"
echo "==================================================="
