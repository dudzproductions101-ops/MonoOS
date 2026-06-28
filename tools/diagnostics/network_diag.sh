#!/usr/bin/env bash
# network_diag.sh -- MonoOS network diagnostics.
#
# Tests: interface enumeration, DNS resolution, HTTP connectivity,
#        latency to the default gateway, and /proc/monoos/net_stats.
#
# Usage: ./network_diag.sh [--json] [--dns <server>] [--timeout <secs>]

set -euo pipefail
JSON=0; DNS_SERVER="1.1.1.1"; TIMEOUT=4
while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)    JSON=1 ;;
    --dns)     DNS_SERVER="$2"; shift ;;
    --timeout) TIMEOUT="$2";    shift ;;
  esac
  shift 2>/dev/null || break
done

PASS=0; FAIL=0
results=()

check() {
  local label="$1" ok="$2" detail="$3"
  results+=("$label|$ok|$detail")
  if [[ "$ok" == "PASS" ]]; then ((PASS++)); else ((FAIL++)); fi
}

# ── Interface enumeration ─────────────────────────────────────────────────────
IF_LIST=$(ip -br addr 2>/dev/null | grep UP | awk "{print \$1}" | tr "\n" " " || echo "none")
check "interfaces" "$([ -n "$IF_LIST" ] && echo PASS || echo FAIL)" "$IF_LIST"

# ── Default gateway ───────────────────────────────────────────────────────────
GW=$(ip route show default 2>/dev/null | awk "/default via/ {print \$3; exit}" || echo "")
if [[ -n "$GW" ]]; then
  if ping -c 2 -W "$TIMEOUT" "$GW" &>/dev/null; then
    LATENCY=$(ping -c 4 -W "$TIMEOUT" "$GW" 2>/dev/null | tail -1 | awk -F"/" "{print \$5}" || echo "?")
    check "gateway_ping" "PASS" "gateway $GW  avg ${LATENCY}ms"
  else
    check "gateway_ping" "FAIL" "gateway $GW unreachable"
  fi
else
  check "gateway_ping" "FAIL" "no default gateway found"
fi

# ── DNS resolution ─────────────────────────────────────────────────────────────
if host -W "$TIMEOUT" monoos.io "$DNS_SERVER" &>/dev/null; then
  check "dns_resolve" "PASS" "resolved monoos.io via $DNS_SERVER"
else
  check "dns_resolve" "FAIL" "DNS resolution failed (server: $DNS_SERVER)"
fi

# ── HTTP connectivity ─────────────────────────────────────────────────────────
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" --max-time "$TIMEOUT"              https://connectivity.monoos.io/probe 2>/dev/null || echo "0")
if [[ "$HTTP_CODE" == "204" || "$HTTP_CODE" == "200" ]]; then
  check "http_connectivity" "PASS" "HTTP ${HTTP_CODE} from connectivity.monoos.io"
elif [[ "$HTTP_CODE" == "0" ]]; then
  check "http_connectivity" "FAIL" "curl timed out or network unreachable"
else
  check "http_connectivity" "FAIL" "unexpected HTTP $HTTP_CODE"
fi

# ── MonoOS network stats ───────────────────────────────────────────────────────
if [[ -f /proc/monoos/net_stats ]]; then
  BLOCKED=$(grep -m1 "packets_blocked" /proc/monoos/net_stats | awk "{print \$2}" || echo "?")
  check "monoos_net_stats" "PASS" "packets_blocked=$BLOCKED"
else
  check "monoos_net_stats" "FAIL" "/proc/monoos/net_stats not found"
fi

# ── Output ────────────────────────────────────────────────────────────────────
if [[ "$JSON" -eq 1 ]]; then
  echo "["
  for i in "${!results[@]}"; do
    IFS="|" read -r label status detail <<< "${results[$i]}"
    comma=$( [[ $i -lt $(( ${#results[@]} - 1 )) ]] && echo "," || echo "" )
    echo "  {"label": "$label", "status": "$status", "detail": "$detail"}${comma}"
  done
  echo "]"
else
  printf "\nMonoOS Network Diagnostics\n%s\n" "$(printf '%0.s-' {1..55})"
  for item in "${results[@]}"; do
    IFS="|" read -r label status detail <<< "$item"
    printf "  %-22s [%-4s]  %s\n" "$label" "$status" "$detail"
  done
  printf "%s\nPass: %d  Fail: %d\n\n" "$(printf '%0.s-' {1..55})" "$PASS" "$FAIL"
fi

exit $(( FAIL > 0 ? 1 : 0 ))
