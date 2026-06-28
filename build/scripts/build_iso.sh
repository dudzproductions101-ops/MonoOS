#!/usr/bin/env bash
# build_iso.sh – Produce a bootable MonoOS ISO image for VirtualBox / QEMU
#
# The ISO uses GRUB 2 as bootloader so it works in both BIOS (El Torito) and
# EFI mode (EFI System Partition inside the ISO).  VirtualBox boots it with
# no extra configuration.
#
# What the ISO contains
# ─────────────────────
#   boot/grub/grub.cfg          – boot menu
#   boot/vmlinuz                – Linux kernel
#   boot/monoos_initrd.cpio.gz  – MonoOS initrd (loads kernel modules)
#   monoos/monoos_rootfs.img    – ext4 rootfs (mounted as /dev/sdb inside VM)
#   monoos/VERSION              – build metadata
#
# Boot flow inside VirtualBox
# ───────────────────────────
#   GRUB loads vmlinuz + initrd.
#   Initrd PID-1 loads the 8 MonoOS kernel modules.
#   Kernel command line includes root=/dev/sdb so PID-1 pivot_roots to the
#   ext4 image (exposed as a second "disk" via a loop device trick or
#   directly when you attach the rootfs as a second VirtualBox disk).
#
# Usage
# ─────
#   ./build/scripts/build_iso.sh [--no-build]
#
#   --no-build   Skip cargo + module build; use previously built artefacts.
#
# Requirements (install on Ubuntu/Debian with):
#   sudo apt install grub-pc-bin grub-efi-amd64-bin grub-common xorriso mtools
#
# VirtualBox test setup (after running this script)
# ─────────────────────────────────────────────────
#   1. New VM → Linux / Other Linux (64-bit) → 1 GB RAM
#   2. Storage → Add optical drive → attach build/images/monoos.iso
#   3. Storage → Add hard disk → attach build/images/monoos_rootfs.img (VMDK/raw)
#      OR: just boot from ISO and stay in ramdisk mode for quick smoke tests.
#   4. Start VM.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# ── Paths ─────────────────────────────────────────────────────────────────────
BUILD_DIR="$ROOT/build/images"
ISO_STAGE="$BUILD_DIR/iso_stage"
ISO_OUT="$BUILD_DIR/monoos.iso"
ROOTFS_IMG="$BUILD_DIR/monoos_rootfs.img"
INITRD="$BUILD_DIR/monoos_initrd.cpio.gz"

# Kernel: prefer a local build, fall back to the running host kernel for dev.
KERNEL="${KERNEL:-$BUILD_DIR/vmlinuz}"
if [[ ! -f "$KERNEL" ]]; then
  KERNEL="$(ls /boot/vmlinuz-* 2>/dev/null | sort -V | tail -1 || true)"
  [[ -f "$KERNEL" ]] || { echo "[iso] ERROR: no kernel found — set KERNEL=/path/to/vmlinuz"; exit 1; }
fi

NO_BUILD=0
for arg in "$@"; do [[ "$arg" == "--no-build" ]] && NO_BUILD=1; done

# ── Prereq check ──────────────────────────────────────────────────────────────
require_cmd() { command -v "$1" >/dev/null 2>&1 || { echo "[iso] ERROR: $1 not found — sudo apt install grub-pc-bin grub-efi-amd64-bin xorriso mtools"; exit 1; }; }
require_cmd grub-mkimage
require_cmd xorriso

log() { echo "[iso] $*"; }

# ── Optional: build rootfs first ─────────────────────────────────────────────
if [[ "$NO_BUILD" -eq 0 ]]; then
  log "Building rootfs (calls build_rootfs.sh)…"
  bash "$SCRIPT_DIR/build_rootfs.sh"
fi

# ── Stage directory ───────────────────────────────────────────────────────────
log "Preparing ISO stage at $ISO_STAGE…"
rm -rf "$ISO_STAGE"
mkdir -p \
  "$ISO_STAGE/boot/grub" \
  "$ISO_STAGE/boot/grub/fonts" \
  "$ISO_STAGE/EFI/BOOT" \
  "$ISO_STAGE/monoos"

# ── Copy kernel + initrd ──────────────────────────────────────────────────────
log "Copying kernel: $KERNEL"
cp "$KERNEL" "$ISO_STAGE/boot/vmlinuz"

if [[ -f "$INITRD" ]]; then
  log "Copying initrd: $INITRD"
  cp "$INITRD" "$ISO_STAGE/boot/monoos_initrd.cpio.gz"
else
  log "WARNING: initrd not found at $INITRD — ISO will boot to kernel panic."
  log "         Run build_kernel_modules.sh first and rebuild the initrd."
  # Create a placeholder so xorriso doesn't fail.
  echo "MISSING_INITRD" | gzip > "$ISO_STAGE/boot/monoos_initrd.cpio.gz"
fi

# ── Copy rootfs image ─────────────────────────────────────────────────────────
if [[ -f "$ROOTFS_IMG" ]]; then
  log "Copying rootfs image (this may take a moment for a 2 GB file)…"
  cp "$ROOTFS_IMG" "$ISO_STAGE/monoos/monoos_rootfs.img"
else
  log "WARNING: rootfs image not found — VM will stay in ramdisk mode."
fi

# ── Version file ──────────────────────────────────────────────────────────────
cat > "$ISO_STAGE/monoos/VERSION" << VEREOF
MonoOS Development Build
Built:    $(date -u +"%Y-%m-%d %H:%M:%S UTC")
Kernel:   $(basename "$KERNEL")
Host:     $(uname -srm)
Commit:   $(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo "unknown")
VEREOF

# ── GRUB config ───────────────────────────────────────────────────────────────
cat > "$ISO_STAGE/boot/grub/grub.cfg" << 'GRUB'
# MonoOS GRUB boot menu
# ───────────────────────────────────────────────────────

set default=0
set timeout=5
set timeout_style=menu

insmod all_video
insmod gfxterm
insmod png

terminal_output gfxterm

menuentry "MonoOS (rootfs on /dev/sdb)" {
    linux  /boot/vmlinuz \
           console=tty0 console=ttyS0,115200n8 \
           root=/dev/sdb rw \
           quiet loglevel=3 \
           monoos.debug=0
    initrd /boot/monoos_initrd.cpio.gz
}

menuentry "MonoOS (ramdisk only — no rootfs)" {
    linux  /boot/vmlinuz \
           console=tty0 console=ttyS0,115200n8 \
           quiet loglevel=3 \
           monoos.debug=0
    initrd /boot/monoos_initrd.cpio.gz
}

menuentry "MonoOS (verbose boot)" {
    linux  /boot/vmlinuz \
           console=tty0 console=ttyS0,115200n8 \
           root=/dev/sdb rw \
           monoos.debug=1
    initrd /boot/monoos_initrd.cpio.gz
}

menuentry "Reboot" {
    reboot
}

menuentry "Power off" {
    halt
}
GRUB

# ── Copy GRUB unicode font (needed for gfxterm) ───────────────────────────────
GRUB_FONT=""
for candidate in \
    /usr/share/grub/unicode.pf2 \
    /usr/share/grub2/unicode.pf2 \
    /boot/grub/fonts/unicode.pf2; do
  if [[ -f "$candidate" ]]; then
    GRUB_FONT="$candidate"
    break
  fi
done
if [[ -n "$GRUB_FONT" ]]; then
  cp "$GRUB_FONT" "$ISO_STAGE/boot/grub/fonts/unicode.pf2"
fi

# ── Build GRUB BIOS El Torito core image ─────────────────────────────────────
log "Building GRUB BIOS core image…"

GRUB_LIB_BIOS=""
for d in /usr/lib/grub/i386-pc /usr/lib/grub2/i386-pc; do
  [[ -d "$d" ]] && GRUB_LIB_BIOS="$d" && break
done

if [[ -n "$GRUB_LIB_BIOS" ]]; then
  grub-mkimage \
    --directory "$GRUB_LIB_BIOS" \
    --prefix    "/boot/grub" \
    --output    "$ISO_STAGE/boot/grub/core.img" \
    --format    i386-pc-eltorito \
    --config    "$ISO_STAGE/boot/grub/grub.cfg" \
    biosdisk iso9660 normal \
    linux echo search search_fs_uuid search_fs_file \
    gfxterm gfxterm_background font png \
    part_gpt part_msdos all_video 2>&1 || true

  if [[ -d "$GRUB_LIB_BIOS" ]]; then
    cp "$GRUB_LIB_BIOS"/{lnxboot.img,cdboot.img} "$ISO_STAGE/boot/grub/" 2>/dev/null || true
  fi
else
  log "WARNING: grub-pc-bin not installed — BIOS boot disabled, EFI only."
fi

# ── Build GRUB EFI image ──────────────────────────────────────────────────────
log "Building GRUB EFI image…"

GRUB_LIB_EFI=""
for d in /usr/lib/grub/x86_64-efi /usr/lib/grub2/x86_64-efi; do
  [[ -d "$d" ]] && GRUB_LIB_EFI="$d" && break
done

if [[ -n "$GRUB_LIB_EFI" ]]; then
  grub-mkimage \
    --directory "$GRUB_LIB_EFI" \
    --prefix    "/boot/grub" \
    --output    "$ISO_STAGE/EFI/BOOT/BOOTX64.EFI" \
    --format    x86_64-efi \
    --config    "$ISO_STAGE/boot/grub/grub.cfg" \
    efidisk iso9660 normal \
    linux echo search search_fs_uuid search_fs_file \
    gfxterm gfxterm_background font png \
    part_gpt part_msdos all_video 2>&1 || true

  # EFI boot image via mtools (needed by xorriso --efi-boot)
  if command -v mformat >/dev/null 2>&1; then
    EFI_IMG="$BUILD_DIR/efi.img"
    dd if=/dev/zero of="$EFI_IMG" bs=1M count=4 2>/dev/null
    mformat -i "$EFI_IMG" -F ::
    mmd     -i "$EFI_IMG" ::/EFI ::/EFI/BOOT
    mcopy   -i "$EFI_IMG" "$ISO_STAGE/EFI/BOOT/BOOTX64.EFI" ::/EFI/BOOT/BOOTX64.EFI
  else
    log "WARNING: mtools not installed — EFI boot image skipped."
    EFI_IMG=""
  fi
else
  log "WARNING: grub-efi-amd64-bin not installed — EFI boot disabled."
  EFI_IMG=""
fi

# ── Assemble ISO with xorriso ─────────────────────────────────────────────────
log "Assembling ISO: $ISO_OUT"

XORRISO_ARGS=(
  -as mkisofs
  -iso-level 3
  -full-iso9660-filenames
  -volid "MONOOS_DEV"
  -appid "MonoOS Development Build"
  -publisher "MonoOS Project"
  -preparer "build_iso.sh"
  -no-emul-boot
  -boot-load-size 4
  -boot-info-table
)

# BIOS El Torito
if [[ -f "$ISO_STAGE/boot/grub/core.img" ]]; then
  XORRISO_ARGS+=(
    -b "boot/grub/core.img"
  )
fi

# EFI partition
if [[ -n "${EFI_IMG:-}" && -f "${EFI_IMG}" ]]; then
  XORRISO_ARGS+=(
    --efi-boot-part --efi-startup-part
    -eltorito-alt-boot
    -e "efi.img"
    -no-emul-boot
  )
  cp "$EFI_IMG" "$ISO_STAGE/efi.img"
fi

XORRISO_ARGS+=(
  -output "$ISO_OUT"
  "$ISO_STAGE"
)

xorriso "${XORRISO_ARGS[@]}" 2>&1

ISO_SIZE=$(du -sh "$ISO_OUT" | cut -f1)

# ── Done ──────────────────────────────────────────────────────────────────────
cat << EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ISO ready:  $ISO_OUT  ($ISO_SIZE)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  VirtualBox quick-start
  ──────────────────────
  1. New VM  → Linux → Other Linux (64-bit)
     RAM: 1024 MB  |  Firmware: BIOS (or EFI)

  2. Settings → Storage
     [Optical]  attach  monoos.iso
     [Hard disk] attach  monoos_rootfs.img  (as SATA/IDE port 1)
     (skip the hard disk if you just want ramdisk-only smoke test)

  3. Start → choose "MonoOS (rootfs on /dev/sdb)" or
     "MonoOS (ramdisk only)" from the GRUB menu.

  QEMU one-liner (BIOS mode)
  ──────────────────────────
  qemu-system-x86_64 \\
    -m 1G \\
    -cdrom  "$ISO_OUT" \\
    -drive  file="$ROOTFS_IMG",format=raw,if=virtio \\
    -serial stdio \\
    -boot d

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
EOF
