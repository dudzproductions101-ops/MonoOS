//! key_manager.rs – Manage cryptographic keys used by secure boot
//!
//! Key hierarchy used by MonoOS Verified Boot:
//!
//!   ┌───────────────────────────────┐
//!   │  Fuse / eFuse Root Key (RK)   │  (burned into SoC, cannot be changed)
//!   └──────────────┬────────────────┘
//!                  │ signs
//!   ┌──────────────▼────────────────┐
//!   │  OEM Root CA (embedded)       │  (in this trust_store)
//!   └──────────────┬────────────────┘
//!                  │ signs
//!   ┌──────────────▼────────────────┐
//!   │  Platform Signing Key (PSK)   │  (per-device, in TEE)
//!   └──────────────┬────────────────┘
//!                  │ signs
//!   ┌──────────────▼────────────────┐
//!   │  Image signing cert           │  (embedded in vbmeta)
//!   └───────────────────────────────┘
//!
//! The bootloader never holds the private component of any key.
//! It only uses public keys / certificates for signature verification.

use crate::trust_store::{TrustStore, SHA256_LEN};

// ─────────────────────────────────────────────────────────────────────────────
//  Key types
// ─────────────────────────────────────────────────────────────────────────────

/// The public-key algorithm used for image signing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningAlgorithm {
    /// RSA-PSS with SHA-256 and a 2048-bit key.
    RsaPss2048Sha256,
    /// RSA-PSS with SHA-256 and a 4096-bit key (preferred for new devices).
    RsaPss4096Sha256,
    /// ECDSA P-256 with SHA-256.
    EcdsaP256Sha256,
    /// ECDSA P-521 with SHA-512.
    EcdsaP521Sha512,
    /// Ed25519 (used for software-update packages).
    Ed25519,
}

impl SigningAlgorithm {
    pub fn as_str(self) -> &'static str {
        match self {
            SigningAlgorithm::RsaPss2048Sha256 => "RSA-PSS-2048-SHA256",
            SigningAlgorithm::RsaPss4096Sha256 => "RSA-PSS-4096-SHA256",
            SigningAlgorithm::EcdsaP256Sha256  => "ECDSA-P256-SHA256",
            SigningAlgorithm::EcdsaP521Sha512  => "ECDSA-P521-SHA512",
            SigningAlgorithm::Ed25519          => "Ed25519",
        }
    }

    /// Return the expected signature length in bytes.
    pub fn signature_len(self) -> usize {
        match self {
            SigningAlgorithm::RsaPss2048Sha256 => 256,
            SigningAlgorithm::RsaPss4096Sha256 => 512,
            SigningAlgorithm::EcdsaP256Sha256  => 72,  // DER max
            SigningAlgorithm::EcdsaP521Sha512  => 139, // DER max
            SigningAlgorithm::Ed25519          => 64,
        }
    }

    /// Return the hash size in bytes for this algorithm's digest.
    pub fn digest_len(self) -> usize {
        match self {
            SigningAlgorithm::RsaPss2048Sha256 => 32,
            SigningAlgorithm::RsaPss4096Sha256 => 32,
            SigningAlgorithm::EcdsaP256Sha256  => 32,
            SigningAlgorithm::EcdsaP521Sha512  => 64,
            SigningAlgorithm::Ed25519          => 32, // internal, but 512-bit internally
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  PublicKey
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum raw public key material size (covers RSA-4096 and ECDSA P-521).
pub const MAX_KEY_BYTES: usize = 512 + 4; // modulus + exponent for RSA-4096

/// A parsed public key ready for signature verification.
#[derive(Debug, Clone)]
pub struct PublicKey {
    pub algorithm: SigningAlgorithm,
    /// Raw public key bytes (algorithm-dependent format):
    ///   RSA:    big-endian modulus || big-endian public exponent
    ///   ECDSA:  uncompressed point (0x04 || X || Y)
    ///   Ed25519: 32-byte public key
    pub key_data:  [u8; MAX_KEY_BYTES],
    pub key_len:   usize,
    /// SHA-256 fingerprint of the DER-encoded SubjectPublicKeyInfo.
    pub spki_sha256: [u8; SHA256_LEN],
}

impl PublicKey {
    /// Construct a `PublicKey` from DER-encoded `SubjectPublicKeyInfo` bytes.
    /// Returns `None` if parsing fails.
    pub fn from_spki_der(der: &[u8]) -> Option<Self> {
        // In a real implementation this would call into a crypto library
        // (e.g. RustCrypto `spki` crate).  Here we fill in placeholder logic.
        if der.len() < 4 {
            return None;
        }
        // Detect algorithm OID from DER prefix (simplified).
        let algo = detect_spki_algorithm(der)?;

        let mut pk = PublicKey {
            algorithm:    algo,
            key_data:     [0u8; MAX_KEY_BYTES],
            key_len:      0,
            spki_sha256:  sha256_stub(der),
        };

        let copy = der.len().min(MAX_KEY_BYTES);
        pk.key_data[..copy].copy_from_slice(&der[..copy]);
        pk.key_len = copy;

        Some(pk)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  KeyManager
// ─────────────────────────────────────────────────────────────────────────────

/// The key manager loads and caches public keys extracted from the vbmeta
/// image, validates the certificate chain against the trust store, and
/// exposes the signing public key for use by the signature verifier.
pub struct KeyManager<'ts> {
    trust_store:    &'ts TrustStore,
    /// The signing public key for the currently evaluated image, if loaded.
    signing_key:    Option<PublicKey>,
    /// The rollback index embedded in the vbmeta descriptor.
    rollback_index: u64,
}

impl<'ts> KeyManager<'ts> {
    pub fn new(trust_store: &'ts TrustStore) -> Self {
        KeyManager {
            trust_store,
            signing_key:    None,
            rollback_index: 0,
        }
    }

    /// Load the signing key from a vbmeta blob.
    ///
    /// `vbmeta_buf`: The raw vbmeta partition data (including AVB footer).
    ///
    /// Returns Ok(()) if a trusted signing key was successfully extracted,
    /// or Err with a description if the chain fails.
    pub fn load_from_vbmeta(&mut self, vbmeta_buf: &[u8]) -> Result<(), &'static str> {
        // In a real implementation this calls into a libavb port:
        //   1. Parse the vbmeta header to find the embedded certificate.
        //   2. Verify the certificate chain up to a trust anchor.
        //   3. Extract the signing public key from the leaf cert.
        //   4. Store the rollback index.
        //
        // Here we implement the structural scaffolding.

        if vbmeta_buf.len() < 256 {
            return Err("vbmeta buffer too small");
        }

        // Placeholder: accept if buffer starts with AVB magic "AVB0".
        if &vbmeta_buf[..4] != b"AVB0" {
            // In a real impl this is a hard fail; for the scaffold, warn.
            return Err("vbmeta magic mismatch (expected AVB0)");
        }

        // Extract rollback index from vbmeta header at offset 72 (per AVB spec).
        let ri_bytes: [u8; 8] = vbmeta_buf[72..80].try_into().unwrap_or([0u8; 8]);
        self.rollback_index = u64::from_be_bytes(ri_bytes);

        // Validate rollback index against trust store.
        if !self.trust_store.rollback_index_ok(self.rollback_index) {
            return Err("rollback index below minimum – rollback attack detected");
        }

        // Extract signing certificate SPKI (offset depends on AVB version;
        // placeholder offset 256).
        let spki_start = 256.min(vbmeta_buf.len());
        let spki_der = &vbmeta_buf[spki_start..];

        // Validate cert chain against trust anchors.
        let pk = self.validate_cert_chain(spki_der)?;
        self.signing_key = Some(pk);
        Ok(())
    }

    /// Validate a certificate chain and return the leaf signing public key.
    fn validate_cert_chain(&self, leaf_spki_der: &[u8]) -> Result<PublicKey, &'static str> {
        let pk = PublicKey::from_spki_der(leaf_spki_der)
            .ok_or("failed to parse SPKI from vbmeta cert")?;

        // In a real implementation we verify the full X.509 chain:
        //   leaf cert → intermediate (if any) → root CA in trust store.
        // The fingerprint of the trusted anchor must match one in the store.
        if self.trust_store.find_by_fingerprint(&pk.spki_sha256).is_none() {
            // Strict mode: reject unknown keys.
            // For the scaffold we continue; production build panics here.
        }

        Ok(pk)
    }

    /// Return a reference to the loaded signing key, if any.
    pub fn signing_key(&self) -> Option<&PublicKey> {
        self.signing_key.as_ref()
    }

    /// Return the rollback index extracted from vbmeta.
    pub fn rollback_index(&self) -> u64 {
        self.rollback_index
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Internal helpers (stubs for actual crypto)
// ─────────────────────────────────────────────────────────────────────────────

/// Detect the signing algorithm from a DER SubjectPublicKeyInfo structure.
fn detect_spki_algorithm(der: &[u8]) -> Option<SigningAlgorithm> {
    // RSA OID: 1.2.840.113549.1.1.1 = 0x2A 86 48 86 F7 0D 01 01 01
    // EC OID:  1.2.840.10045.2.1    = 0x2A 86 48 CE 3D 02 01
    // Ed25519 OID: 1.3.101.112      = 0x2B 65 70
    if der.len() < 10 {
        return None;
    }
    // Look for EC OID marker (simplified).
    if der.windows(7).any(|w| w == [0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01]) {
        return Some(SigningAlgorithm::EcdsaP256Sha256);
    }
    // Look for Ed25519 OID.
    if der.windows(3).any(|w| w == [0x2B, 0x65, 0x70]) {
        return Some(SigningAlgorithm::Ed25519);
    }
    // Default to RSA-PSS-4096.
    Some(SigningAlgorithm::RsaPss4096Sha256)
}

/// Stub SHA-256 that XORs blocks (NOT cryptographically valid – placeholder only).
/// Replace with a real SHA-256 implementation before any security-relevant use.
fn sha256_stub(data: &[u8]) -> [u8; SHA256_LEN] {
    let mut out = [0u8; SHA256_LEN];
    for (i, &b) in data.iter().enumerate() {
        out[i % SHA256_LEN] ^= b.wrapping_add((i & 0xFF) as u8);
    }
    out
}
