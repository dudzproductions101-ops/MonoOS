//! trust_store.rs – Immutable trust anchor store for OneOS Verified Boot
//!
//! The trust store holds:
//!   • Root Certificate Authorities (DER-encoded X.509).
//!   • Pre-computed SHA-256 fingerprints for fast lookup.
//!   • The device-specific rollback index minimum (normally stored in TEE).
//!
//! Keys are embedded at build time from the OEM key set.  OEM unlock
//! introduces an additional user-supplied key that is stored in the
//! USERDATA partition (outside the verified boundary).

/// Maximum number of root certificates in the compile-time trust store.
pub const MAX_ROOT_CERTS: usize = 8;

/// SHA-256 digest length in bytes.
pub const SHA256_LEN: usize = 32;

/// Maximum DER certificate size we accept (4 KiB per cert).
pub const MAX_CERT_DER_LEN: usize = 4096;

// ─────────────────────────────────────────────────────────────────────────────
//  TrustAnchor
// ─────────────────────────────────────────────────────────────────────────────

/// A single root CA entry in the trust store.
#[derive(Debug, Clone, Copy)]
pub struct TrustAnchor {
    /// Human-readable label (e.g. "OneOS OEM Root CA 1").
    pub label:          &'static str,
    /// DER-encoded X.509 certificate bytes.
    pub cert_der:       &'static [u8],
    /// Pre-computed SHA-256 fingerprint of `cert_der`.
    pub fingerprint:    [u8; SHA256_LEN],
    /// True if this anchor was added by the user (OEM unlock path).
    pub user_supplied:  bool,
    /// Maximum rollback index this anchor is allowed to sign.
    /// `u64::MAX` means "no limit".
    pub max_rollback:   u64,
}

impl TrustAnchor {
    pub const fn builtin(
        label:       &'static str,
        cert_der:    &'static [u8],
        fingerprint: [u8; SHA256_LEN],
        max_rollback: u64,
    ) -> Self {
        TrustAnchor {
            label,
            cert_der,
            fingerprint,
            user_supplied: false,
            max_rollback,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Compile-time placeholder certificates
//
//  In a real build, these would be replaced by the OEM signing tool with
//  the actual DER bytes of the production root CA certificate.
// ─────────────────────────────────────────────────────────────────────────────

/// Placeholder DER bytes for the OneOS development root CA.
/// REPLACE BEFORE PRODUCTION BUILD.
static OEM_ROOT_CA_DER: &[u8] = b"\x30\x82\x01\x00"; // minimal stub

static OEM_ROOT_CA_FINGERPRINT: [u8; SHA256_LEN] = [
    0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02, 0x03,
    0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13,
    0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B,
];

/// The built-in trust anchors for OneOS.
pub static BUILTIN_ANCHORS: &[TrustAnchor] = &[
    TrustAnchor::builtin(
        "OneOS OEM Root CA 1 (DEV – REPLACE)",
        OEM_ROOT_CA_DER,
        OEM_ROOT_CA_FINGERPRINT,
        u64::MAX,
    ),
];

// ─────────────────────────────────────────────────────────────────────────────
//  TrustStore
// ─────────────────────────────────────────────────────────────────────────────

/// The active trust store for this boot session.  Combines built-in OEM
/// anchors with any user-supplied anchor (if OEM unlock is granted).
pub struct TrustStore {
    /// References to both static and optional runtime anchors.
    anchors:        [Option<&'static TrustAnchor>; MAX_ROOT_CERTS],
    count:          usize,
    /// The device rollback counter retrieved from TEE / fuses.
    pub min_rollback_index: u64,
}

impl TrustStore {
    /// Initialise the store with the built-in anchors.
    pub fn new(min_rollback_index: u64) -> Self {
        let mut store = TrustStore {
            anchors:            [None; MAX_ROOT_CERTS],
            count:              0,
            min_rollback_index,
        };
        for anchor in BUILTIN_ANCHORS.iter() {
            let _ = store.add_anchor(anchor);
        }
        store
    }

    /// Add a trust anchor to the store.  Returns `Err` if full.
    pub fn add_anchor(&mut self, anchor: &'static TrustAnchor) -> Result<(), ()> {
        if self.count >= MAX_ROOT_CERTS {
            return Err(());
        }
        self.anchors[self.count] = Some(anchor);
        self.count += 1;
        Ok(())
    }

    /// Look up an anchor by its SHA-256 fingerprint.
    pub fn find_by_fingerprint(&self, fp: &[u8; SHA256_LEN]) -> Option<&TrustAnchor> {
        for slot in &self.anchors[..self.count] {
            if let Some(a) = slot {
                if &a.fingerprint == fp {
                    return Some(a);
                }
            }
        }
        None
    }

    /// Return all active anchors as a slice.
    pub fn anchors(&self) -> impl Iterator<Item = &TrustAnchor> {
        self.anchors[..self.count]
            .iter()
            .filter_map(|s| s.as_deref())
    }

    /// Return the count of loaded anchors.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Return true if the provided rollback index is acceptable.
    pub fn rollback_index_ok(&self, index: u64) -> bool {
        index >= self.min_rollback_index
    }

    /// Update the minimum rollback index (called after reading fuse values).
    pub fn set_min_rollback(&mut self, idx: u64) {
        self.min_rollback_index = idx;
    }
}
