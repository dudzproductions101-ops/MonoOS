# OneOS Build Toolchains

This directory contains toolchain descriptor files used by the OneOS build
system.  Actual toolchain binaries are **not** committed to the repository;
they are fetched by `build/scripts/setup_toolchains.sh`.

## Supported toolchains

| Name | Target | Host | Used for |
|------|--------|------|---------|
| `aarch64-linux-gnu` | ARM64 Linux | x86-64 Linux/macOS | Kernel, kernel modules, C HAL drivers |
| `llvm-aarch64` | ARM64 | x86-64 | Rust/Clang userspace binaries |
| `x86_64-linux-gnu` | x86-64 Linux | x86-64 Linux | Emulator / CI target |
| `arm-linux-gnueabihf` | ARMv7-A | x86-64 Linux | 32-bit compat libraries |

## Toolchain descriptor format

Each toolchain is described by a `.cmake` toolchain file and a
corresponding `.env` shell snippet that sets `CC`, `CXX`, `AR`, `LD`
and `SYSROOT` for non-CMake build systems.

## Setup

```bash
./build/scripts/setup_toolchains.sh --all
```

Or for a single toolchain:

```bash
./build/scripts/setup_toolchains.sh --toolchain aarch64-linux-gnu
```

The script downloads pre-built Clang/LLVM 17 and GCC 13 cross-compiler
tarballs from the OneOS build infrastructure CDN, verifies their SHA-256
checksums, and unpacks them into this directory.
