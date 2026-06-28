# MonoOS Roadmap

## Status: pre-alpha / architecture scaffold

MonoOS is a learning/hobby OS project. It is **not yet a bootable,
functional operating system** — see the root `README.md` for the honest
summary. This roadmap tracks what's real vs. planned, so contributors don't
assume more is finished than actually is.

## What currently builds and is tested

- `boot/boot_manager`, `boot/secure_boot` — A/B slot selection, boot state
  persistence, verified-boot orchestration logic (crypto math stubbed, see
  below). 45 tests across `testing/unit` + `testing/integration`.
- `sdk/rust` (`monoos-sdk`) — full safe Rust bindings over the app SDK C
  ABI (context, permissions, storage, notifications, network, audio,
  media, ui), with a mock runtime for host-side testing. 19 tests.
- `sdk/templates/basic_app`, `sdk/templates/media_player_app` — example
  apps, build and link against the mock runtime.
- `security` (`monoos-security`) — tracker blocker, telemetry guard,
  camera/microphone/network monitors, privacy dashboard. 21 tests.
- `security/crypto` (`monoos-crypto`) — AES-256-GCM scoped storage
  encryption, HKDF key derivation, Argon2id passphrase-based key
  derivation, Ed25519 package signing. 18 tests, built on audited
  RustCrypto crates.
- `packages` (`monoos-packages`) — OPK install pipeline, package database,
  repo manager (HTTP + Tor transport), signature trust store. 10 tests.
- Bootloader (`boot/bootloader/*.c`), kernel modules, drivers, HAL, and
  framework C/C++ — syntax-verified and reviewed, not yet build-tested
  against a real kernel source tree (no kernel headers for a matching
  version are available in this dev environment).

## Near-term (next to build)

1. **Root build orchestration** — a single top-level build script/Makefile
   that walks every crate/module in the right order and produces a flashable
   image, instead of building each component separately.
2. **Real verified-boot crypto** — `boot/secure_boot`'s signature/hash
   verification is currently a documented placeholder (`key_manager.rs`,
   `signature_verifier.rs`). Needs a `no_std`-compatible SHA-256 + signature
   implementation (the userspace crypto in `security/crypto` can't be
   reused directly since the bootloader stage can't link `std`).
3. **SQLite-backed package database** — `packages/package_manager` is
   currently in-memory only; `load()`/`save()` are stubs.
4. **App Store** — a system app (`apps/app_store`) on top of the existing
   `packages` crate: browsable catalog (via `repositories::RepoManager`,
   including the existing Tor transport for privacy-preserving discovery),
   permission-gated install/uninstall flow, update checking. The package
   manager backend is ready; the UI and catalog-serving piece are not built
   yet.
5. **Android-parity features not yet started**: app drawer / recents
   screen, intents (inter-app sharing/launch), accessibility services,
   biometric unlock tied into `security/crypto`'s keystore, doze/battery
   optimization, clipboard manager, home-screen widgets, work profiles.
6. **Kernel module build verification** — get an actual matching kernel
   source tree to confirm `kernel/*/Makefile` targets build, not just
   syntax-check.

## Known, intentional placeholders (not bugs)

- `security/crypto`'s `Cargo.toml` pins several dependency versions
  (`ed25519-dalek = "=2.1.1"`, `zeroize = "=1.7.0"`, `base64ct = "=1.6.0"`)
  because this environment's `rustc` (1.75) predates `edition2024`, which
  newer releases of those crates require. If you build with a current
  stable toolchain (1.81+), these pins can likely be relaxed.
- `monoos_sdk`-style mock runtime (`sdk/rust/src/mock_runtime.rs`) is for
  host-side `cargo test` only — never link it into a real device build.
