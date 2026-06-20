# llvm-aarch64.cmake – CMake toolchain file for Clang/LLVM ARM64 userspace
#
# Used for all Rust-backed C FFI libraries and the system UI renderer.

set(LLVM_TOOLCHAIN_DIR "${CMAKE_CURRENT_LIST_DIR}/llvm-aarch64"
    CACHE PATH "Path to the LLVM/Clang aarch64 toolchain")

set(CMAKE_SYSTEM_NAME      Linux)
set(CMAKE_SYSTEM_PROCESSOR aarch64)

set(CMAKE_C_COMPILER   "${LLVM_TOOLCHAIN_DIR}/bin/clang")
set(CMAKE_CXX_COMPILER "${LLVM_TOOLCHAIN_DIR}/bin/clang++")
set(CMAKE_AR           "${LLVM_TOOLCHAIN_DIR}/bin/llvm-ar")
set(CMAKE_RANLIB       "${LLVM_TOOLCHAIN_DIR}/bin/llvm-ranlib")
set(CMAKE_STRIP        "${LLVM_TOOLCHAIN_DIR}/bin/llvm-strip")
set(CMAKE_LINKER       "${LLVM_TOOLCHAIN_DIR}/bin/ld.lld")

set(CMAKE_C_COMPILER_TARGET   "aarch64-linux-gnu")
set(CMAKE_CXX_COMPILER_TARGET "aarch64-linux-gnu")

set(CMAKE_SYSROOT "${LLVM_TOOLCHAIN_DIR}/sysroot")

set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)

# LTO and hardening flags for release builds
set(CMAKE_C_FLAGS_RELEASE   "-O2 -flto=thin -fstack-protector-strong -D_FORTIFY_SOURCE=2")
set(CMAKE_CXX_FLAGS_RELEASE "-O2 -flto=thin -fstack-protector-strong -D_FORTIFY_SOURCE=2")
set(CMAKE_EXE_LINKER_FLAGS_RELEASE    "-Wl,-z,relro,-z,now -fuse-ld=lld")
set(CMAKE_SHARED_LINKER_FLAGS_RELEASE "-Wl,-z,relro,-z,now -fuse-ld=lld")
