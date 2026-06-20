# aarch64-linux-gnu.cmake – CMake toolchain file for ARM64 Linux target
#
# Set TOOLCHAIN_DIR to the directory containing the unpacked toolchain.
# Default: the directory containing this file.

set(TOOLCHAIN_DIR "${CMAKE_CURRENT_LIST_DIR}/aarch64-linux-gnu"
    CACHE PATH "Path to the aarch64-linux-gnu toolchain directory")

set(CMAKE_SYSTEM_NAME      Linux)
set(CMAKE_SYSTEM_PROCESSOR aarch64)

set(CMAKE_C_COMPILER   "${TOOLCHAIN_DIR}/bin/aarch64-linux-gnu-gcc")
set(CMAKE_CXX_COMPILER "${TOOLCHAIN_DIR}/bin/aarch64-linux-gnu-g++")
set(CMAKE_AR           "${TOOLCHAIN_DIR}/bin/aarch64-linux-gnu-ar")
set(CMAKE_RANLIB       "${TOOLCHAIN_DIR}/bin/aarch64-linux-gnu-ranlib")
set(CMAKE_STRIP        "${TOOLCHAIN_DIR}/bin/aarch64-linux-gnu-strip")
set(CMAKE_LINKER       "${TOOLCHAIN_DIR}/bin/aarch64-linux-gnu-ld")

set(CMAKE_SYSROOT "${TOOLCHAIN_DIR}/sysroot")

set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)

# Kernel-module build flags
set(KBUILD_EXTRA_SYMBOLS "" CACHE STRING "Extra Module.symvers paths")
set(KDIR "/lib/modules/linux-oneos-arm64/build"
    CACHE PATH "Kernel build directory for out-of-tree modules")

# ABI flags required for OneOS ARM64 targets
add_compile_options(
    -march=armv8-a+crc+crypto
    -mtune=cortex-a76
    -mno-outline-atomics
)
