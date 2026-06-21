#!/usr/bin/env python3
"""bugreport.py -- MonoOS bug report generator.

Collects system state, logs, and diagnostics into a single zip archive
for submission to the MonoOS issue tracker.

Usage:
    bugreport.py [--output FILE] [--no-logs] [--no-crash] [--verbose]

Collected artifacts:
    system_info.txt         OS version, build, model, kernel
    proc_meminfo.txt        /proc/meminfo
    proc_loadavg.txt        /proc/loadavg
    proc_net_dev.txt        /proc/net/dev
    logcat.txt              Last 2000 lines of logcat (if accessible)
    kmsg.txt                Last 500 kernel ring buffer lines
    tombstones/             Native crash reports (last 10)
    anr/                    ANR traces (last 5)
    monoos/                  /proc/monoos/* snapshots
    packages.json           Installed packages list
"""

import argparse
import datetime
import json
import os
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

VERSION = "1.0.0"

def run(cmd, timeout=5):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        return r.stdout or r.stderr or ""
    except Exception as e:
        return f"(error: {e})"

def read_file(path, max_lines=None):
    try:
        text = Path(path).read_text(errors="replace")
        if max_lines:
            lines = text.splitlines()
            if len(lines) > max_lines:
                text = "\n".join(lines[-max_lines:])
        return text
    except OSError as e:
        return f"(unavailable: {e})"

def collect_proc_monoos():
    entries = {}
    base = Path("/proc/monoos")
    if not base.exists():
        return {"error": "/proc/monoos not found"}
    for entry in sorted(base.iterdir()):
        if entry.is_file():
            entries[entry.name] = read_file(entry, max_lines=200)
    return entries

def collect_packages():
    pkg_db = Path("/data/system/packages.xml")
    if pkg_db.exists():
        return read_file(pkg_db, max_lines=5000)
    # Try pm list via adb if on host
    return run(["adb", "shell", "pm", "list", "packages", "-f"])

def collect_tombstones(limit=10):
    results = {}
    tomb_dir = Path("/data/tombstones")
    if not tomb_dir.exists():
        return results
    stones = sorted(tomb_dir.glob("tombstone_*"), reverse=True)[:limit]
    for s in stones:
        results[s.name] = read_file(s, max_lines=300)
    return results

def collect_anr(limit=5):
    results = {}
    anr_dir = Path("/data/anr")
    if not anr_dir.exists():
        return results
    anrs = sorted(anr_dir.glob("anr_*"), reverse=True)[:limit]
    for a in anrs:
        results[a.name] = read_file(a, max_lines=500)
    return results

def build_system_info():
    def getprop(key):
        return run(["getprop", key]).strip() or run(["sh", "-c", f"getprop {key}"]).strip() or "unknown"

    return "\n".join([
        f"MonoOS Bug Report",
        f"Generated : {datetime.datetime.utcnow().isoformat()}Z",
        f"Tool      : bugreport.py {VERSION}",
        "=" * 50,
        f"OS Version : {getprop('ro.monoos.version')}",
        f"Build ID   : {getprop('ro.monoos.build.id')}",
        f"Model      : {getprop('ro.product.model')}",
        f"ABI        : {getprop('ro.product.cpu.abi')}",
        f"Kernel     : {run(['uname', '-r']).strip()}",
        f"Hostname   : {run(['hostname']).strip()}",
    ])

def main():
    p = argparse.ArgumentParser(description="MonoOS bug report generator")
    p.add_argument("--output",   default="", help="Output zip filename (default: bugreport_<ts>.zip)")
    p.add_argument("--no-logs",  action="store_true")
    p.add_argument("--no-crash", action="store_true")
    p.add_argument("--verbose",  action="store_true")
    args = p.parse_args()

    ts      = datetime.datetime.utcnow().strftime("%Y%m%d_%H%M%S")
    outfile = args.output or f"bugreport_{ts}.zip"

    print(f"MonoOS Bug Report v{VERSION}")
    print(f"Collecting diagnostics...")

    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)

        def add(name, content):
            if args.verbose:
                print(f"  + {name}")
            path = tmp / name
            path.parent.mkdir(parents=True, exist_ok=True)
            if isinstance(content, dict):
                path.write_text(json.dumps(content, indent=2))
            else:
                path.write_text(str(content))

        add("system_info.txt",     build_system_info())
        add("proc_meminfo.txt",    read_file("/proc/meminfo"))
        add("proc_loadavg.txt",    read_file("/proc/loadavg"))
        add("proc_version.txt",    read_file("/proc/version"))
        add("proc_net_dev.txt",    read_file("/proc/net/dev"))
        add("proc_mounts.txt",     read_file("/proc/mounts"))
        add("packages.txt",        collect_packages())

        # /proc/monoos snapshots
        monoos_data = collect_proc_monoos()
        for name, content in monoos_data.items():
            add(f"monoos/{name}", content)

        # Logs
        if not args.no_logs:
            add("kmsg.txt",    run(["dmesg", "-T"], timeout=10))
            add("logcat.txt",  run(["logcat", "-d", "-t", "2000"], timeout=15))

        # Crash data
        if not args.no_crash:
            for name, content in collect_tombstones().items():
                add(f"tombstones/{name}", content)
            for name, content in collect_anr().items():
                add(f"anr/{name}", content)

        # Write zip
        with zipfile.ZipFile(outfile, "w", zipfile.ZIP_DEFLATED) as zf:
            for path in sorted(tmp.rglob("*")):
                if path.is_file():
                    zf.write(path, path.relative_to(tmp))

    size_kb = Path(outfile).stat().st_size // 1024
    print(f"\nBug report written: {outfile}  ({size_kb} KB)")
    print(f"Submit at: https://bugs.monoos.io/new  or  email dev@monoos.io")

if __name__ == "__main__":
    main()
