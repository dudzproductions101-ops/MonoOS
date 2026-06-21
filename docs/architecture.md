# MonoOS Architecture

MonoOS is a Linux-kernel-based mobile operating system, structured similarly
to AOSP: a C kernel layer, a HAL/framework layer in C++, system services and
the package manager in Rust, and applications in QML + Rust.

This document is a map of the tree, not a guarantee that every subsystem is
production-complete — see `docs/roadmap.md` for what's implemented vs.
planned.

## Layers, bottom to top

```
+-----------------------------------------------------------+
| apps/            QML system apps (Settings, Files, ...)   |
| sdk/             Public app SDK (C ABI + Rust bindings)   |
+-----------------------------------------------------------+
| framework/        App framework services (permissions,    |
| services/         notifications, accounts, packages...)   |
| packages/          OPK package manager, installer, repos  |
| security/          Privacy engine + encryption (crypto/)  |
| networking/        Captive portal, VPN, DNS               |
| telephony/         RIL, SMS, call stack                   |
+-----------------------------------------------------------+
| multimedia/        Audio/video/camera/graphics pipelines  |
| hal/                Hardware abstraction layer (C++)      |
+-----------------------------------------------------------+
| init/               PID 1, early_init, service_loader     |
| drivers/            Kernel drivers (modem, audio, camera) |
| kernel/             Linux kernel modules (scheduler, mm,  |
|                      filesystem, security/LSM, power, net)|
| boot/               Bootloader, secure boot, boot manager |
+-----------------------------------------------------------+
```

## Directory guide

- **`boot/`** - `bootloader/` (C, freestanding x86_64-unknown-none target),
  `boot_manager/` and `secure_boot/` (Rust, `no_std`, linked into the
  bootloader as static libraries - see `build/scripts/build_bootloader.sh`),
  `recovery/`.
- **`kernel/`** - out-of-tree Linux kernel modules in C: `core/` (scheduler,
  process, memory, syscalls), `filesystem/` (VFS overlay for scoped
  storage), `networking/` (privacy-focused netfilter hooks), `power/`,
  `security/` (the `monoos_lsm` Linux Security Module).
- **`drivers/`** - kernel drivers for modem, audio, display, touchscreen,
  sensors, storage, camera.
- **`init/`** - `early_init/` (PID 1, mounts, mode setup) and
  `service_loader/` (spawns and supervises system services).
- **`hal/`** - C++ hardware abstraction interfaces (`IAudioHal`, etc.)
  loaded at runtime via `dlopen()` from `/vendor/lib64/hw/*.so`.
- **`multimedia/`** - camera pipeline, audio engine, Vulkan graphics
  context.
- **`framework/`** - app-facing framework services: permissions,
  application lifecycle, notifications, graphics (`render_engine.cpp`),
  storage, accounts, packages, multimedia.
- **`packages/`** (crate `monoos-packages`) - the OPK package format
  pipeline: `installer/` (verify -> extract -> register), `package_manager/`
  (installed-package database), `repositories/` (repo sync, including Tor
  transport), `signatures/` (trust store + signature verification, backed
  by `security/crypto`).
- **`security/`** (crate `monoos-security`) - the privacy engine:
  `tracker_blocker/`, `telemetry_guard/`, `camera_monitor/`,
  `microphone_monitor/`, `network_monitor/`, `privacy_dashboard/`, plus
  **`security/crypto/`** (crate `monoos-crypto`): AES-256-GCM scoped
  storage encryption, HKDF key derivation, Ed25519 package signing -
  built on audited RustCrypto crates, not hand-rolled.
- **`networking/`** - captive portal detection and related network glue.
- **`telephony/`** - cellular/SMS/call stack.
- **`sdk/`** - the public app SDK: `api/*.h` (the C ABI every app links
  against), `rust/` (crate `monoos-sdk`, safe Rust bindings over that C ABI,
  with a `mock-runtime` feature for host-side testing), `templates/`
  (starter apps), `tools/`, `documentation/`.
- **`apps/`** - first-party QML system apps.
- **`ui/`**, **`assets/`** - system UI shell and shared assets (fonts,
  icons, splash).
- **`testing/`** - `unit/` and `integration/` (Rust, `cargo test`-able),
  `hardware/` (on-device diagnostic tools), `kernel/` (KUnit suites),
  `ui/` (Playwright/QtTest).
- **`tools/`** - developer tools: flashing, debugging (`monoos_logcat`,
  `monoos_dumpsys`, `monoos_strace`), profiling, diagnostics.
- **`build/`** - toolchain setup and build scripts.

## Build system

There is no single root build command yet (see roadmap). Each Rust
component is its own Cargo crate/workspace:

| Path | What it builds |
|---|---|
| `boot/boot_manager`, `boot/secure_boot` | `no_std` staticlibs linked into the bootloader ELF |
| `testing/unit`, `testing/integration`, `testing/hardware` | Test workspaces - this is where boot/secure_boot logic is actually exercised with `std` |
| `sdk/rust` | The app SDK (`monoos-sdk`) |
| `sdk/templates/*` | Example apps (`cdylib`s loaded by the runtime) |
| `security`, `security/crypto` | Privacy engine + crypto primitives |
| `packages` | Package manager |

C/C++ components build via the Makefiles next to each module
(`kernel/*/Makefile` for out-of-tree kernel modules, standard `kbuild`
conventions) or `build/scripts/build_bootloader.sh` for the bootloader.

## Security model

- **Verified boot**: `boot/secure_boot` implements the AVB-style pipeline
  (vbmeta -> key extraction -> per-image signature + hash verification ->
  rollback-index enforcement) with three enforcement modes (Enforcing /
  Permissive / Disabled). The actual signature/hash math is currently a
  documented placeholder pending a `no_std`-compatible crypto integration
  for the bootloader stage specifically (the userspace-side crypto, used
  for package signing and storage encryption, is real - see below).
- **Package signing**: OPK packages are signed with Ed25519
  (`security/crypto::package_signing`), verified against a trust store
  (`packages/signatures`) before install.
- **Scoped storage encryption**: every app's private files directory is
  encrypted at rest with AES-256-GCM, using a key derived via HKDF-SHA256
  from the device master key and the app's package name
  (`security/crypto::file_crypto`). No per-app key is stored separately.
- **Privacy engine**: tracker/telemetry blocking, and indicator-light-style
  monitors for camera/microphone/network access, all surfaced through
  `privacy_dashboard`.

## Conventions worth knowing before you edit

- Several Rust crates point `[lib] path` at a non-conventional file (e.g.
  `boot_manager.rs` instead of `src/lib.rs`) to avoid restructuring
  directories that predate the crate wiring. New crates should prefer the
  conventional `src/lib.rs` layout unless there's a reason not to.
- `no_std` crates (`boot_manager`, `secure_boot`) gate `std` behind a Cargo
  feature (`std`), not `cfg(test)` - `cfg(test)` does not apply when the
  crate is built as its declared `staticlib` artifact, which happens
  unconditionally. See the comment in either crate's `Cargo.toml`.
