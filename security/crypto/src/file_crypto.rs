//! file_crypto.rs – Per-app scoped storage encryption (AES-256-GCM).
//!
//! Every app's private files directory (`monoos_sdk::storage::files_dir()`)
//! is encrypted at rest with a key derived from the device master key and
//! the app's package name, so:
//!   - Files are unreadable if the storage medium is removed/imaged.
//!   - One app's compromised key never exposes another app's files (each
//!     package gets a cryptographically independent derived key).
//!   - No per-app key needs to be separately generated, stored, or
//!     backed up — it's always re-derivable from the master key.
//!
//! Format: each encrypted file is `[12-byte nonce][ciphertext][16-byte tag]`.
//! A fresh random nonce is generated per encryption call (AES-GCM requires
//! nonce uniqueness per key; reuse would catastrophically break
//! confidentiality and integrity).

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use crate::keystore::MasterKey;

pub const NONCE_LEN: usize = 12;
pub const TAG_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    EncryptionFailed,
    DecryptionFailed,
    CiphertextTooShort,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::EncryptionFailed => write!(f, "encryption failed"),
            CryptoError::DecryptionFailed => write!(f, "decryption failed (wrong key, or data was tampered with)"),
            CryptoError::CiphertextTooShort => write!(f, "ciphertext too short to contain a nonce"),
        }
    }
}
impl std::error::Error for CryptoError {}

/// A key scoped to one app package, derived from the device master key.
/// Construct via [`derive_app_key`] rather than directly.
pub struct ScopedStorageKey {
    cipher: Aes256Gcm,
}

/// Derive the AES-256-GCM key used to encrypt `package_name`'s scoped
/// storage. Deterministic: calling this again with the same master key and
/// package name always yields a key that can decrypt previously-encrypted
/// files for that package.
pub fn derive_app_key(master: &MasterKey, package_name: &str) -> ScopedStorageKey {
    let info = format!("monoos:scoped-storage:{package_name}");
    let raw = master.derive(info.as_bytes());
    let key = Key::<Aes256Gcm>::from_slice(&raw);
    ScopedStorageKey { cipher: Aes256Gcm::new(key) }
}

impl ScopedStorageKey {
    /// Encrypt a file's plaintext contents. Returns `nonce || ciphertext || tag`.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)?;
        let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Decrypt data previously produced by [`encrypt`]. Fails (rather than
    /// returning garbage) if the key is wrong or the data was modified —
    /// AES-GCM's authentication tag makes tampering detectable.
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if data.len() < NONCE_LEN + TAG_LEN {
            return Err(CryptoError::CiphertextTooShort);
        }
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_master() -> MasterKey {
        MasterKey::from_bytes([0x42u8; 32])
    }

    #[test]
    fn encrypt_then_decrypt_roundtrips() {
        let key = derive_app_key(&test_master(), "com.example.app");
        let plaintext = b"hello, private app data!";
        let encrypted = key.encrypt(plaintext).unwrap();
        let decrypted = key.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn ciphertext_is_not_plaintext() {
        let key = derive_app_key(&test_master(), "com.example.app");
        let plaintext = b"sensitive data here";
        let encrypted = key.encrypt(plaintext).unwrap();
        assert_ne!(&encrypted[NONCE_LEN..], plaintext.as_slice());
    }

    #[test]
    fn two_encryptions_of_same_plaintext_differ() {
        // Nonces must be random per call, so ciphertexts must differ even
        // for identical plaintext under the same key.
        let key = derive_app_key(&test_master(), "com.example.app");
        let a = key.encrypt(b"same plaintext").unwrap();
        let b = key.encrypt(b"same plaintext").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn different_packages_get_independent_keys() {
        let master = test_master();
        let key_a = derive_app_key(&master, "com.app.a");
        let key_b = derive_app_key(&master, "com.app.b");
        let encrypted = key_a.encrypt(b"secret").unwrap();
        // App B's key must not be able to decrypt App A's data.
        assert!(key_b.decrypt(&encrypted).is_err());
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let key = derive_app_key(&test_master(), "com.example.app");
        let mut encrypted = key.encrypt(b"integrity matters").unwrap();
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xFF; // flip bits in the auth tag
        assert_eq!(key.decrypt(&encrypted).unwrap_err(), CryptoError::DecryptionFailed);
    }

    #[test]
    fn truncated_ciphertext_rejected() {
        let key = derive_app_key(&test_master(), "com.example.app");
        assert_eq!(key.decrypt(&[0u8; 5]).unwrap_err(), CryptoError::CiphertextTooShort);
    }
}
