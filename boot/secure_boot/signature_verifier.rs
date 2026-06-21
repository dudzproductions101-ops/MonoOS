//! signature_verifier.rs – Verify cryptographic signatures on boot images
//!
//! Implements the Android Verified Boot 2.0 (AVB 2.0) signature check
//! protocol:
//!
//!   1. Hash the payload (kernel / initramfs / dtb) with SHA-256 (or SHA-512).
//!   2. Read the signature block appended to the image footer.
//!   3. Verify the signature over the hash using the signing public key from
//!      the vbmeta partition (already extracted by KeyManager).
//!
//! For RSA-PSS and ECDSA, we require a real math library in production
//! (e.g. `rsa`, `p256` from RustCrypto).  This file contains the
//! structural logic and calls into stub verify functions that must be
//! replaced before a production build.

use crate::key_manager::{PublicKey, SigningAlgorithm};
use crate::trust_store::SHA256_LEN;

// ─────────────────────────────────────────────────────────────────────────────
//  Image descriptor
// ─────────────────────────────────────────────────────────────────────────────

/// Which image type is being verified (affects footer location).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Kernel,
    Initramfs,
    Dtb,
    VbMeta,
    RecoveryKernel,
    OtaPackage,
}

impl ImageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ImageKind::Kernel         => "kernel",
            ImageKind::Initramfs      => "initramfs",
            ImageKind::Dtb            => "dtb",
            ImageKind::VbMeta         => "vbmeta",
            ImageKind::RecoveryKernel => "recovery-kernel",
            ImageKind::OtaPackage     => "ota-package",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  AVB image footer (at the end of every signed partition)
// ─────────────────────────────────────────────────────────────────────────────

pub const AVB_FOOTER_MAGIC: &[u8; 4]  = b"AVBf";
pub const AVB_FOOTER_SIZE:  usize     = 64;

/// AVB 2.0 image footer (last 64 bytes of the partition).
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct AvbFooter {
    /// Must be b"AVBf".
    pub magic:               [u8; 4],
    pub version_major:       u32,
    pub version_minor:       u32,
    /// Original image size (before AVB data was appended).
    pub original_image_size: u64,
    /// Byte offset of the vbmeta block from the start of the partition.
    pub vbmeta_offset:       u64,
    /// Size in bytes of the vbmeta block.
    pub vbmeta_size:         u64,
    pub _reserved:           [u8; 28],
}

impl AvbFooter {
    pub fn from_bytes(buf: &[u8; AVB_FOOTER_SIZE]) -> Option<Self> {
        if &buf[..4] != AVB_FOOTER_MAGIC {
            return None;
        }
        // Safety: AVB_FOOTER_SIZE == sizeof(AvbFooter) and buf is aligned.
        let footer = unsafe { *(buf.as_ptr() as *const AvbFooter) };
        Some(footer)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  HashDescriptor (embedded in vbmeta)
// ─────────────────────────────────────────────────────────────────────────────

/// AVB hash descriptor: ties a partition hash to its expected value.
#[derive(Debug, Clone, Copy)]
pub struct HashDescriptor {
    pub partition_name: [u8; 64],
    pub expected_hash:  [u8; 32],
    pub hash_algo:      HashAlgorithm,
    pub flags:          u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    Sha256,
    Sha512,
}

impl HashAlgorithm {
    pub fn digest_len(self) -> usize {
        match self {
            HashAlgorithm::Sha256 => 32,
            HashAlgorithm::Sha512 => 64,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  VerificationResult
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationResult {
    /// Signature and hash match; image is trusted.
    Ok,
    /// vbmeta or footer missing / malformed.
    MalformedImage,
    /// Computed hash does not match the expected value in vbmeta.
    HashMismatch,
    /// Cryptographic signature verification failed.
    SignatureInvalid,
    /// The signing key is not in the trust store.
    UntrustedKey,
    /// Rollback index below the device minimum.
    RollbackViolation,
    /// DM-verity error detected on the partition.
    VerityError,
    /// Internal / unsupported algorithm.
    InternalError,
}

impl VerificationResult {
    pub fn is_ok(self) -> bool { self == VerificationResult::Ok }

    pub fn as_str(self) -> &'static str {
        match self {
            VerificationResult::Ok               => "OK",
            VerificationResult::MalformedImage   => "Malformed image",
            VerificationResult::HashMismatch     => "Hash mismatch",
            VerificationResult::SignatureInvalid => "Signature invalid",
            VerificationResult::UntrustedKey     => "Untrusted signing key",
            VerificationResult::RollbackViolation => "Rollback violation",
            VerificationResult::VerityError      => "DM-verity error",
            VerificationResult::InternalError    => "Internal error",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  SignatureVerifier
// ─────────────────────────────────────────────────────────────────────────────

pub struct SignatureVerifier<'k> {
    signing_key:     &'k PublicKey,
    rollback_index:  u64,
    min_rollback:    u64,
}

impl<'k> SignatureVerifier<'k> {
    pub fn new(
        signing_key:  &'k PublicKey,
        rollback_index: u64,
        min_rollback:   u64,
    ) -> Self {
        SignatureVerifier { signing_key, rollback_index, min_rollback }
    }

    /// Verify a complete image buffer (kernel, initramfs, etc.).
    ///
    /// The function:
    ///   1. Reads the AVB footer from the last 64 bytes.
    ///   2. Locates the vbmeta block inside the image.
    ///   3. Hashes the payload (bytes 0..original_image_size).
    ///   4. Verifies the signature over the vbmeta signing block.
    ///   5. Compares the computed hash against the expected hash in the
    ///      vbmeta hash descriptor.
    pub fn verify_image(
        &self,
        image:     &[u8],
        kind:      ImageKind,
    ) -> VerificationResult {
        // Step 1: Read AVB footer.
        if image.len() < AVB_FOOTER_SIZE {
            return VerificationResult::MalformedImage;
        }
        let footer_bytes: &[u8; AVB_FOOTER_SIZE] = image[image.len() - AVB_FOOTER_SIZE..]
            .try_into()
            .unwrap();
        let footer = match AvbFooter::from_bytes(footer_bytes) {
            Some(f) => f,
            None    => return VerificationResult::MalformedImage,
        };

        let orig_size   = u64::from_be(footer.original_image_size) as usize;
        let vmeta_off   = u64::from_be(footer.vbmeta_offset) as usize;
        let vmeta_size  = u64::from_be(footer.vbmeta_size) as usize;

        if orig_size > image.len() || vmeta_off + vmeta_size > image.len() {
            return VerificationResult::MalformedImage;
        }

        // Step 2: Check rollback index.
        if self.rollback_index < self.min_rollback {
            return VerificationResult::RollbackViolation;
        }

        // Step 3: Hash the payload.
        let payload = &image[..orig_size];
        let computed_hash = sha256_compute(payload);

        // Step 4: Extract expected hash from vbmeta.
        let vbmeta_block = &image[vmeta_off..vmeta_off + vmeta_size];
        let expected_hash = match extract_expected_hash(vbmeta_block, kind) {
            Some(h) => h,
            None    => return VerificationResult::MalformedImage,
        };

        // Step 5: Compare hashes.
        if !constant_time_eq(&computed_hash, &expected_hash[..SHA256_LEN]) {
            return VerificationResult::HashMismatch;
        }

        // Step 6: Verify signature (delegated to algorithm-specific stub).
        let sig_result = self.verify_signature(vbmeta_block);
        if !sig_result {
            return VerificationResult::SignatureInvalid;
        }

        VerificationResult::Ok
    }

    /// Verify the signature field in a vbmeta block.
    fn verify_signature(&self, vbmeta_block: &[u8]) -> bool {
        // In production: call into RustCrypto rsa / p256 crate.
        // For now, stub returns true so the rest of the boot can proceed.
        match self.signing_key.algorithm {
            SigningAlgorithm::RsaPss2048Sha256
            | SigningAlgorithm::RsaPss4096Sha256 => rsa_pss_verify_stub(vbmeta_block, self.signing_key),
            SigningAlgorithm::EcdsaP256Sha256
            | SigningAlgorithm::EcdsaP521Sha512  => ecdsa_verify_stub(vbmeta_block, self.signing_key),
            SigningAlgorithm::Ed25519            => ed25519_verify_stub(vbmeta_block, self.signing_key),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  C FFI exports (called from boot_main.c)
// ─────────────────────────────────────────────────────────────────────────────

/// Verify kernel image.  Returns 0 on success, non-zero on failure.
///
/// # Safety
/// `image` must point to `size` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn monoos_verify_kernel(image: *const u8, size: u64) -> i32 {
    if image.is_null() || size == 0 {
        return -1;
    }
    let buf = core::slice::from_raw_parts(image, size as usize);

    // In a real build we'd look up the key from a global KeyManager
    // initialised earlier by monoos_secure_boot_init().
    // For now: stub key, stub verifier.
    let result = stub_verify(buf, ImageKind::Kernel);
    if result.is_ok() { 0 } else { result as i32 }
}

/// Verify initramfs image.  Returns 0 on success.
///
/// # Safety
/// `image` must point to `size` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn monoos_verify_initramfs(image: *const u8, size: u64) -> i32 {
    if image.is_null() || size == 0 {
        return -1;
    }
    let buf = core::slice::from_raw_parts(image, size as usize);
    let result = stub_verify(buf, ImageKind::Initramfs);
    if result.is_ok() { 0 } else { result as i32 }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Internal stubs (replace with real crypto before production)
// ─────────────────────────────────────────────────────────────────────────────

fn stub_verify(image: &[u8], _kind: ImageKind) -> VerificationResult {
    // Minimal check: if image has AVB footer, accept it.
    // A real implementation uses a KeyManager + SignatureVerifier chain.
    if image.len() < AVB_FOOTER_SIZE {
        // No AVB footer: allow if image has a valid Linux magic.
        if image.len() >= 4 && image[0x1FE..].starts_with(&[0x55, 0xAA]) {
            return VerificationResult::Ok; // bzImage boot flag present
        }
        // Allow zero-size or placeholder images in development builds.
        return VerificationResult::Ok;
    }
    // Stub: always OK in development; replace for production.
    VerificationResult::Ok
}

fn sha256_compute(data: &[u8]) -> [u8; 32] {
    // Stub: NOT a real SHA-256.  Replace with RustCrypto sha2::Sha256.
    let mut out = [0u8; 32];
    for (i, &b) in data.iter().enumerate() {
        out[i % 32] = out[i % 32].wrapping_add(b).wrapping_add((i as u8).wrapping_mul(31));
    }
    out
}

fn extract_expected_hash(vbmeta: &[u8], _kind: ImageKind) -> Option<[u8; 64]> {
    // AVB hash descriptor location in vbmeta is after the header (256 bytes).
    // Stub: return zeros.
    if vbmeta.len() < 64 { return None; }
    Some([0u8; 64])
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut acc: u8 = 0;
    for (&x, &y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}

fn rsa_pss_verify_stub(_data: &[u8], _key: &PublicKey) -> bool { true }
fn ecdsa_verify_stub(_data: &[u8], _key: &PublicKey) -> bool { true }
fn ed25519_verify_stub(_data: &[u8], _key: &PublicKey) -> bool { true }

// ─────────────────────────────────────────────────────────────────────────────
//  Standalone verification (no pre-loaded key manager)
// ─────────────────────────────────────────────────────────────────────────────

/// Verify an image without a pre-extracted signing key.  Used by the
/// `BootValidator::verify_single` path when vbmeta is not available
/// separately (e.g., legacy single-partition layout).
pub fn verify_image_standalone(image: &[u8], kind: ImageKind) -> VerificationResult {
    stub_verify(image, kind)
}
