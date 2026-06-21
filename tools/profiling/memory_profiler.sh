#!/usr/bin/env bash
# memory_profiler.sh -- MonoOS per-process and system memory profiler.
#
# Reports RSS/PSS/USS breakdown from /proc/<pid>/smaps_rollup (or smaps
# as a fallback), plus system-wide memory pressure from the MonoOS
# kernel memory module (/proc/monoos/mm).
#
# Usage:
#   ./memory_profiler.sh --pid <pid>
#   ./memory_profiler.sh --package <name>
#   ./memory_profiler.sh --top [--count 15]
#   ./memory_profiler.sh --watch --pid <pid> [--interval 2]

set -euo pipefail

COUNT=15
INTERVAL=2
WATCH=0
PID=""
PACKAGE=""
MODE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pid)      PID="$2";      MODE="pid";     shift ;;
    --package)  PACKAGE="$2";  MODE="package"; shift ;;
    --top)      MODE="top" ;;
    --count)    COUNT="$2";    shift ;;
    --interval) INTERVAL="$2"; shift ;;
    --watch)    WATCH=1 ;;
    -h|--help)
      grep "^#" "$0" | sed "s/^# \\{0,1\\}//"
      exit 0
      ;;
  esac
  shift
done

resolve_pid_from_package() {
  local pkg="$1"
  local resolved
  resolved=$(pidof "$pkg" 2>/dev/null | awk "{print \$1}" || true)
  if [[ -z "$resolved" ]]; then
    for d in /proc/[0-9]*; do
      [[ -r "$d/cmdline" ]] || continue
      if grep -qa "$pkg" "$d/cmdline" 2>/dev/null; then
        resolved="${d#/proc/}"
        break
      fi
    done
  fi
  echo "$resolved"
}

human_kb() {
  local kb="$1"
  if (( kb >= 1048576 )); then
    awk -v k="$kb" 'BEGIN{printf "%.2f GB", k/1048576}'
  elif (( kb >= 1024 )); then
    awk -v k="$kb" 'BEGIN{printf "%.1f MB", k/1024}'
  else
    echo "${kb} KB"
  fi
}

report_pid() {
  local pid="$1"
  if [[ ! -d "/proc/$pid" ]]; then
    echo "ERROR: PID $pid not found." >&2
    return 1
  fi

  local comm
  comm=$(tr -d '\0' < "/proc/$pid/comm" 2>/dev/null || echo "?")

  echo ""
  echo "Memory profile: PID $pid ($comm)"
  echo "$(printf '%0.s-' {1..50})"

  local rollup="/proc/$pid/smaps_rollup"
  if [[ -r "$rollup" ]]; then
    local rss pss private_clean private_dirty shared_clean shared_dirty swap
    rss=$(awk '/^Rss:/{print $2}' "$rollup")
    pss=$(awk '/^Pss:/{print $2}' "$rollup")
    private_clean=$(awk '/^Private_Clean:/{print $2}' "$rollup")
    private_dirty=$(awk '/^Private_Dirty:/{print $2}' "$rollup")
    shared_clean=$(awk '/^Shared_Clean:/{print $2}' "$rollup")
    shared_dirty=$(awk '/^Shared_Dirty:/{print $2}' "$rollup")
    swap=$(awk '/^Swap:/{print $2}' "$rollup")

    printf "  RSS            : %s\n" "$(human_kb "${rss:-0}")"
    printf "  PSS            : %s\n" "$(human_kb "${pss:-0}")"
    printf "  Private Clean  : %s\n" "$(human_kb "${private_clean:-0}")"
    printf "  Private Dirty  : %s\n" "$(human_kb "${private_dirty:-0}")"
    printf "  Shared Clean   : %s\n" "$(human_kb "${shared_clean:-0}")"
    printf "  Shared Dirty   : %s\n" "$(human_kb "${shared_dirty:-0}")"
    printf "  Swap           : %s\n" "$(human_kb "${swap:-0}")"
    local uss=$(( ${private_clean:-0} + ${private_dirty:-0} ))
    printf "  USS (estimate) : %s\n" "$(human_kb "$uss")"
  elif [[ -r "/proc/$pid/status" ]]; then
    # Fallback when smaps_rollup is unavailable (older kernels).
    local vmrss
    vmrss=$(awk '/^VmRSS:/{print $2}' "/proc/$pid/status")
    printf "  RSS (VmRSS)    : %s   (smaps_rollup unavailable; limited detail)\n" "$(human_kb "${vmrss:-0}")"
  else
    echo "  Unable to read memory info for this process (permission denied?)."
    return 1
  fi
}

report_top() {
  echo ""
  echo "Top $COUNT processes by RSS"
  echo "$(printf '%0.s-' {1..60})"
  printf "%-10s %-24s %12s\n" "PID" "COMM" "RSS"

  for d in /proc/[0-9]*; do
    local pid="${d#/proc/}"
    [[ -r "$d/status" ]] || continue
    local rss comm
    rss=$(awk '/^VmRSS:/{print $2}' "$d/status" 2>/dev/null || echo 0)
    [[ -n "$rss" ]] || rss=0
    comm=$(tr -d '\0' < "$d/comm" 2>/dev/null || echo "?")
    echo "$rss|$pid|$comm"
  done | sort -t'|' -k1 -rn | head -n "$COUNT" | while IFS='|' read -r rss pid comm; do
    printf "%-10s %-24s %12s\n" "$pid" "$comm" "$(human_kb "$rss")"
  done
}

report_monoos_mm() {
  if [[ -r /proc/monoos/mm ]]; then
    echo ""
    echo "Kernel memory module (/proc/monoos/mm)"
    echo "$(printf '%0.s-' {1..40})"
    sed 's/^/  /' /proc/monoos/mm
  fi
}

run_once() {
  case "$MODE" in
    pid)
      report_pid "$PID"
      report_monoos_mm
      ;;
    package)
      local resolved
      resolved=$(resolve_pid_from_package "$PACKAGE")
      if [[ -z "$resolved" ]]; then
        echo "ERROR: no running process found for package '$PACKAGE'" >&2
        exit 1
      fi
      echo "Resolved $PACKAGE -> PID $resolved"
      report_pid "$resolved"
      report_monoos_mm
      ;;
    top)
      report_top
      report_monoos_mm
      ;;
    *)
      echo "Usage: $(basename "$0") --pid <pid> | --package <name> | --top [--count N]" >&2
      exit 1
      ;;
  esac
}

if [[ "$WATCH" -eq 1 ]]; then
  echo "Watching every ${INTERVAL}s. Press Ctrl-C to stop."
  while true; do
    clear
    run_once
    sleep "$INTERVAL"
  done
else
  run_once
fi
