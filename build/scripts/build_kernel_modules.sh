#!/usr/bin/env bash
# build_kernel_modules.sh – Compile out-of-tree MonoOS kernel modules
# against a pre-built Linux kernel build directory.
#
# Usage:
#   KDIR=/path/to/linux-build ./build/scripts/build_kernel_modules.sh

set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
KDIR="${KDIR:-/lib/modules/$(uname -r)/build}"
MODULE_DIR="$ROOT/kernel"
BUILD_DIR="$ROOT/build/images/modules"

mkdir -p "$BUILD_DIR"

for mod_subdir in \
  core/memory core/process core/scheduler core/syscalls \
  filesystem networking power security; do
  src="$MODULE_DIR/$mod_subdir"
  if [ -f "$src/Makefile" ]; then
    echo "[build] Building kernel module: $mod_subdir"
    make -C "$KDIR" M="$src" modules 2>&1
    find "$src" -name "*.ko" -exec cp {} "$BUILD_DIR/" \;
  fi
done

echo "[build] Kernel modules built to $BUILD_DIR"
