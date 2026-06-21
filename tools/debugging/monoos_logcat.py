#!/usr/bin/env python3
"""monoos_logcat.py -- MonoOS structured log reader.

Reads from /proc/monoos/fs_events, /proc/monoos/net_events, and the kernel
ring buffer (via /dev/kmsg) and presents a unified, colour-coded log stream.

Usage:
    monoos_logcat.py [OPTIONS]

Options:
    -l, --level LEVEL   Minimum log level: V D I W E F  (default: D)
    -t, --tag   TAG     Only show entries whose tag contains TAG
    -p, --pid   PID     Filter to a specific process ID
    --no-color          Disable ANSI colour output
    --kmsg              Also stream the kernel ring buffer (/dev/kmsg)
    --net               Also stream /proc/monoos/net_events
    --fs                Also stream /proc/monoos/fs_events
    --lsm               Also stream /proc/monoos/lsm_audit
    --since  N          Only show entries from the last N seconds
    --format FORMAT     Output format: text (default) | json
    -h, --help          Print this message and exit
"""

import argparse
import json
import os
import re
import select
import sys
import time

# ── ANSI colour helpers ────────────────────────────────────────────────────

RESET  = "[0m"
BOLD   = "[1m"
LEVEL_COLORS = {
    "V": "[37m",    # white
    "D": "[36m",    # cyan
    "I": "[32m",    # green
    "W": "[33m",    # yellow
    "E": "[31m",    # red
    "F": "[35m",    # magenta
}
LEVEL_ORDER = {"V": 0, "D": 1, "I": 2, "W": 3, "E": 4, "F": 5}

def colorise(level, text, use_color):
    if not use_color:
        return text
    color = LEVEL_COLORS.get(level.upper(), "")
    return f"{color}{text}{RESET}"

# ── Log entry dataclass ────────────────────────────────────────────────────

class LogEntry:
    __slots__ = ("ts", "level", "pid", "tag", "message", "source")

    def __init__(self, ts, level, pid, tag, message, source="system"):
        self.ts      = ts
        self.level   = level.upper()
        self.pid     = pid
        self.tag     = tag
        self.message = message
        self.source  = source

    def to_text(self, use_color=True):
        ts_str  = f"{self.ts:>14.3f}"
        prefix  = f"{ts_str}  {self.level}/[{self.pid:>6}]  {self.tag:<24}  "
        body    = self.message
        return colorise(self.level, prefix + body, use_color)

    def to_json(self):
        return json.dumps({
            "ts":      self.ts,
            "level":   self.level,
            "pid":     self.pid,
            "tag":     self.tag,
            "message": self.message,
            "source":  self.source,
        })

# ── Source readers ─────────────────────────────────────────────────────────

def read_kmsg(fd, since_ts):
    """Parse one line from /dev/kmsg into a LogEntry."""
    try:
        line = os.read(fd, 4096).decode("utf-8", errors="replace").rstrip()
    except BlockingIOError:
        return None
    # Format: <priority>,<seq>,<timestamp_us>,-;message
    m = re.match(r"(\d+),(\d+),(\d+),-;(.*)", line)
    if not m:
        return None
    priority  = int(m.group(1)) & 7
    ts_us     = int(m.group(3))
    ts        = ts_us / 1_000_000.0
    msg       = m.group(4)
    level_map = {0: "F", 1: "F", 2: "F", 3: "E", 4: "W", 5: "I", 6: "I", 7: "D"}
    level     = level_map.get(priority, "D")
    if ts < since_ts:
        return None
    return LogEntry(ts, level, 0, "kernel", msg, source="kmsg")

def read_proc_events(path, since_ts, source, level="I"):
    """Read all pending lines from a /proc/monoos/* ring buffer file."""
    entries = []
    try:
        with open(path, "r") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                parts = line.split(None, 5)
                if len(parts) < 5:
                    continue
                try:
                    ts  = int(parts[0]) / 1_000_000_000.0
                    pid = int(parts[1])
                    uid = parts[2]
                    msg = " ".join(parts[3:])
                except (ValueError, IndexError):
                    continue
                if ts < since_ts:
                    continue
                entries.append(LogEntry(ts, level, pid, f"uid={uid}", msg, source=source))
    except (OSError, PermissionError):
        pass
    return entries

# ── Main ───────────────────────────────────────────────────────────────────

def parse_args():
    p = argparse.ArgumentParser(
        description="MonoOS structured log reader",
        add_help=False,
    )
    p.add_argument("-l", "--level",    default="D",   metavar="LEVEL")
    p.add_argument("-t", "--tag",      default=None,  metavar="TAG")
    p.add_argument("-p", "--pid",      default=None,  type=int)
    p.add_argument("--no-color",       action="store_true")
    p.add_argument("--kmsg",           action="store_true")
    p.add_argument("--net",            action="store_true")
    p.add_argument("--fs",             action="store_true")
    p.add_argument("--lsm",            action="store_true")
    p.add_argument("--since",          default=0,     type=float, metavar="N")
    p.add_argument("--format",         default="text", choices=["text", "json"])
    p.add_argument("-h", "--help",     action="store_true")
    return p.parse_args()

def should_show(entry, min_level, tag_filter, pid_filter):
    if LEVEL_ORDER.get(entry.level, 0) < LEVEL_ORDER.get(min_level, 0):
        return False
    if tag_filter and tag_filter.lower() not in entry.tag.lower():
        return False
    if pid_filter is not None and entry.pid != pid_filter:
        return False
    return True

def print_entry(entry, fmt, use_color):
    if fmt == "json":
        print(entry.to_json(), flush=True)
    else:
        print(entry.to_text(use_color), flush=True)

def main():
    args = parse_args()

    if args.help:
        print(__doc__); sys.exit(0)

    use_color  = sys.stdout.isatty() and not args.no_color
    min_level  = args.level.upper()
    since_ts   = time.monotonic() - args.since if args.since > 0 else 0.0
    fmt        = args.format

    if use_color:
        tag_part = f"  tag={args.tag}" if args.tag else ""
        sys.stdout.write(
            f"{BOLD}MonoOS logcat  (level>={min_level}{tag_part}){RESET}\n\n"
        )

    # Open kmsg non-blocking if requested.
    kmsg_fd = None
    if args.kmsg:
        try:
            kmsg_fd = os.open("/dev/kmsg", os.O_RDONLY | os.O_NONBLOCK)
            os.lseek(kmsg_fd, 0, os.SEEK_END)   # jump to tail
        except OSError:
            sys.stderr.write("Warning: cannot open /dev/kmsg (need root?)\n")
            kmsg_fd = None

    try:
        while True:
            entries = []

            # Kernel ring buffer
            if kmsg_fd is not None:
                while True:
                    e = read_kmsg(kmsg_fd, since_ts)
                    if e is None:
                        break
                    entries.append(e)

            # /proc ring buffers
            if args.net:
                entries += read_proc_events("/proc/monoos/net_events", since_ts,
                                            "net_events", level="I")
            if args.fs:
                entries += read_proc_events("/proc/monoos/fs_events", since_ts,
                                            "fs_events", level="I")
            if args.lsm:
                entries += read_proc_events("/proc/monoos/lsm_audit", since_ts,
                                            "lsm_audit", level="W")

            entries.sort(key=lambda e: e.ts)

            for entry in entries:
                if should_show(entry, min_level, args.tag, args.pid):
                    print_entry(entry, fmt, use_color)

            if not entries:
                time.sleep(0.05)

    except KeyboardInterrupt:
        pass
    finally:
        if kmsg_fd is not None:
            os.close(kmsg_fd)

if __name__ == "__main__":
    main()
