//! Unit tests for the MonoOS secure boot crate.
//!
//! Run with: cargo test -p secure_boot_tests
//!
//! Note: signature/hash verification in `monoos_secure_boot` is explicitly
//! documented as scaffolding (stub crypto) pending a real RustCrypto
//! integration. These tests exercise the structural logic around it
//! (trust store management, footer parsing, report aggregation, enforcement
//! policy) rather than asserting cryptographic correctness.

#[cfg(test)]
mod trust_store_tests {
    use monoos_secure_boot::trust_store::TrustStore;

    #[test]
    fn new_store_contains_builtin_anchor() {
        let store = TrustStore::new(0);
        assert!(store.count() >= 1, "expected at least the built-in OEM anchor");
    }

    #[test]
    fn rollback_index_ok_respects_minimum() {
        let store = TrustStore::new(10);
        assert!(!store.rollback_index_ok(9));
        assert!(store.rollback_index_ok(10));
        assert!(store.rollback_index_ok(11));
    }

    #[test]
    fn set_min_rollback_updates_threshold() {
        let mut store = TrustStore::new(0);
        store.set_min_rollback(5);
        assert_eq!(store.min_rollback_index, 5);
        assert!(!store.rollback_index_ok(4));
    }

    #[test]
    fn unknown_fingerprint_not_found() {
        let store = TrustStore::new(0);
        let bogus = [0xAAu8; 32];
        assert!(store.find_by_fingerprint(&bogus).is_none());
    }
}

#[cfg(test)]
mod key_manager_tests {
    use monoos_secure_boot::key_manager::{PublicKey, SigningAlgorithm};

    #[test]
    fn signature_len_matches_known_algorithms() {
        assert_eq!(SigningAlgorithm::RsaPss2048Sha256.signature_len(), 256);
        assert_eq!(SigningAlgorithm::RsaPss4096Sha256.signature_len(), 512);
        assert_eq!(SigningAlgorithm::Ed25519.signature_len(), 64);
    }

    #[test]
    fn algorithm_as_str_is_stable() {
        assert_eq!(SigningAlgorithm::Ed25519.as_str(), "Ed25519");
        assert_eq!(SigningAlgorithm::EcdsaP256Sha256.as_str(), "ECDSA-P256-SHA256");
    }

    #[test]
    fn from_spki_der_rejects_short_input() {
        assert!(PublicKey::from_spki_der(&[0u8; 2]).is_none());
    }

    #[test]
    fn from_spki_der_detects_ed25519_oid() {
        // Minimal buffer containing the Ed25519 OID marker (1.3.101.112).
        let mut der = vec![0u8; 16];
        der[5..8].copy_from_slice(&[0x2B, 0x65, 0x70]);
        let pk = PublicKey::from_spki_der(&der).expect("should parse");
        assert_eq!(pk.algorithm, SigningAlgorithm::Ed25519);
    }
}

#[cfg(test)]
mod signature_verifier_tests {
    use monoos_secure_boot::signature_verifier::{
        AvbFooter, ImageKind, VerificationResult, AVB_FOOTER_SIZE,
    };

    #[test]
    fn avb_footer_rejects_bad_magic() {
        let buf = [0u8; AVB_FOOTER_SIZE];
        assert!(AvbFooter::from_bytes(&buf).is_none());
    }

    #[test]
    fn avb_footer_parses_valid_magic() {
        let mut buf = [0u8; AVB_FOOTER_SIZE];
        buf[..4].copy_from_slice(b"AVBf");
        let footer = AvbFooter::from_bytes(&buf).expect("should parse");
        assert_eq!(&footer.magic, b"AVBf");
    }

    #[test]
    fn verification_result_is_ok_only_for_ok_variant() {
        assert!(VerificationResult::Ok.is_ok());
        assert!(!VerificationResult::HashMismatch.is_ok());
        assert!(!VerificationResult::SignatureInvalid.is_ok());
    }

    #[test]
    fn image_kind_labels_are_stable() {
        assert_eq!(ImageKind::Kernel.as_str(), "kernel");
        assert_eq!(ImageKind::OtaPackage.as_str(), "ota-package");
    }
}

#[cfg(test)]
mod boot_validator_tests {
    use monoos_secure_boot::{BootValidator, EnforcementMode};
    use monoos_secure_boot::signature_verifier::{ImageKind, VerificationResult};

    #[test]
    fn disabled_mode_always_passes() {
        let validator = BootValidator::new(0, EnforcementMode::Disabled);
        let report = validator.validate_all(&[], &[], &[], &[]);
        assert!(report.passed);
        assert_eq!(report.mode, EnforcementMode::Disabled);
    }

    #[test]
    fn enforcing_mode_fails_on_malformed_vbmeta() {
        let validator = BootValidator::new(0, EnforcementMode::Enforcing);
        // Empty vbmeta buffer is too small to parse -> MalformedImage.
        let report = validator.validate_all(&[], b"kernel-bytes", &[], &[]);
        assert!(!report.passed);
        assert_eq!(report.failure_count(), 1);
    }

    #[test]
    fn enforcement_mode_labels_are_stable() {
        assert_eq!(EnforcementMode::Enforcing.as_str(), "ENFORCING");
        assert_eq!(EnforcementMode::Permissive.as_str(), "PERMISSIVE");
        assert_eq!(EnforcementMode::Disabled.as_str(), "DISABLED");
    }

    #[test]
    fn verify_single_disabled_mode_returns_ok() {
        let validator = BootValidator::new(0, EnforcementMode::Disabled);
        let result = validator.verify_single(&[], ImageKind::Kernel);
        assert_eq!(result, VerificationResult::Ok);
    }
}
