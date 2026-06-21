#!/usr/bin/env bash
# monoos_strace.sh -- Lightweight syscall trace wrapper for MonoOS.
#
# Wraps the standard strace(1) with MonoOS-appropriate defaults:
#   - Filters to MonoOS-specific syscall numbers (400-402).
#   - Annotates syscall numbers with their MonoOS names.
#   - Optionally attaches to a running process by package name.
#
# Usage:
#   monoos_strace.sh -p <pid>                Trace an existing PID
#   monoos_strace.sh -pkg <package>          Look up PID by package name, then trace
#   monoos_strace.sh -- <cmd> [args...]      Trace a new command
#   monoos_strace.sh -o <file> -- <cmd>      Write output to file
#   monoos_strace.sh -e <expr> -- <cmd>      Custom strace expression

set -euo pipefail

MONOOS_SYSCALLS="400,401,402"   # perm_check, perm_set, privacy_stat
STRACE="${STRACE:-strace}"

usage() {
  cat <<EOF
Usage:
  $(basename "$0") -p <pid>              Trace existing process
  $(basename "$0") -pkg <package>        Trace by package name
  $(basename "$0") -o <file> -- <cmd>    Trace command, write to file
  $(basename "$0") -- <cmd> [args]       Trace command to stdout
  $(basename "$0") -e net -- <cmd>       Trace network syscalls
  $(basename "$0") -e monoos -- <cmd>     Trace MonoOS-specific syscalls only

Examples:
  monoos_strace.sh -pkg com.example.camera
  monoos_strace.sh -- ls /data
EOF
  exit 1
}

# Check strace is available
if ! command -v "$STRACE" &>/dev/null; then
  echo "Error: strace not found. Install via: apt install strace" >&2
  exit 1
fi

# Default options
PID=""
PKG=""
OUTPUT=""
EXPR=""
CMD_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -p)        PID="$2";    shift 2 ;;
    -pkg)      PKG="$2";    shift 2 ;;
    -o)        OUTPUT="$2"; shift 2 ;;
    -e)        EXPR="$2";   shift 2 ;;
    -h|--help) usage ;;
    --)        shift; CMD_ARGS=("$@"); break ;;
    *)         CMD_ARGS=("$@"); break ;;
  esac
done

# Resolve package name to PID
if [[ -n "$PKG" ]]; then
  PID=$(adb shell pidof "$PKG" 2>/dev/null | tr -d '\r' || true)
  if [[ -z "$PID" ]]; then
    echo "Error: process for package '$PKG' not found." >&2
    exit 1
  fi
  echo "Attaching to $PKG (PID $PID)..."
fi

# Build strace expression
if [[ -z "$EXPR" ]]; then
  # Default: show all syscalls + always show MonoOS-specific ones
  EXPR_FLAG="-e trace=all"
elif [[ "$EXPR" == "monoos" ]]; then
  EXPR_FLAG="-e raw=${MONOOS_SYSCALLS}"
elif [[ "$EXPR" == "net" ]]; then
  EXPR_FLAG="-e trace=network"
else
  EXPR_FLAG="-e trace=$EXPR"
fi

# Output flag
OUT_FLAG=()
if [[ -n "$OUTPUT" ]]; then
  OUT_FLAG=("-o" "$OUTPUT")
  echo "Writing strace output to $OUTPUT"
fi

# Annotation filter: replace syscall numbers with MonoOS names
annotate() {
  sed     -e "s/syscall_0x190\b/monoos_perm_check/g"     -e "s/syscall_0x191\b/monoos_perm_set/g"       -e "s/syscall_0x192\b/monoos_privacy_stat/g"     -e "s/syscall(400)/monoos_perm_check/g"          -e "s/syscall(401)/monoos_perm_set/g"            -e "s/syscall(402)/monoos_privacy_stat/g"
}

# Run
STRACE_ARGS=(
  -f              # follow forks
  -tt             # microsecond timestamps
  -s 256          # max string length
  "${EXPR_FLAG}"
  "${OUT_FLAG[@]}"
)

if [[ -n "$PID" ]]; then
  exec "$STRACE" "${STRACE_ARGS[@]}" -p "$PID" 2>&1 | annotate
elif [[ ${#CMD_ARGS[@]} -gt 0 ]]; then
  exec "$STRACE" "${STRACE_ARGS[@]}" -- "${CMD_ARGS[@]}" 2>&1 | annotate
else
  usage
fi
