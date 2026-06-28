#!/usr/bin/env bash
# build_kernel_modules.sh – Compile out-of-tree MonoOS kernel modules
# against a pre-built Linux kernel build directory, then optionally sign them.
#
# Usage:
#   KDIR=/path/to/linux-build ./build/scripts/build_kernel_modules.sh
#
# Module signing (removes the (OE) taint on Secure Boot devices):
#   The script auto-generates a build keypair on first run and stores it in
#   build/keys/signing_key.{pem,x509}.  To use it on a real device, add the
#   x509 cert to the kernel's trusted keyring (CONFIG_SYSTEM_TRUSTED_KEYS).
#
#   Set SIGN=0 to disable signing (e.g. in CI where the key isn't available).

set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
KDIR="${KDIR:-/lib/modules/$(uname -r)/build}"
MODULE_DIR="$ROOT/kernel"
BUILD_DIR="$ROOT/build/images/modules"
KEY_DIR="$ROOT/build/keys"
SIGN="${SIGN:-1}"

mkdir -p "$BUILD_DIR" "$KEY_DIR"

# ── Key generation ────────────────────────────────────────────────────────────
# Generate a signing keypair once; reuse on subsequent builds.
SIGNING_KEY="$KEY_DIR/signing_key.pem"
SIGNING_CERT="$KEY_DIR/signing_key.x509"

if [[ "$SIGN" -eq 1 ]]; then
  if [[ ! -f "$SIGNING_KEY" || ! -f "$SIGNING_CERT" ]]; then
    echo "[build] Generating module signing key (first run)…"
    openssl req -new -x509 \
      -newkey rsa:2048 \
      -keyout "$SIGNING_KEY" \
      -out    "$SIGNING_CERT" \
      -days   3650 \
      -subj   "/CN=MonoOS Module Signing Key/" \
      -nodes  2>/dev/null
    echo "[build] Signing key written to $KEY_DIR"
    echo "[build] To enroll on a Secure Boot device:"
    echo "[build]   Add $SIGNING_CERT to CONFIG_SYSTEM_TRUSTED_KEYS in kernel .config"
  fi

  # Locate the sign-file utility that ships with the kernel build tree.
  SIGN_FILE="$KDIR/scripts/sign-file"
  if [[ ! -x "$SIGN_FILE" ]]; then
    echo "[build] WARNING: $SIGN_FILE not found — skipping signing."
    echo "[build]   Modules will load with (OE) taint on this kernel."
    SIGN=0
  fi
fi

# ── Build loop ────────────────────────────────────────────────────────────────
for mod_subdir in \
  core/memory core/process core/scheduler core/syscalls \
  filesystem networking power security; do
  src="$MODULE_DIR/$mod_subdir"
  if [ -f "$src/Makefile" ]; then
    echo "[build] Building kernel module: $mod_subdir"
    make -C "$KDIR" M="$src" modules 2>&1
    find "$src" -name "*.ko" | while read -r ko; do
      cp "$ko" "$BUILD_DIR/"
      dst="$BUILD_DIR/$(basename "$ko")"

      if [[ "$SIGN" -eq 1 ]]; then
        "$SIGN_FILE" sha256 "$SIGNING_KEY" "$SIGNING_CERT" "$dst" 2>/dev/null \
          && echo "[build]   signed: $(basename "$dst")" \
          || echo "[build]   sign failed for $(basename "$dst") — continuing"
      fi
    done
  fi
done

echo "[build] Kernel modules built to $BUILD_DIR"
if [[ "$SIGN" -eq 1 ]]; then
  echo "[build] Modules signed with $SIGNING_CERT"
fi
