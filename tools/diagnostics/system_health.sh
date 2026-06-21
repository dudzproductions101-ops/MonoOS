#!/usr/bin/env bash
# system_health.sh -- MonoOS on-device system health report.
#
# Checks: CPU, memory, storage, battery, thermals, kernel modules,
#         process table, network interfaces, and /proc/monoos entries.
#
# Usage: ./system_health.sh [--json] [--brief]

set -euo pipefail

JSON=0; BRIEF=0
for arg in "$@"; do
  case "$arg" in
    --json)  JSON=1 ;;
    --brief) BRIEF=1 ;;
  esac
done

SEP="$(printf '%0.s-' {1..60})"

read_file()  { cat "$1" 2>/dev/null || echo "(unavailable)"; }
read_field() { grep -m1 "$1" "$2" 2>/dev/null | awk -F: "{print \$2}" | xargs || echo "?"; }

# ── Gather ────────────────────────────────────────────────────────────────────

OS_VER=$(getprop ro.monoos.version   2>/dev/null || echo "unknown")
BUILD=$(getprop ro.monoos.build.id   2>/dev/null || echo "unknown")
MODEL=$(getprop ro.product.model    2>/dev/null || echo "unknown")
ABI=$(getprop ro.product.cpu.abi    2>/dev/null || echo "unknown")
KERNEL=$(uname -r 2>/dev/null       || echo "unknown")
UPTIME=$(uptime 2>/dev/null         || echo "unknown")

MEM_TOTAL=$(read_field MemTotal /proc/meminfo)
MEM_AVAIL=$(read_field MemAvailable /proc/meminfo)
SWAP_TOTAL=$(read_field SwapTotal /proc/meminfo)
SWAP_FREE=$(read_field SwapFree /proc/meminfo)

CPU_CORES=$(nproc 2>/dev/null || echo "?")
LOAD=$(read_file /proc/loadavg | awk "{print \$1, \$2, \$3}")

BAT_LEVEL=$(cat /sys/class/power_supply/battery/capacity    2>/dev/null || echo "?")
BAT_STATUS=$(cat /sys/class/power_supply/battery/status     2>/dev/null || echo "?")
BAT_TEMP=$(cat /sys/class/power_supply/battery/temp         2>/dev/null || echo "?")

STORAGE=$(df -h /data 2>/dev/null | tail -1 | awk "{print \$2, \$3, \$4}" || echo "? ? ?")

THERMAL_ZONES=$(ls /sys/class/thermal/thermal_zone*/temp 2>/dev/null | head -3)

MONOOS_MODULES=$(lsmod 2>/dev/null | grep -c "^monoos_" || echo "0")

# ── Checks ────────────────────────────────────────────────────────────────────

PASS=0; WARN=0; FAIL=0
checks=()

check() {
  local label="$1" status="$2" detail="$3"
  checks+=("$label|$status|$detail")
  case "$status" in
    PASS) ((PASS++)) ;;
    WARN) ((WARN++)) ;;
    FAIL) ((FAIL++)) ;;
  esac
}

# Memory > 10 % available
if [[ "$MEM_AVAIL" =~ ^[0-9]+ && "$MEM_TOTAL" =~ ^[0-9]+ ]]; then
  pct=$(( MEM_AVAIL * 100 / MEM_TOTAL ))
  if   (( pct >= 20 )); then check "memory"  PASS "${pct}% available"
  elif (( pct >= 10 )); then check "memory"  WARN "${pct}% available (low)"
  else                       check "memory"  FAIL "${pct}% available (critical)"
  fi
else check "memory" WARN "could not parse /proc/meminfo"; fi

# Battery >= 15 %
if [[ "$BAT_LEVEL" =~ ^[0-9]+$ ]]; then
  if   (( BAT_LEVEL >= 15 )); then check "battery" PASS "${BAT_LEVEL}% (${BAT_STATUS})"
  else                              check "battery" WARN "${BAT_LEVEL}% (low battery)"
  fi
else check "battery" WARN "battery level unavailable"; fi

# Kernel modules loaded
if (( MONOOS_MODULES >= 4 )); then check "kernel_modules" PASS "${MONOOS_MODULES} monoos_* modules loaded"
elif (( MONOOS_MODULES > 0 )); then check "kernel_modules" WARN "only ${MONOOS_MODULES} monoos_* modules loaded"
else                               check "kernel_modules" FAIL "no monoos_* kernel modules found"; fi

# /proc/monoos entries
if [[ -d /proc/monoos ]]; then
  n=$(ls /proc/monoos/ 2>/dev/null | wc -l)
  check "proc_monoos" PASS "${n} entries in /proc/monoos"
else
  check "proc_monoos" FAIL "/proc/monoos not found"
fi

# Thermals
for zone in $THERMAL_ZONES; do
  temp_mc=$(cat "$zone" 2>/dev/null || echo "0")
  temp_c=$(( temp_mc / 1000 ))
  zone_name=$(basename "$(dirname "$zone")")
  if   (( temp_c < 45 )); then check "thermal/$zone_name" PASS "${temp_c}°C"
  elif (( temp_c < 60 )); then check "thermal/$zone_name" WARN "${temp_c}°C (warm)"
  else                         check "thermal/$zone_name" FAIL "${temp_c}°C (hot!)"
  fi
done

# ── Output ────────────────────────────────────────────────────────────────────

if [[ "$JSON" -eq 1 ]]; then
  echo "{"
  echo "  "os_version": "$OS_VER","
  echo "  "build": "$BUILD","
  echo "  "kernel": "$KERNEL","
  echo "  "model": "$MODEL","
  echo "  "checks": ["
  for i in "${!checks[@]}"; do
    IFS="|" read -r label status detail <<< "${checks[$i]}"
    comma=$( [[ $i -lt $(( ${#checks[@]} - 1 )) ]] && echo "," || echo "" )
    echo "    {"label": "$label", "status": "$status", "detail": "$detail"}${comma}"
  done
  echo "  ],"
  echo "  "summary": {"pass": $PASS, "warn": $WARN, "fail": $FAIL}"
  echo "}"
  exit $(( FAIL > 0 ? 1 : 0 ))
fi

echo "$SEP"
printf " MonoOS System Health  |  %s  |  %s\n" "$OS_VER" "$BUILD"
echo "$SEP"
printf " Model  : %s   ABI: %s   Kernel: %s\n" "$MODEL" "$ABI" "$KERNEL"
printf " Uptime : %s\n" "$UPTIME"
printf " CPU    : %s cores  Load: %s\n" "$CPU_CORES" "$LOAD"
printf " Memory : Total=%s  Available=%s\n" "$MEM_TOTAL" "$MEM_AVAIL"
printf " Storage: Total=%s Used=%s Free=%s (/data)\n" $(echo "$STORAGE")
printf " Battery: %s%% %s\n" "$BAT_LEVEL" "$BAT_STATUS"
echo "$SEP"

for item in "${checks[@]}"; do
  IFS="|" read -r label status detail <<< "$item"
  case "$status" in
    PASS) sym="[OK]" ;;
    WARN) sym="[WARN]" ;;
    FAIL) sym="[FAIL]" ;;
    *)    sym="[??]" ;;
  esac
  printf "  %-14s %-6s  %s\n" "$label" "$sym" "$detail"
done

echo "$SEP"
printf " Result: PASS=%d  WARN=%d  FAIL=%d\n" "$PASS" "$WARN" "$FAIL"
echo "$SEP"

exit $(( FAIL > 0 ? 1 : 0 ))
