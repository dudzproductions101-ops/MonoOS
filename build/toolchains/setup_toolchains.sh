#!/usr/bin/env bash
# setup_toolchains.sh -- Download and verify OneOS build toolchains.
#
# Usage:
#   ./build/toolchains/setup_toolchains.sh [--all | --toolchain <name>]
#
# Available toolchain names:
#   aarch64-linux-gnu    GCC 13 cross-compiler for ARM64 kernel/drivers
#   llvm-aarch64         Clang 17 for ARM64 userspace
#   x86_64-linux-gnu     GCC 13 for x86-64 emulator target
#   arm-linux-gnueabihf  GCC 13 for 32-bit ARMv7 compat

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CDN="https://build.oneos.io/toolchains/v1"
TOOLCHAINS=(aarch64-linux-gnu llvm-aarch64 x86_64-linux-gnu arm-linux-gnueabihf)

# SHA-256 checksums for each toolchain tarball.
declare -A CHECKSUMS=(
  [aarch64-linux-gnu]="a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
  [llvm-aarch64]="b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3"
  [x86_64-linux-gnu]="c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4"
  [arm-linux-gnueabihf]="d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5"
)

download_toolchain() {
  local name="$1"
  local tarball="${name}.tar.xz"
  local url="${CDN}/${tarball}"
  local dest="${SCRIPT_DIR}/${name}"

  if [[ -d "$dest" ]]; then
    echo "[setup] $name already installed at $dest -- skipping."
    return 0
  fi

  echo "[setup] Downloading $name..."
  curl -fsSL --progress-bar -o "/tmp/${tarball}" "$url"

  echo "[setup] Verifying checksum..."
  local expected="${CHECKSUMS[$name]}"
  local actual
  actual=$(sha256sum "/tmp/${tarball}" | awk '{print $1}')
  if [[ "$actual" != "$expected" ]]; then
    echo "[setup] ERROR: checksum mismatch for $name!" >&2
    echo "[setup]   expected: $expected" >&2
    echo "[setup]   got:      $actual"   >&2
    rm -f "/tmp/${tarball}"
    return 1
  fi

  echo "[setup] Extracting $name..."
  mkdir -p "$dest"
  tar -xf "/tmp/${tarball}" -C "$dest" --strip-components=1
  rm -f "/tmp/${tarball}"
  echo "[setup] $name installed."
}

SELECTED=()
if [[ "${1:-}" == "--all" ]]; then
  SELECTED=("${TOOLCHAINS[@]}")
elif [[ "${1:-}" == "--toolchain" && -n "${2:-}" ]]; then
  SELECTED=("$2")
else
  echo "Usage: $0 [--all | --toolchain <name>]"
  echo "Available: ${TOOLCHAINS[*]}"
  exit 1
fi

for tc in "${SELECTED[@]}"; do
  download_toolchain "$tc"
done

echo "[setup] All requested toolchains installed."
