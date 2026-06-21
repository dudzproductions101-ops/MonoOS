//! package_signing.rs – Ed25519 signing and verification for OPK packages.
//!
//! This is the real cryptographic backing for
//! `packages/signatures/signature_verifier.rs`'s trust-store logic: that
//! module decides *which* keys are trusted and tracks fingerprints; this
//! module provides the actual signature math (sign at build time on the
//! developer's machine, verify at install time on-device).
//!
//! Ed25519 (RFC 8032) is used rather than RSA: smaller keys and signatures,
//! fast verification (important on-device at install time), and no padding
//! scheme footguns (no PKCS#1v1.5/PSS parameter confusion).

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey, Signature};
use sha2::{Digest, Sha256};

pub const PUBLIC_KEY_LEN: usize = 32;
pub const SIGNATURE_LEN: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningError {
    InvalidKeyBytes,
    InvalidSignatureBytes,
    VerificationFailed,
}

impl std::fmt::Display for SigningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SigningError::InvalidKeyBytes => write!(f, "invalid Ed25519 key bytes"),
            SigningError::InvalidSignatureBytes => write!(f, "invalid Ed25519 signature bytes"),
            SigningError::VerificationFailed => write!(f, "signature verification failed"),
        }
    }
}
impl std::error::Error for SigningError {}

/// A developer's package-signing keypair. The secret half never leaves the
/// developer's build machine in a real deployment; MonoOS devices only ever
/// see [`PackagePublicKey`].
pub struct PackageSigningKey {
    inner: SigningKey,
}

impl PackageSigningKey {
    /// Generate a new random signing key.
    pub fn generate() -> Self {
        use rand_core::OsRng;
        PackageSigningKey { inner: SigningKey::generate(&mut OsRng) }
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        PackageSigningKey { inner: SigningKey::from_bytes(bytes) }
    }

    pub fn public_key(&self) -> PackagePublicKey {
        PackagePublicKey { inner: self.inner.verifying_key() }
    }

    /// Sign the SHA-256 digest of an OPK package's contents (matching the
    /// `META-INF/MONOOS.SF` digest scheme described in
    /// `signature_verifier.rs`).
    pub fn sign(&self, opk_bytes: &[u8]) -> [u8; SIGNATURE_LEN] {
        let digest = Sha256::digest(opk_bytes);
        self.inner.sign(&digest).to_bytes()
    }
}

/// The public half of a developer's signing key, as distributed to MonoOS
/// (added to the trust store) or embedded in an OPK's signature block.
#[derive(Clone)]
pub struct PackagePublicKey {
    inner: VerifyingKey,
}

impl PackagePublicKey {
    pub fn from_bytes(bytes: &[u8; PUBLIC_KEY_LEN]) -> Result<Self, SigningError> {
        VerifyingKey::from_bytes(bytes)
            .map(|inner| PackagePublicKey { inner })
            .map_err(|_| SigningError::InvalidKeyBytes)
    }

    pub fn to_bytes(&self) -> [u8; PUBLIC_KEY_LEN] {
        self.inner.to_bytes()
    }

    /// SHA-256 fingerprint of the public key, hex-encoded — this is the
    /// string `signature_verifier.rs`'s `TrustedCert::fingerprint` and
    /// `VerifyError::UntrustedKey` carry around.
    pub fn fingerprint_hex(&self) -> String {
        let digest = Sha256::digest(self.inner.as_bytes());
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Verify a signature over an OPK package's contents.
    pub fn verify(&self, opk_bytes: &[u8], signature: &[u8; SIGNATURE_LEN]) -> Result<(), SigningError> {
        let digest = Sha256::digest(opk_bytes);
        let sig = Signature::from_bytes(signature);
        self.inner
            .verify(&digest, &sig)
            .map_err(|_| SigningError::VerificationFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_then_verify_succeeds() {
        let key = PackageSigningKey::generate();
        let pubkey = key.public_key();
        let opk_data = b"fake OPK zip bytes for testing";
        let sig = key.sign(opk_data);
        assert!(pubkey.verify(opk_data, &sig).is_ok());
    }

    #[test]
    fn verify_fails_for_tampered_data() {
        let key = PackageSigningKey::generate();
        let pubkey = key.public_key();
        let sig = key.sign(b"original content");
        assert!(pubkey.verify(b"tampered content", &sig).is_err());
    }

    #[test]
    fn verify_fails_with_wrong_public_key() {
        let key_a = PackageSigningKey::generate();
        let key_b = PackageSigningKey::generate();
        let opk_data = b"some package bytes";
        let sig = key_a.sign(opk_data);
        assert!(key_b.public_key().verify(opk_data, &sig).is_err());
    }

    #[test]
    fn fingerprint_is_deterministic_and_key_dependent() {
        let key_a = PackageSigningKey::from_bytes(&[1u8; 32]);
        let key_b = PackageSigningKey::from_bytes(&[1u8; 32]);
        let key_c = PackageSigningKey::from_bytes(&[2u8; 32]);
        assert_eq!(key_a.public_key().fingerprint_hex(), key_b.public_key().fingerprint_hex());
        assert_ne!(key_a.public_key().fingerprint_hex(), key_c.public_key().fingerprint_hex());
        assert_eq!(key_a.public_key().fingerprint_hex().len(), 64); // 32 bytes hex-encoded
    }

    #[test]
    fn roundtrip_through_bytes() {
        let key = PackageSigningKey::generate();
        let pubkey = key.public_key();
        let bytes = pubkey.to_bytes();
        let restored = PackagePublicKey::from_bytes(&bytes).unwrap();
        assert_eq!(pubkey.fingerprint_hex(), restored.fingerprint_hex());
    }
}
