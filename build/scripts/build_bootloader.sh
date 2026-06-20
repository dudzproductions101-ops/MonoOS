#!/usr/bin/env bash
# build_bootloader.sh – Compile the OneOS C bootloader and link it to the
# Rust boot-manager and secure-boot static libraries.
#
# Requires:
#   - x86_64-elf-gcc (cross-compiler, available via Homebrew or apt)
#   - x86_64-elf-binutils (ld, objcopy)
#   - cargo + rustup with thumbv8m.main-none-eabihf or x86_64-unknown-none target
#   - nasm (or as from binutils) for entry.S
#
# Usage:
#   ./build/scripts/build_bootloader.sh [--release] [--clean]

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# ── Configuration ─────────────────────────────────────────────────────────────
TARGET_TRIPLE="x86_64-unknown-none"
BOOTLOADER_DIR="$ROOT/boot/bootloader"
BOOT_MGR_DIR="$ROOT/boot/boot_manager"
SECURE_BOOT_DIR="$ROOT/boot/secure_boot"
BUILD_DIR="$ROOT/build/images"
RELEASE_DIR="$ROOT/build/release"

CC="${CC:-x86_64-elf-gcc}"
AS="${AS:-x86_64-elf-as}"
LD="${LD:-x86_64-elf-ld}"
OBJCOPY="${OBJCOPY:-x86_64-elf-objcopy}"
CARGO="${CARGO:-cargo}"

CFLAGS="-std=c11 -ffreestanding -fno-builtin -fno-stack-protector \
        -mno-red-zone -mno-mmx -mno-sse -mno-sse2 \
        -Wall -Wextra -Werror \
        -I${BOOTLOADER_DIR}/include"

PROFILE="debug"
CLEAN=0

for arg in "$@"; do
  case "$arg" in
    --release) PROFILE="release" ;;
    --clean)   CLEAN=1 ;;
  esac
done

if [[ "$CLEAN" -eq 1 ]]; then
  echo "[build] Cleaning..."
  rm -rf "$BUILD_DIR"/*.o "$BUILD_DIR"/bootloader.elf "$BUILD_DIR"/bootloader.bin
  (cd "$BOOT_MGR_DIR"   && "$CARGO" clean)
  (cd "$SECURE_BOOT_DIR" && "$CARGO" clean)
fi

mkdir -p "$BUILD_DIR" "$RELEASE_DIR"

CARGO_FLAGS=""
if [[ "$PROFILE" == "release" ]]; then
  CARGO_FLAGS="--release"
  CFLAGS="$CFLAGS -O2 -DNDEBUG"
else
  CFLAGS="$CFLAGS -O0 -g -DDEBUG"
fi

# ── Step 1: Build Rust static libraries ───────────────────────────────────────
echo "[build] Building Rust boot_manager (${PROFILE})..."
(
  cd "$BOOT_MGR_DIR"
  RUSTFLAGS="-C panic=abort -C opt-level=z" \
    "$CARGO" build $CARGO_FLAGS --target "$TARGET_TRIPLE" 2>&1
)

echo "[build] Building Rust secure_boot (${PROFILE})..."
(
  cd "$SECURE_BOOT_DIR"
  RUSTFLAGS="-C panic=abort -C opt-level=z" \
    "$CARGO" build $CARGO_FLAGS --target "$TARGET_TRIPLE" 2>&1
)

RUST_LIB_DIR_MGR="$BOOT_MGR_DIR/target/${TARGET_TRIPLE}/${PROFILE}"
RUST_LIB_DIR_SB="$SECURE_BOOT_DIR/target/${TARGET_TRIPLE}/${PROFILE}"

# ── Step 2: Compile C sources ─────────────────────────────────────────────────
echo "[build] Compiling C bootloader sources..."

C_SOURCES=(
  boot_args.c
  boot_main.c
  cpu_init.c
  initramfs_loader.c
  kernel_loader.c
  memory_map.c
)

OBJECTS=()
for src in "${C_SOURCES[@]}"; do
  obj="$BUILD_DIR/${src%.c}.o"
  echo "  CC $src"
  $CC $CFLAGS -c "$BOOTLOADER_DIR/$src" -o "$obj"
  OBJECTS+=("$obj")
done

# Assemble entry.S
echo "  AS entry.S"
ASM_OBJ="$BUILD_DIR/entry.o"
$AS --64 "$BOOTLOADER_DIR/entry.S" -o "$ASM_OBJ"
OBJECTS+=("$ASM_OBJ")

# ── Step 3: Link ──────────────────────────────────────────────────────────────
echo "[build] Linking..."
ELF="$BUILD_DIR/bootloader.elf"
$LD \
  -T "$BOOTLOADER_DIR/linker.ld" \
  -o "$ELF" \
  "${OBJECTS[@]}" \
  -L"$RUST_LIB_DIR_MGR"   -loneos_boot_manager \
  -L"$RUST_LIB_DIR_SB"    -loneos_secure_boot \
  --no-undefined

echo "[build] Stripping and creating flat binary..."
BIN="$RELEASE_DIR/bootloader.bin"
$OBJCOPY -O binary "$ELF" "$BIN"

SIZE=$(wc -c < "$BIN")
echo "[build] Done. bootloader.bin = ${SIZE} bytes"
echo "[build] Output: $BIN"
