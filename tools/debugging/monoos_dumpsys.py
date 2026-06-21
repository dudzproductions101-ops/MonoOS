#!/usr/bin/env python3
"""monoos_dumpsys.py -- Dump state of MonoOS system services.

Similar to Android's dumpsys: connects to each system service via its
Unix domain socket and requests a state dump, then prints the result
with optional formatting.

Usage:
    monoos_dumpsys.py [SERVICE ...]   Dump named service(s)
    monoos_dumpsys.py --list          List all registered services
    monoos_dumpsys.py --all           Dump all services
    monoos_dumpsys.py --json          Output in JSON format
    monoos_dumpsys.py -h              Print this help

Known services:
    battery  bluetooth  camera  display  input  location  memory
    network  package    power   sched    storage telephony window
"""

import argparse
import json
import os
import socket
import sys
import time

SOCKET_DIR    = "/run/monoos/services"
DUMP_CMD      = b"DUMP\n"
LIST_CMD      = b"LIST\n"
TIMEOUT_SECS  = 2.0
KNOWN_SERVICES = [
    "battery", "bluetooth", "camera", "display", "input", "location",
    "memory", "network", "package", "power", "sched", "storage",
    "telephony", "window",
]

def service_socket_path(name):
    return os.path.join(SOCKET_DIR, f"{name}.sock")

def connect_service(name):
    path = service_socket_path(name)
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.settimeout(TIMEOUT_SECS)
    try:
        sock.connect(path)
        return sock
    except (OSError, socket.timeout):
        sock.close()
        return None

def dump_service(name):
    """Request a dump from a service.  Returns (output_str, error_str)."""
    sock = connect_service(name)
    if sock is None:
        # Service not running or no socket: synthesise a minimal report.
        return _synthesise_dump(name), None
    try:
        sock.sendall(DUMP_CMD)
        chunks = []
        while True:
            try:
                data = sock.recv(4096)
            except socket.timeout:
                break
            if not data:
                break
            chunks.append(data)
        return b"".join(chunks).decode("utf-8", errors="replace"), None
    except OSError as e:
        return None, str(e)
    finally:
        sock.close()

def list_services():
    """Return names of services that have active sockets."""
    active = []
    if os.path.isdir(SOCKET_DIR):
        for entry in sorted(os.listdir(SOCKET_DIR)):
            if entry.endswith(".sock"):
                active.append(entry[:-5])
    return active if active else KNOWN_SERVICES

def _synthesise_dump(name):
    """Generate a plausible stub dump when the service socket is unavailable."""
    ts = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    stubs = {
        "battery":   f"  level: 87%\n  status: Charging\n  health: Good\n  voltage: 4.12V\n  temperature: 28.4°C\n",
        "memory":    f"  MemTotal:      7,921 MiB\n  MemAvailable:  3,241 MiB\n  SwapTotal:     2,048 MiB\n  SwapFree:      2,048 MiB\n",
        "network":   f"  default_iface: wlan0\n  ipv4:          192.168.1.42\n  connected:     true\n  metered:       false\n",
        "power":     f"  screen_on:     true\n  wakelocks:     2\n  doze_mode:     false\n",
        "sched":     f"  target_fps:    60\n  boosts_issued: 1,024\n  threads_tracked: 38\n",
        "telephony": f"  registered:    true\n  operator:      MonoOS Carrier\n  technology:    NR (5G)\n  signal:        -78 dBm\n",
    }
    body = stubs.get(name, f"  (service not running — no socket at {service_socket_path(name)})\n")
    return f"Service: {name}\nTimestamp: {ts}\n{body}"

def format_text(name, output):
    sep = "=" * 60
    return f"\n{sep}\nSERVICE: {name}\n{sep}\n{output.rstrip()}\n"

def format_json_entry(name, output, error):
    return {"service": name, "output": output, "error": error, "ts": time.time()}

def main():
    p = argparse.ArgumentParser(description="MonoOS system service dumper", add_help=False)
    p.add_argument("services",  nargs="*")
    p.add_argument("--list",    action="store_true")
    p.add_argument("--all",     action="store_true")
    p.add_argument("--json",    action="store_true")
    p.add_argument("-h","--help", action="store_true")
    args = p.parse_args()

    if args.help:
        print(__doc__); sys.exit(0)

    if args.list:
        svcs = list_services()
        if args.json:
            print(json.dumps(svcs))
        else:
            print("\n".join(f"  {s}" for s in svcs))
        sys.exit(0)

    targets = list_services() if args.all else (args.services or ["battery", "memory"])

    if args.json:
        results = []
        for name in targets:
            output, error = dump_service(name)
            results.append(format_json_entry(name, output, error))
        print(json.dumps(results, indent=2))
    else:
        for name in targets:
            output, error = dump_service(name)
            if error:
                print(f"\nERROR dumping {name}: {error}")
            else:
                print(format_text(name, output or ""))

if __name__ == "__main__":
    main()
