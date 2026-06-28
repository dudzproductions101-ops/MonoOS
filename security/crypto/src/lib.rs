//! monoos-crypto – Encryption primitives for MonoOS.
//!
//! Three layers, each independently usable:
//!   - [`keystore`]: master key management and HKDF-based key derivation.
//!   - [`file_crypto`]: AES-256-GCM encryption for per-app scoped storage.
//!   - [`package_signing`]: Ed25519 signing/verification for OPK packages.
//!
//! Built entirely on audited RustCrypto crates (`sha2`, `aes-gcm`, `hkdf`,
//! `ed25519-dalek`, `argon2`) rather than hand-rolled cryptography.

pub mod file_crypto;
pub mod keystore;
pub mod package_signing;

pub use file_crypto::{derive_app_key, CryptoError, ScopedStorageKey};
pub use keystore::{derive_key_from_passphrase, KeyDerivationError, MasterKey};
pub use package_signing::{PackagePublicKey, PackageSigningKey, SigningError};
