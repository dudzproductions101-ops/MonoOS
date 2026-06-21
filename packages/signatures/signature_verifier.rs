//! signature_verifier.rs – OPK package signature verifier
//!
//! Verifies PKCS#7 / CMS signatures over OPK packages.
//! The trust store contains OneOS-sanctioned developer certificates;
//! only packages signed by a trusted key are installed by default.
//! OEM-unlock allows sideloading packages with user-provided keys.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    NoSignatureBlock,
    MalformedSignature,
    UntrustedKey { fingerprint: String },
    Expired { since_secs: u64 },
    DigestMismatch,
    InternalError(String),
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifyError::NoSignatureBlock         => write!(f, "no signature block found"),
            VerifyError::MalformedSignature        => write!(f, "signature block is malformed"),
            VerifyError::UntrustedKey { fingerprint } => write!(f, "untrusted key: {fingerprint}"),
            VerifyError::Expired { since_secs }   => write!(f, "certificate expired {since_secs}s ago"),
            VerifyError::DigestMismatch            => write!(f, "content digest mismatch"),
            VerifyError::InternalError(e)          => write!(f, "internal error: {e}"),
        }
    }
}

/// A trusted certificate anchor.
#[derive(Debug, Clone)]
pub struct TrustedCert {
    pub label:       String,
    pub fingerprint: String,   // SHA-256, hex
    pub expires_at:  u64,      // Unix seconds; 0 = no expiry
}

pub struct PackageSignatureVerifier {
    trusted:      HashMap<String, TrustedCert>,
    allow_unknown: bool,   // true when OEM-unlock is active
}

impl PackageSignatureVerifier {
    pub fn new() -> Self {
        let mut v = PackageSignatureVerifier { trusted: HashMap::new(), allow_unknown: false };
        // Built-in OneOS developer cert anchor (placeholder fingerprint).
        v.add_trusted(TrustedCert {
            label:       "OneOS Developer Root".into(),
            fingerprint: "deadbeef00010203040506070809101112131415161718191a1b1c1d1e1f2021".into(),
            expires_at:  0,
        });
        v
    }

    pub fn add_trusted(&mut self, cert: TrustedCert) {
        self.trusted.insert(cert.fingerprint.clone(), cert);
    }

    pub fn set_allow_unknown(&mut self, allow: bool) { self.allow_unknown = allow; }

    /// Verify the signature block attached to an OPK zip.
    ///
    /// `opk_bytes`: full raw bytes of the OPK file.
    pub fn verify(&self, opk_bytes: &[u8]) -> Result<String, VerifyError> {
        // Real implementation:
        //   1. Locate META-INF/ONEOS.RSA in the zip (PKCS#7 DER blob).
        //   2. Locate META-INF/ONEOS.SF  (signature file containing digests).
        //   3. Parse the PKCS#7 and verify the signature over ONEOS.SF.
        //   4. Compute SHA-256 of each manifest entry and compare to ONEOS.SF.
        //   5. Extract the signer certificate's SHA-256 fingerprint.
        //   6. Check fingerprint against self.trusted.

        if opk_bytes.is_empty() {
            return Err(VerifyError::NoSignatureBlock);
        }

        // Stub: accept if the magic "OPK1" header is present.
        if opk_bytes.len() >= 4 && &opk_bytes[..4] == b"OPK1" {
            let fp = "deadbeef00010203040506070809101112131415161718191a1b1c1d1e1f2021";
            if self.trusted.contains_key(fp) || self.allow_unknown {
                return Ok(fp.to_owned());
            } else {
                return Err(VerifyError::UntrustedKey { fingerprint: fp.to_owned() });
            }
        }

        // For test / CI packages that don't start with OPK1, accept if allow_unknown.
        if self.allow_unknown {
            return Ok("user-sideload".to_owned());
        }

        Err(VerifyError::NoSignatureBlock)
    }

    pub fn is_trusted(&self, fingerprint: &str) -> bool {
        self.trusted.contains_key(fingerprint)
    }

    pub fn trusted_count(&self) -> usize { self.trusted.len() }
}

impl Default for PackageSignatureVerifier { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_rejected() {
        let v = PackageSignatureVerifier::new();
        assert!(v.verify(&[]).is_err());
    }

    #[test]
    fn opk1_magic_trusted() {
        let v = PackageSignatureVerifier::new();
        let mut data = b"OPK1".to_vec();
        data.extend(vec![0u8; 100]);
        assert!(v.verify(&data).is_ok());
    }

    #[test]
    fn unknown_allowed_when_oem_unlocked() {
        let mut v = PackageSignatureVerifier::new();
        v.set_allow_unknown(true);
        assert!(v.verify(b"random bytes").is_ok());
    }

    #[test]
    fn unknown_rejected_when_locked() {
        let v = PackageSignatureVerifier::new();
        assert!(v.verify(b"random bytes").is_err());
    }
}
