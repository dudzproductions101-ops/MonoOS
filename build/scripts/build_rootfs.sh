#!/usr/bin/env bash
# build_rootfs.sh – Create a MonoOS root filesystem ext4 image and
# configure QEMU to boot from it instead of staying in ramdisk.
#
# What this does
# ──────────────
# 1. Builds all service binaries (cargo build --release in services/).
# 2. Creates a 2 GB sparse ext4 image at build/images/monoos_rootfs.img.
# 3. Populates the standard directory tree inside it.
# 4. Copies service binaries → /system/bin/, kernel modules → /system/lib/modules/.
# 5. Writes /system/etc/fstab, /system/etc/hostname, and the startup profile.
# 6. Prints the QEMU command line to use with the new disk.
#
# Usage
# ──────
#   ./build/scripts/build_rootfs.sh [--clean] [--no-build]
#
# Options
#   --clean      Remove existing rootfs image before creating a new one.
#   --no-build   Skip cargo build (use previously built binaries).
#
# Requirements
#   - cargo + rustup (Rust stable)
#   - e2fsprogs  (mkfs.ext4, e2fsck, debugfs)
#   - OR: mke2fs (same package on most distros)
#
# The ext4 image is created as a sparse file — it reports 2 GB to the
# filesystem but only occupies as many disk blocks as are actually written.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# ── Paths ─────────────────────────────────────────────────────────────────────
SERVICES_DIR="$ROOT/services"
KERNEL_MOD_DIR="$ROOT/build/images/modules"
BUILD_DIR="$ROOT/build/images"
RELEASE_DIR="$ROOT/build/release"
ROOTFS_IMG="$BUILD_DIR/monoos_rootfs.img"
MOUNT_TMP="$BUILD_DIR/rootfs_mnt"
STARTUP_CONF="$ROOT/init/startup_profiles/normal.conf"
HOSTNAME_FILE="$ROOT/init/hostname"

# ── Options ───────────────────────────────────────────────────────────────────
CLEAN=0
NO_BUILD=0

for arg in "$@"; do
  case "$arg" in
    --clean)    CLEAN=1    ;;
    --no-build) NO_BUILD=1 ;;
  esac
done

# ── Helpers ───────────────────────────────────────────────────────────────────
log()  { echo "[rootfs] $*"; }
die()  { echo "[rootfs] ERROR: $*" >&2; exit 1; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "Required tool not found: $1 (install e2fsprogs)"
}

# ── Prereq check ──────────────────────────────────────────────────────────────
require_cmd mkfs.ext4
require_cmd debugfs

if [[ "$NO_BUILD" -eq 0 ]]; then
  command -v cargo >/dev/null 2>&1 || die "cargo not found — install Rust: https://rustup.rs"
fi

mkdir -p "$BUILD_DIR" "$RELEASE_DIR"

# ── Step 1: Build service binaries ───────────────────────────────────────────
if [[ "$NO_BUILD" -eq 0 ]]; then
  log "Building service binaries (release)…"
  (
    cd "$SERVICES_DIR"
    cargo build --release --workspace 2>&1
  )
  log "Service binaries built."
fi

# Locate Rust release output directory.
# cargo puts workspace binaries in services/target/release/.
CARGO_TARGET="$SERVICES_DIR/target/release"

# ── Step 2: Create ext4 image ─────────────────────────────────────────────────
if [[ "$CLEAN" -eq 1 && -f "$ROOTFS_IMG" ]]; then
  log "Removing existing rootfs image."
  rm -f "$ROOTFS_IMG"
fi

if [[ ! -f "$ROOTFS_IMG" ]]; then
  log "Creating 2 GB sparse ext4 image at $ROOTFS_IMG…"
  # truncate creates a sparse file — no 2 GB of actual zeros written to disk.
  truncate -s 2G "$ROOTFS_IMG"
  mkfs.ext4 -F \
    -L "monoos-system" \
    -m 1 \
    -O "^has_journal" \
    "$ROOTFS_IMG"
  log "ext4 image created."
else
  log "Reusing existing image (use --clean to recreate)."
fi

# ── Step 3: Populate the directory tree via debugfs ──────────────────────────
# debugfs can create directories and write files without mounting (no root needed).
log "Populating directory tree…"

debugfs -w "$ROOTFS_IMG" << 'DEBUGFS'
mkdir system
mkdir system/bin
mkdir system/lib
mkdir system/lib/modules
mkdir system/etc
mkdir system/etc/monoos
mkdir data
mkdir data/user
mkdir data/app
mkdir data/local
mkdir data/local/tmp
mkdir cache
mkdir proc
mkdir sys
mkdir dev
DEBUGFS

log "Directory tree created."

# ── Step 4: Copy service binaries → /system/bin/ ─────────────────────────────
log "Copying service binaries…"

SERVICES=(
  permission_service
  system_server
  package_service
  account_service
  app_service
  audio_service
  bluetooth_service
  camera_service
  gps_service
  network_service
  power_service
  settings_service
  storage_service
  update_service
  wifi_service
)

BIN_COUNT=0
for svc in "${SERVICES[@]}"; do
  bin="$CARGO_TARGET/$svc"
  if [[ -f "$bin" ]]; then
    debugfs -w "$ROOTFS_IMG" -R "write $bin /system/bin/$svc" 2>/dev/null
    BIN_COUNT=$((BIN_COUNT + 1))
    log "  → /system/bin/$svc"
  else
    log "  SKIP: $bin not found (run without --no-build)"
  fi
done

log "Copied $BIN_COUNT service binaries."

# ── Step 5: Copy kernel modules → /system/lib/modules/ ───────────────────────
log "Copying kernel modules…"
MOD_COUNT=0
if [[ -d "$KERNEL_MOD_DIR" ]]; then
  for ko in "$KERNEL_MOD_DIR"/*.ko; do
    [[ -f "$ko" ]] || continue
    name="$(basename "$ko")"
    debugfs -w "$ROOTFS_IMG" -R "write $ko /system/lib/modules/$name" 2>/dev/null
    MOD_COUNT=$((MOD_COUNT + 1))
    log "  → /system/lib/modules/$name"
  done
fi
log "Copied $MOD_COUNT kernel modules."

# ── Step 6: Write config files ────────────────────────────────────────────────
log "Writing config files…"

# /system/etc/fstab
FSTAB_TMP="$(mktemp)"
cat > "$FSTAB_TMP" << 'FSTAB'
# MonoOS fstab
# <device>      <mount>  <type>   <options>               <dump> <pass>
/dev/sda        /        ext4     rw,relatime,errors=panic  0      1
proc            /proc    proc     nosuid,nodev,noexec       0      0
sysfs           /sys     sysfs    nosuid,nodev,noexec       0      0
devtmpfs        /dev     devtmpfs nosuid,mode=0755          0      0
tmpfs           /cache   tmpfs    nosuid,nodev,size=128m    0      0
tmpfs           /run     tmpfs    nosuid,nodev,mode=0755    0      0
FSTAB
debugfs -w "$ROOTFS_IMG" -R "write $FSTAB_TMP /system/etc/fstab" 2>/dev/null
rm "$FSTAB_TMP"
log "  → /system/etc/fstab"

# /system/etc/hostname
HN_TMP="$(mktemp)"
if [[ -f "$HOSTNAME_FILE" ]]; then
  cp "$HOSTNAME_FILE" "$HN_TMP"
else
  echo "monoos-dev" > "$HN_TMP"
fi
debugfs -w "$ROOTFS_IMG" -R "write $HN_TMP /system/etc/hostname" 2>/dev/null
rm "$HN_TMP"
log "  → /system/etc/hostname"

# /system/etc/monoos/normal.conf (startup profile)
if [[ -f "$STARTUP_CONF" ]]; then
  debugfs -w "$ROOTFS_IMG" -R "write $STARTUP_CONF /system/etc/monoos/normal.conf" 2>/dev/null
  log "  → /system/etc/monoos/normal.conf"
else
  log "  SKIP: $STARTUP_CONF not found"
fi

# ── Step 7: Verify ────────────────────────────────────────────────────────────
log "Verifying image…"
e2fsck -p "$ROOTFS_IMG" 2>&1 || true  # -p auto-fixes; non-zero is OK on first run
log "Image ready: $ROOTFS_IMG"

# ── Print QEMU command line ───────────────────────────────────────────────────
INITRD="$BUILD_DIR/monoos_initrd.cpio.gz"
KERNEL="${KERNEL:-/boot/vmlinuz-$(uname -r)}"

cat << EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  rootfs image:  $ROOTFS_IMG
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  To boot in QEMU with the rootfs (adjust paths as needed):

  qemu-system-x86_64 \\
    -kernel "$KERNEL" \\
    -initrd "$INITRD" \\
    -drive  file="$ROOTFS_IMG",format=raw,if=virtio \\
    -append "console=ttyS0 root=/dev/vda rw quiet" \\
    -nographic \\
    -m 1G

  The initrd loads the kernel modules; the PID-1 init script
  will then pivot_root to /dev/vda and start services from
  /system/bin/ as defined in /system/etc/monoos/normal.conf.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
EOF
