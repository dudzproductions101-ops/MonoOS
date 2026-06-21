//! keystore.rs – Master key management and per-purpose key derivation.
//!
//! MonoOS derives all symmetric keys from a single 256-bit master key using
//! HKDF-SHA256 with purpose-specific `info` strings, rather than storing
//! many independent keys. This is the same pattern Android's
//! `FileBasedEncryption` / `vold` keystore uses: one root secret, many
//! derived per-context keys, so compromising one derived key does not
//! reveal the others or the root.
//!
//! The master key itself is expected to be sealed by hardware (TEE/StrongBox
//! equivalent) on a real device; here we provide the software-side
//! derivation and an Argon2id-based unlock path for the case where the
//! master key is itself protected by a user passphrase (e.g. first-boot
//! disk encryption password).

use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroize;

pub const MASTER_KEY_LEN: usize = 32;

/// A 256-bit master key. Zeroized automatically when dropped so it never
/// lingers in memory longer than necessary.
#[derive(Clone)]
pub struct MasterKey([u8; MASTER_KEY_LEN]);

impl Drop for MasterKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl std::fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Deliberately redacted: a derived Debug impl would risk leaking
        // key material into logs, panic messages, or `unwrap_err()` output.
        f.write_str("MasterKey([redacted])")
    }
}

impl MasterKey {
    /// Wrap raw key bytes (e.g. unsealed from hardware-backed storage).
    pub fn from_bytes(bytes: [u8; MASTER_KEY_LEN]) -> Self {
        MasterKey(bytes)
    }

    /// Generate a new random master key using the OS CSPRNG.
    pub fn generate() -> Self {
        use rand_core::{OsRng, RngCore};
        let mut bytes = [0u8; MASTER_KEY_LEN];
        OsRng.fill_bytes(&mut bytes);
        MasterKey(bytes)
    }

    /// Derive a purpose-bound 256-bit subkey via HKDF-SHA256.
    ///
    /// `info` should uniquely identify the use case, e.g.
    /// `"monoos:scoped-storage:com.example.app"` or `"monoos:opk-signing"`.
    /// Different info strings always yield unconditionally different,
    /// cryptographically independent keys from the same master key.
    pub fn derive(&self, info: &[u8]) -> [u8; 32] {
        let hk = Hkdf::<Sha256>::new(None, &self.0);
        let mut out = [0u8; 32];
        // Safe to unwrap: 32 bytes is well within HKDF-SHA256's max output
        // (255 * 32 bytes), so expand() cannot fail here.
        hk.expand(info, &mut out).expect("HKDF expand: output length is valid for SHA-256");
        out
    }
}

/// Derive a master key from a user passphrase using Argon2id, suitable for
/// sealing/unsealing the on-disk master key blob with a device PIN or
/// password (first-boot encryption setup, or unlocking after a factory
/// reset migration).
///
/// `salt` must be unique per device/installation and at least 16 bytes;
/// callers should persist it alongside the encrypted master key blob.
pub fn derive_key_from_passphrase(passphrase: &str, salt: &[u8]) -> Result<MasterKey, KeyDerivationError> {
    use argon2::{Argon2, PasswordHasher};
    use argon2::password_hash::SaltString;

    if salt.len() < 16 {
        return Err(KeyDerivationError::SaltTooShort);
    }
    let salt_str = SaltString::encode_b64(salt).map_err(|_| KeyDerivationError::InvalidSalt)?;
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(passphrase.as_bytes(), &salt_str)
        .map_err(|_| KeyDerivationError::HashingFailed)?;
    let raw = hash.hash.ok_or(KeyDerivationError::HashingFailed)?;
    let bytes = raw.as_bytes();
    if bytes.len() < MASTER_KEY_LEN {
        return Err(KeyDerivationError::HashingFailed);
    }
    let mut key = [0u8; MASTER_KEY_LEN];
    key.copy_from_slice(&bytes[..MASTER_KEY_LEN]);
    let result = MasterKey::from_bytes(key);
    // The intermediate stack copy isn't auto-zeroized by argon2's API, so
    // clear our local copy explicitly. `result`'s own buffer is separately
    // owned and protected by ZeroizeOnDrop.
    key.zeroize();
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDerivationError {
    SaltTooShort,
    InvalidSalt,
    HashingFailed,
}

impl std::fmt::Display for KeyDerivationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyDerivationError::SaltTooShort => write!(f, "salt must be at least 16 bytes"),
            KeyDerivationError::InvalidSalt => write!(f, "salt could not be encoded"),
            KeyDerivationError::HashingFailed => write!(f, "Argon2id key derivation failed"),
        }
    }
}
impl std::error::Error for KeyDerivationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_is_deterministic_for_same_info() {
        let mk = MasterKey::from_bytes([7u8; MASTER_KEY_LEN]);
        let a = mk.derive(b"monoos:scoped-storage:com.example.app");
        let b = mk.derive(b"monoos:scoped-storage:com.example.app");
        assert_eq!(a, b);
    }

    #[test]
    fn derive_differs_across_info_strings() {
        let mk = MasterKey::from_bytes([7u8; MASTER_KEY_LEN]);
        let a = mk.derive(b"monoos:scoped-storage:com.app.one");
        let b = mk.derive(b"monoos:scoped-storage:com.app.two");
        assert_ne!(a, b);
    }

    #[test]
    fn derive_differs_across_master_keys() {
        let mk1 = MasterKey::from_bytes([1u8; MASTER_KEY_LEN]);
        let mk2 = MasterKey::from_bytes([2u8; MASTER_KEY_LEN]);
        assert_ne!(mk1.derive(b"same-info"), mk2.derive(b"same-info"));
    }

    #[test]
    fn generate_produces_distinct_keys() {
        let a = MasterKey::generate();
        let b = MasterKey::generate();
        // Astronomically unlikely to collide; confirms randomness is wired up.
        assert_ne!(a.derive(b"x"), b.derive(b"x"));
    }

    #[test]
    fn passphrase_derivation_is_deterministic_for_same_salt() {
        let salt = b"0123456789abcdef";
        let k1 = derive_key_from_passphrase("correct horse battery staple", salt).unwrap();
        let k2 = derive_key_from_passphrase("correct horse battery staple", salt).unwrap();
        assert_eq!(k1.derive(b"x"), k2.derive(b"x"));
    }

    #[test]
    fn passphrase_derivation_differs_for_different_passphrases() {
        let salt = b"0123456789abcdef";
        let k1 = derive_key_from_passphrase("password one", salt).unwrap();
        let k2 = derive_key_from_passphrase("password two", salt).unwrap();
        assert_ne!(k1.derive(b"x"), k2.derive(b"x"));
    }

    #[test]
    fn short_salt_rejected() {
        assert_eq!(
            derive_key_from_passphrase("pw", b"short").unwrap_err(),
            KeyDerivationError::SaltTooShort
        );
    }
}
