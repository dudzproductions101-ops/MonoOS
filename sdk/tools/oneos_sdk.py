#!/usr/bin/env python3
"""OneOS SDK command-line tool.

Usage:
  oneos-sdk new <package_name> [--lang rust|cpp]   Create a new app project
  oneos-sdk build [--release] [--target <abi>]     Build the current project
  oneos-sdk install [--device <serial>]            Install OPK to a device
  oneos-sdk run [--device <serial>]                Build, install, and launch
  oneos-sdk audit                                  Check for known CVEs
  oneos-sdk publish <path.opk> --api-key <key>     Upload to package repo
  oneos-sdk --version                              Print SDK version
"""

import argparse, json, os, shutil, subprocess, sys
from pathlib import Path

SDK_VERSION = "1.0.0"
DEFAULT_ABI = "arm64-v8a"
REPO_URL    = "https://packages.oneos.io/api/v1/upload"
TEMPLATES   = Path(__file__).parent.parent / "templates"

def run(cmd, cwd=None, check=True):
    print("  $", " ".join(str(c) for c in cmd))
    return subprocess.run(cmd, cwd=cwd, check=check)

def find_manifest():
    for p in [Path.cwd(), *Path.cwd().parents]:
        c = p / "META-INF" / "manifest.toml"
        if c.exists():
            return c
    raise FileNotFoundError("No META-INF/manifest.toml found.")

def read_manifest(path):
    out = {}
    for line in path.read_text().splitlines():
        line = line.strip()
        if line.startswith("#") or "=" not in line:
            continue
        k, _, v = line.partition("=")
        out[k.strip()] = v.strip().strip('"').strip("'")
    return out

def cmd_new(args):
    pkg = args.package_name
    dst = Path.cwd() / pkg.split(".")[-1]
    src = TEMPLATES / "basic_app"
    if not src.exists():
        print(f"Template not found: {src}", file=sys.stderr); return 1
    if dst.exists():
        print(f"Directory exists: {dst}", file=sys.stderr); return 1
    shutil.copytree(src, dst)
    mf = dst / "META-INF" / "manifest.toml"
    mf.write_text(mf.read_text().replace("com.example.basicapp", pkg))
    print(f"Created: {dst}")
    return 0

def cmd_build(args):
    mf   = find_manifest()
    root = mf.parent.parent
    meta = read_manifest(mf)
    tgt  = getattr(args, "target", DEFAULT_ABI) or DEFAULT_ABI
    prof = "release" if getattr(args, "release", False) else "debug"
    rust_triple = {
        "arm64-v8a":   "aarch64-unknown-linux-gnu",
        "armeabi-v7a": "armv7-unknown-linux-gnueabihf",
        "x86_64":      "x86_64-unknown-linux-gnu",
    }.get(tgt, "aarch64-unknown-linux-gnu")
    cargo = ["cargo", "build", f"--target={rust_triple}"]
    if prof == "release":
        cargo.append("--release")
    try:
        run(cargo, cwd=root)
    except subprocess.CalledProcessError:
        return 1
    pkg = meta.get("package_name", "app")
    ver = meta.get("version_name", "1.0.0")
    opk = root / "build" / prof / f"{pkg}-{ver}.opk"
    opk.parent.mkdir(parents=True, exist_ok=True)
    arc = shutil.make_archive(str(opk.with_suffix("")), "zip", root)
    Path(arc).rename(opk)
    print(f"Built: {opk}")
    return 0

def cmd_install(args):
    mf   = find_manifest()
    root = mf.parent.parent
    meta = read_manifest(mf)
    pkg  = meta.get("package_name", "app")
    ver  = meta.get("version_name", "1.0.0")
    opk  = root / "build" / "debug" / f"{pkg}-{ver}.opk"
    if not opk.exists():
        opk = root / "build" / "release" / f"{pkg}-{ver}.opk"
    if not opk.exists():
        print("Build first: oneos-sdk build", file=sys.stderr); return 1
    dev  = getattr(args, "device", "auto") or "auto"
    adb  = shutil.which("adb")
    if not adb:
        print(f"adb not found. Copy {opk} manually."); return 0
    df   = ["-s", dev] if dev != "auto" else []
    run([adb] + df + ["push", str(opk), "/sdcard/Download/"])
    run([adb] + df + ["shell", "pm", "install", f"/sdcard/Download/{opk.name}"])
    print(f"Installed: {pkg}")
    return 0

def cmd_run(args):
    rc = cmd_build(args)
    if rc: return rc
    rc = cmd_install(args)
    if rc: return rc
    mf  = find_manifest()
    pkg = read_manifest(mf).get("package_name", "app")
    adb = shutil.which("adb")
    if adb:
        subprocess.run([adb, "shell", "am", "start", "-n", f"{pkg}/.MainActivity"], check=False)
    return 0

def cmd_audit(_args):
    print("Running security audit...")
    return subprocess.run(["cargo", "audit"], check=False).returncode if shutil.which("cargo-audit") else 1

def cmd_publish(args):
    opk = Path(args.opk_path)
    if not opk.exists():
        print(f"Not found: {opk}", file=sys.stderr); return 1
    key = getattr(args, "api_key", None) or os.environ.get("ONEOS_DEV_KEY")
    if not key:
        print("Provide --api-key or set ONEOS_DEV_KEY", file=sys.stderr); return 1
    curl = shutil.which("curl")
    if curl:
        return subprocess.run(
            [curl, "-sS", "-X", "POST", REPO_URL,
             "-H", f"Authorization: Bearer {key}", "-F", f"package=@{opk}"],
            check=False).returncode
    print("curl not found"); return 1

def main():
    p = argparse.ArgumentParser(prog="oneos-sdk")
    p.add_argument("--version", action="version", version=f"oneos-sdk {SDK_VERSION}")
    sub = p.add_subparsers(dest="command")

    pn = sub.add_parser("new"); pn.add_argument("package_name"); pn.add_argument("--lang", default="rust")
    pb = sub.add_parser("build"); pb.add_argument("--release", action="store_true"); pb.add_argument("--target", default=DEFAULT_ABI)
    pi = sub.add_parser("install"); pi.add_argument("--device", default="auto")
    pr = sub.add_parser("run"); pr.add_argument("--release", action="store_true"); pr.add_argument("--target", default=DEFAULT_ABI); pr.add_argument("--device", default="auto")
    sub.add_parser("audit")
    pp = sub.add_parser("publish"); pp.add_argument("opk_path"); pp.add_argument("--api-key")

    args = p.parse_args()
    if not args.command:
        p.print_help(); return 0

    return {"new": cmd_new, "build": cmd_build, "install": cmd_install,
            "run": cmd_run, "audit": cmd_audit, "publish": cmd_publish}[args.command](args)

if __name__ == "__main__":
    sys.exit(main())
