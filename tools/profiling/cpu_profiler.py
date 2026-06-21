#!/usr/bin/env python3
"""cpu_profiler.py -- MonoOS on-device CPU sampling profiler.

Samples /proc/<pid>/stat (or every process under /proc when no PID is
given) at a fixed interval and aggregates per-thread CPU usage into a
flat or call-tree report. Designed to run directly on a MonoOS device
over adb shell, with no external dependencies beyond /proc.

Usage:
    cpu_profiler.py --pid <pid> [--duration 10] [--interval 0.1]
    cpu_profiler.py --package com.example.app [--duration 10]
    cpu_profiler.py --top [--duration 10]          System-wide top consumers
    cpu_profiler.py --output report.json
"""

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

CLK_TCK = 100  # USER_HZ, standard on Linux unless CONFIG_HZ differs


def read_stat(pid: int):
    """Parse /proc/<pid>/stat. Returns (comm, utime, stime) or None."""
    try:
        text = Path(f"/proc/{pid}/stat").read_text()
    except OSError:
        return None
    # comm field may contain spaces/parens; find the last ')' to split safely.
    rparen = text.rfind(")")
    if rparen == -1:
        return None
    comm = text[text.find("(") + 1:rparen]
    rest = text[rparen + 2:].split()
    try:
        utime = int(rest[11])  # field 14 overall: utime
        stime = int(rest[12])  # field 15 overall: stime
    except (IndexError, ValueError):
        return None
    return comm, utime, stime


def list_threads(pid: int):
    task_dir = Path(f"/proc/{pid}/task")
    if not task_dir.exists():
        return []
    return [int(p.name) for p in task_dir.iterdir() if p.name.isdigit()]


def find_pid_by_package(package: str):
    """Resolve a package name to a PID via pidof or /proc scanning."""
    try:
        out = subprocess.run(["pidof", package], capture_output=True, text=True, timeout=3)
        pid_str = out.stdout.strip().split()
        if pid_str:
            return int(pid_str[0])
    except (OSError, subprocess.SubprocessError):
        pass
    # Fallback: scan /proc/*/cmdline
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        cmdline_path = entry / "cmdline"
        try:
            cmdline = cmdline_path.read_bytes().split(b"\x00")[0].decode(errors="replace")
        except OSError:
            continue
        if package in cmdline:
            return int(entry.name)
    return None


def list_all_pids():
    return [int(p.name) for p in Path("/proc").iterdir() if p.name.isdigit()]


def sample_cpu_times(pids):
    """Return {pid: (comm, utime, stime)} for all readable pids."""
    samples = {}
    for pid in pids:
        result = read_stat(pid)
        if result:
            samples[pid] = result
    return samples


def profile_pid(pid: int, duration: float, interval: float, threads: bool, verbose: bool):
    """Sample CPU usage of a single process (and optionally its threads)."""
    targets = [pid] + (list_threads(pid) if threads else [])
    start_samples = sample_cpu_times(targets)
    if not start_samples:
        print(f"ERROR: PID {pid} not found or unreadable.", file=sys.stderr)
        return None

    start_wall = time.monotonic()
    time.sleep(duration)
    end_wall = time.monotonic()
    end_samples = sample_cpu_times(list(start_samples.keys()))

    wall_elapsed = end_wall - start_wall
    results = []
    for tid, (comm, u0, s0) in start_samples.items():
        if tid not in end_samples:
            continue
        _, u1, s1 = end_samples[tid]
        cpu_ticks = (u1 - u0) + (s1 - s0)
        cpu_secs = cpu_ticks / CLK_TCK
        pct = (cpu_secs / wall_elapsed * 100) if wall_elapsed > 0 else 0.0
        results.append({"tid": tid, "comm": comm, "cpu_seconds": round(cpu_secs, 3),
                         "cpu_percent": round(pct, 2)})
        if verbose:
            print(f"  tid={tid:<8} comm={comm:<20} cpu={pct:.1f}%")

    results.sort(key=lambda r: r["cpu_percent"], reverse=True)
    return {"pid": pid, "duration_s": round(wall_elapsed, 2), "threads": results}


def profile_system_top(duration: float, interval: float, top_n: int):
    """Sample CPU usage across all processes and report the top consumers."""
    pids = list_all_pids()
    start_samples = sample_cpu_times(pids)
    start_wall = time.monotonic()
    time.sleep(duration)
    end_wall = time.monotonic()
    end_samples = sample_cpu_times(list(start_samples.keys()))

    wall_elapsed = end_wall - start_wall
    results = []
    for pid, (comm, u0, s0) in start_samples.items():
        if pid not in end_samples:
            continue
        _, u1, s1 = end_samples[pid]
        cpu_ticks = (u1 - u0) + (s1 - s0)
        cpu_secs = cpu_ticks / CLK_TCK
        pct = (cpu_secs / wall_elapsed * 100) if wall_elapsed > 0 else 0.0
        if cpu_secs > 0:
            results.append({"pid": pid, "comm": comm, "cpu_seconds": round(cpu_secs, 3),
                             "cpu_percent": round(pct, 2)})

    results.sort(key=lambda r: r["cpu_percent"], reverse=True)
    return {"duration_s": round(wall_elapsed, 2), "processes": results[:top_n]}


def print_report(report, top_n: int):
    if "threads" in report:
        print(f"\nCPU profile for PID {report['pid']}  ({report['duration_s']}s sample window)")
        print(f"{'TID':<10}{'COMM':<24}{'CPU%':>8}{'CPU(s)':>10}")
        print("-" * 54)
        for t in report["threads"][:top_n]:
            print(f"{t['tid']:<10}{t['comm']:<24}{t['cpu_percent']:>7.1f}%{t['cpu_seconds']:>10.3f}")
    else:
        print(f"\nSystem-wide top CPU consumers  ({report['duration_s']}s sample window)")
        print(f"{'PID':<10}{'COMM':<24}{'CPU%':>8}{'CPU(s)':>10}")
        print("-" * 54)
        for p in report["processes"][:top_n]:
            print(f"{p['pid']:<10}{p['comm']:<24}{p['cpu_percent']:>7.1f}%{p['cpu_seconds']:>10.3f}")
    print()


def main():
    p = argparse.ArgumentParser(description="MonoOS CPU sampling profiler")
    p.add_argument("--pid",       type=int, help="Profile a specific PID")
    p.add_argument("--package",   help="Resolve a package name to a PID and profile it")
    p.add_argument("--top",       action="store_true", help="System-wide top CPU consumers")
    p.add_argument("--threads",   action="store_true", help="Also break down by thread (with --pid)")
    p.add_argument("--duration",  type=float, default=10.0, help="Sample window in seconds")
    p.add_argument("--interval",  type=float, default=0.1, help="Reserved for future sub-sampling")
    p.add_argument("--top-n",     type=int, default=15, help="Number of rows to display")
    p.add_argument("--output",    help="Write JSON report to this file")
    p.add_argument("--verbose",   action="store_true")
    args = p.parse_args()

    target_pid = args.pid
    if args.package:
        target_pid = find_pid_by_package(args.package)
        if target_pid is None:
            print(f"ERROR: no running process found for package '{args.package}'", file=sys.stderr)
            sys.exit(1)
        print(f"Resolved {args.package} -> PID {target_pid}")

    if target_pid:
        print(f"Sampling PID {target_pid} for {args.duration}s...")
        report = profile_pid(target_pid, args.duration, args.interval, args.threads, args.verbose)
        if report is None:
            sys.exit(1)
    elif args.top:
        print(f"Sampling all processes for {args.duration}s...")
        report = profile_system_top(args.duration, args.interval, args.top_n)
    else:
        p.print_help()
        sys.exit(1)

    print_report(report, args.top_n)

    if args.output:
        Path(args.output).write_text(json.dumps(report, indent=2))
        print(f"Report written to {args.output}")


if __name__ == "__main__":
    main()
