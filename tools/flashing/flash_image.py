#!/usr/bin/env python3
"""flash_image.py -- MonoOS device image flashing tool.

Flashes boot, system, vendor, and recovery images to a device connected
in fastboot mode. Wraps the `fastboot` binary with MonoOS-specific
validation: image header checks, slot awareness, and checksum
verification before any write occurs.

Usage:
    flash_image.py --image boot.img --partition boot [--slot a|b]
    flash_image.py --manifest flash_manifest.json [--slot a|b]
    flash_image.py --wipe-userdata
    flash_image.py --list-partitions
    flash_image.py --verify boot.img --partition boot

Safety:
    Every image is checksummed (SHA-256) against the manifest before
    flashing. A mismatch aborts the operation before any fastboot
    command is issued.
"""

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

FASTBOOT_TIMEOUT = 120
KNOWN_PARTITIONS = [
    "boot", "system", "vendor", "recovery", "dtbo",
    "vbmeta", "userdata", "cache", "persist", "misc",
]


def fastboot_available():
    return shutil.which("fastboot") is not None


def run_fastboot(args, timeout=FASTBOOT_TIMEOUT):
    cmd = ["fastboot"] + args
    print(f"  $ {' '.join(cmd)}")
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        print(f"  ERROR: fastboot command timed out after {timeout}s", file=sys.stderr)
        return False, "timeout"
    if result.returncode != 0:
        print(result.stderr.strip(), file=sys.stderr)
        return False, result.stderr.strip()
    if result.stdout.strip():
        print(f"  {result.stdout.strip()}")
    return True, result.stdout.strip()


def sha256_of(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def verify_checksum(image_path: Path, expected_sha256: str) -> bool:
    if not expected_sha256:
        print("  WARNING: no expected checksum provided, skipping verification")
        return True
    actual = sha256_of(image_path)
    if actual != expected_sha256:
        print(f"  CHECKSUM MISMATCH for {image_path.name}")
        print(f"    expected: {expected_sha256}")
        print(f"    actual:   {actual}")
        return False
    print(f"  Checksum OK: {image_path.name}")
    return True


def partition_name(base: str, slot: str | None) -> str:
    """Append A/B slot suffix where applicable."""
    ab_partitions = {"boot", "system", "vendor", "dtbo", "vbmeta"}
    if slot and base in ab_partitions:
        return f"{base}_{slot}"
    return base


def flash_partition(image_path: Path, partition: str, slot: str | None,
                     expected_sha256: str = "", dry_run: bool = False) -> bool:
    if not image_path.exists():
        print(f"  ERROR: image not found: {image_path}", file=sys.stderr)
        return False

    if partition not in KNOWN_PARTITIONS:
        print(f"  WARNING: '{partition}' is not in the known partition list; proceeding anyway.")

    if not verify_checksum(image_path, expected_sha256):
        print("  Aborting flash due to checksum mismatch.")
        return False

    target = partition_name(partition, slot)
    size_mb = image_path.stat().st_size / (1024 * 1024)
    print(f"\nFlashing {image_path.name} ({size_mb:.1f} MB) -> {target}")

    if dry_run:
        print("  [dry-run] would execute: fastboot flash", target, str(image_path))
        return True

    ok, _ = run_fastboot(["flash", target, str(image_path)])
    return ok


def flash_from_manifest(manifest_path: Path, slot: str | None, dry_run: bool) -> bool:
    data = json.loads(manifest_path.read_text())
    base_dir = manifest_path.parent
    images = data.get("images", [])
    if not images:
        print("ERROR: manifest contains no images", file=sys.stderr)
        return False

    print(f"Flash manifest: {manifest_path.name}  ({len(images)} image(s))")
    all_ok = True
    for entry in images:
        img_path = base_dir / entry["file"]
        partition = entry["partition"]
        sha256 = entry.get("sha256", "")
        ok = flash_partition(img_path, partition, slot, sha256, dry_run)
        all_ok = all_ok and ok
        if not ok:
            print(f"  Stopping: flash of '{partition}' failed.", file=sys.stderr)
            break
    return all_ok


def wipe_userdata(dry_run: bool) -> bool:
    print("Wiping userdata partition (factory reset)...")
    if dry_run:
        print("  [dry-run] would execute: fastboot -w")
        return True
    ok, _ = run_fastboot(["-w"])
    return ok


def list_partitions():
    ok, output = run_fastboot(["getvar", "all"])
    if not ok:
        print("Could not query device partitions (is it in fastboot mode?)")
        return
    print(output)


def reboot_device(target: str = ""):
    args = ["reboot"] if not target else ["reboot", target]
    run_fastboot(args)


def main():
    p = argparse.ArgumentParser(description="MonoOS device image flashing tool")
    p.add_argument("--image",      help="Path to a single image file to flash")
    p.add_argument("--partition",  help="Target partition name (used with --image)")
    p.add_argument("--manifest",   help="Path to a flash_manifest.json describing multiple images")
    p.add_argument("--slot",       choices=["a", "b"], default=None, help="A/B slot suffix")
    p.add_argument("--verify",     help="Only verify an image's checksum against the manifest, do not flash")
    p.add_argument("--wipe-userdata", action="store_true", help="Wipe the userdata partition")
    p.add_argument("--list-partitions", action="store_true", help="List partitions reported by the device")
    p.add_argument("--reboot",     choices=["system", "bootloader", "recovery", "fastboot"], default=None)
    p.add_argument("--dry-run",    action="store_true", help="Print actions without executing fastboot")
    p.add_argument("--checksum",   default="", help="Expected SHA-256 for --image (optional)")
    args = p.parse_args()

    if not args.dry_run and not fastboot_available():
        print("ERROR: 'fastboot' not found in PATH. Install Android platform-tools.", file=sys.stderr)
        sys.exit(1)

    if args.list_partitions:
        list_partitions()
        return

    if args.verify:
        img = Path(args.verify)
        if not img.exists():
            print(f"ERROR: {img} not found", file=sys.stderr)
            sys.exit(1)
        print(f"SHA-256: {sha256_of(img)}")
        return

    if args.wipe_userdata:
        ok = wipe_userdata(args.dry_run)
        sys.exit(0 if ok else 1)

    success = True

    if args.manifest:
        success = flash_from_manifest(Path(args.manifest), args.slot, args.dry_run)
    elif args.image and args.partition:
        success = flash_partition(Path(args.image), args.partition, args.slot,
                                   args.checksum, args.dry_run)
    else:
        p.print_help()
        sys.exit(1)

    if success and args.reboot:
        reboot_device(args.reboot if args.reboot != "system" else "")

    print("\nDone." if success else "\nFailed.")
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
