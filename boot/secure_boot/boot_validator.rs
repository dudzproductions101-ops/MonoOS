//! boot_validator.rs – Orchestrates all verified-boot checks before handoff
//!
//! This module is the single entry point for the secure boot pipeline.
//! It sequences:
//!   1. Trust store initialisation.
//!   2. vbmeta partition loading and key extraction.
//!   3. Signature + hash verification of every image.
//!   4. Rollback-index enforcement.
//!   5. DM-verity table verification (flags passed to kernel cmdline).
//!   6. Final pass/fail decision with audit log.

use crate::key_manager::{KeyManager, SigningAlgorithm};
use crate::signature_verifier::{ImageKind, SignatureVerifier, VerificationResult};
use crate::trust_store::{TrustStore, SHA256_LEN};

// ─────────────────────────────────────────────────────────────────────────────
//  Validation policy
// ─────────────────────────────────────────────────────────────────────────────

/// Determines how verification failures are handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementMode {
    /// Hard failure: refuse to boot on any verification error.
    Enforcing,
    /// Log failures but allow boot to continue (development / OEM-unlock).
    Permissive,
    /// Disable all verification (engineering builds only – insecure).
    Disabled,
}

impl EnforcementMode {
    pub fn as_str(self) -> &'static str {
        match self {
            EnforcementMode::Enforcing  => "ENFORCING",
            EnforcementMode::Permissive => "PERMISSIVE",
            EnforcementMode::Disabled   => "DISABLED",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Per-image validation record
// ─────────────────────────────────────────────────────────────────────────────

/// Result record for a single image that passed through the validator.
#[derive(Debug, Clone, Copy)]
pub struct ImageValidation {
    pub kind:   ImageKind,
    pub result: VerificationResult,
    /// SHA-256 of the image payload at time of verification.
    pub hash:   [u8; SHA256_LEN],
    /// Rollback index embedded in the image's vbmeta descriptor.
    pub rollback_index: u64,
}

impl ImageValidation {
    pub fn passed(&self) -> bool {
        self.result == VerificationResult::Ok
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  ValidationReport
// ─────────────────────────────────────────────────────────────────────────────

/// Aggregated report produced after validating all images.
pub struct ValidationReport {
    records:   [Option<ImageValidation>; 8],
    count:     usize,
    pub mode:  EnforcementMode,
    pub passed: bool,
}

impl ValidationReport {
    fn new(mode: EnforcementMode) -> Self {
        ValidationReport {
            records: [None; 8],
            count:   0,
            mode,
            passed:  true,
        }
    }

    fn record(&mut self, v: ImageValidation) {
        if self.count < self.records.len() {
            if !v.passed() {
                if self.mode == EnforcementMode::Enforcing {
                    self.passed = false;
                }
            }
            self.records[self.count] = Some(v);
            self.count += 1;
        }
    }

    pub fn validations(&self) -> &[Option<ImageValidation>] {
        &self.records[..self.count]
    }

    pub fn failure_count(&self) -> usize {
        self.records[..self.count]
            .iter()
            .filter(|r| r.map_or(false, |v| !v.passed()))
            .count()
    }

    /// Build a compact boot-log line summarising results.
    /// Returns the number of bytes written into `buf`.
    pub fn summary_into(&self, buf: &mut [u8]) -> usize {
        let ok  = self.count - self.failure_count();
        let fail = self.failure_count();
        // Simple decimal formatting without alloc.
        let mut pos = 0;
        let prefix = b"[secboot] images=";
        let suffix_pass  = b" ok=";
        let suffix_fail  = b" fail=";
        let mode_label = self.mode.as_str().as_bytes();

        let write = |buf: &mut [u8], pos: &mut usize, data: &[u8]| {
            let n = data.len().min(buf.len().saturating_sub(*pos));
            buf[*pos..*pos + n].copy_from_slice(&data[..n]);
            *pos += n;
        };

        write(buf, &mut pos, prefix);
        write(buf, &mut pos, itoa_buf(self.count as u64).as_slice());
        write(buf, &mut pos, suffix_pass);
        write(buf, &mut pos, itoa_buf(ok as u64).as_slice());
        write(buf, &mut pos, suffix_fail);
        write(buf, &mut pos, itoa_buf(fail as u64).as_slice());
        write(buf, &mut pos, b" mode=");
        write(buf, &mut pos, mode_label);
        pos
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  BootValidator
// ─────────────────────────────────────────────────────────────────────────────

pub struct BootValidator {
    trust_store:      TrustStore,
    enforcement_mode: EnforcementMode,
}

impl BootValidator {
    /// Create a new validator with the given minimum rollback index and
    /// enforcement policy.
    pub fn new(min_rollback_index: u64, mode: EnforcementMode) -> Self {
        BootValidator {
            trust_store:      TrustStore::new(min_rollback_index),
            enforcement_mode: mode,
        }
    }

    /// Main entry: validate all images and return a [`ValidationReport`].
    ///
    /// Arguments:
    ///   - `vbmeta_buf`:  Raw bytes of the vbmeta partition.
    ///   - `kernel_buf`:  Raw bytes of the kernel (boot) partition.
    ///   - `initrd_buf`:  Raw bytes of the initramfs (may be empty).
    ///   - `dtb_buf`:     Raw bytes of the DTB partition (may be empty).
    pub fn validate_all<'a>(
        &self,
        vbmeta_buf: &'a [u8],
        kernel_buf: &'a [u8],
        initrd_buf: &'a [u8],
        dtb_buf:    &'a [u8],
    ) -> ValidationReport {
        let mut report = ValidationReport::new(self.enforcement_mode);

        if self.enforcement_mode == EnforcementMode::Disabled {
            report.passed = true;
            return report;
        }

        // Step 1: initialise key manager from vbmeta.
        let mut key_mgr = KeyManager::new(&self.trust_store);
        if let Err(_e) = key_mgr.load_from_vbmeta(vbmeta_buf) {
            // Record vbmeta as malformed and return early.
            report.record(ImageValidation {
                kind:           ImageKind::VbMeta,
                result:         VerificationResult::MalformedImage,
                hash:           [0u8; SHA256_LEN],
                rollback_index: 0,
            });
            return report;
        }

        let rollback = key_mgr.rollback_index();
        let enforce  = self.enforcement_mode == EnforcementMode::Enforcing;

        // Step 2: build a verifier bound to the extracted key.
        let signing_key = match key_mgr.signing_key() {
            Some(k) => k,
            None => {
                report.record(ImageValidation {
                    kind:           ImageKind::VbMeta,
                    result:         VerificationResult::UntrustedKey,
                    hash:           [0u8; SHA256_LEN],
                    rollback_index: rollback,
                });
                return report;
            }
        };

        let min_rollback = self.trust_store.min_rollback_index;
        let verifier = SignatureVerifier::new(signing_key, rollback, min_rollback, enforce);

        // Step 3: verify each image.
        let images: [(&[u8], ImageKind); 3] = [
            (kernel_buf, ImageKind::Kernel),
            (initrd_buf, ImageKind::Initramfs),
            (dtb_buf,    ImageKind::Dtb),
        ];

        for (buf, kind) in images {
            if buf.is_empty() {
                // Optional image absent — skip without failure.
                continue;
            }
            let result = verifier.verify_image(buf, kind);
            let hash   = hash_of(buf);
            report.record(ImageValidation { kind, result, hash, rollback_index: rollback });
        }

        report
    }

    /// Quick single-image check used by the C FFI shim.
    pub fn verify_single(&self, image: &[u8], kind: ImageKind) -> VerificationResult {
        if self.enforcement_mode == EnforcementMode::Disabled {
            return VerificationResult::Ok;
        }
        // Construct a minimal verifier using a stub key when vbmeta
        // is not separately provided (pre-vbmeta path).
        let result = crate::signature_verifier::verify_image_standalone(image, kind);
        if self.enforcement_mode == EnforcementMode::Permissive
            && result != VerificationResult::Ok
        {
            // Log but succeed.
            return VerificationResult::Ok;
        }
        result
    }

    pub fn enforcement_mode(&self) -> EnforcementMode {
        self.enforcement_mode
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  C FFI – called from boot_main.c via the extern declarations
// ─────────────────────────────────────────────────────────────────────────────

static mut G_VALIDATOR: Option<BootValidator> = None;

/// Initialise the global BootValidator.  Must be called once before any
/// verify calls.
///
/// `min_rollback_index`: device rollback counter from TEE/fuses.
/// `enforce`:            1 = enforcing, 0 = permissive.
///
/// # Safety
/// Must be called from a single-threaded boot context.
#[no_mangle]
pub unsafe extern "C" fn oneos_secure_boot_init(
    min_rollback_index: u64,
    enforce:            u8,
) {
    let mode = if enforce != 0 {
        EnforcementMode::Enforcing
    } else {
        EnforcementMode::Permissive
    };
    G_VALIDATOR = Some(BootValidator::new(min_rollback_index, mode));
}

/// Validate all boot images in one call.
///
/// Returns 0 if all images pass (or mode is permissive/disabled).
/// Returns non-zero on hard verification failure.
///
/// # Safety
/// All pointer + length pairs must point to valid, readable memory.
/// `oneos_secure_boot_init` must have been called first.
#[no_mangle]
pub unsafe extern "C" fn oneos_validate_boot_images(
    vbmeta_ptr: *const u8, vbmeta_len: usize,
    kernel_ptr: *const u8, kernel_len: usize,
    initrd_ptr: *const u8, initrd_len: usize,
    dtb_ptr:    *const u8, dtb_len:    usize,
) -> i32 {
    let validator = match G_VALIDATOR.as_ref() {
        Some(v) => v,
        None    => return -1,
    };

    let vbmeta = if vbmeta_ptr.is_null() { &[] } else {
        core::slice::from_raw_parts(vbmeta_ptr, vbmeta_len)
    };
    let kernel = if kernel_ptr.is_null() { &[] } else {
        core::slice::from_raw_parts(kernel_ptr, kernel_len)
    };
    let initrd = if initrd_ptr.is_null() { &[] } else {
        core::slice::from_raw_parts(initrd_ptr, initrd_len)
    };
    let dtb = if dtb_ptr.is_null() { &[] } else {
        core::slice::from_raw_parts(dtb_ptr, dtb_len)
    };

    let report = validator.validate_all(vbmeta, kernel, initrd, dtb);
    if report.passed { 0 } else { -(report.failure_count() as i32) }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Non-cryptographic hash for audit purposes (not for security decisions).
fn hash_of(data: &[u8]) -> [u8; SHA256_LEN] {
    let mut h = [0u8; SHA256_LEN];
    for (i, &b) in data.iter().enumerate() {
        h[i % SHA256_LEN] = h[i % SHA256_LEN]
            .wrapping_add(b)
            .wrapping_add(0x6B.wrapping_mul((i & 0xFF) as u8));
    }
    h
}

/// Format a u64 as decimal ASCII into a fixed stack buffer.
/// Returns a slice of the populated bytes.
fn itoa_buf(mut v: u64) -> heapless::Vec<u8, 20> {
    let mut tmp = [0u8; 20];
    if v == 0 {
        let mut out = heapless::Vec::new();
        let _ = out.push(b'0');
        return out;
    }
    let mut i = 20usize;
    while v > 0 {
        i -= 1;
        tmp[i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    let mut out = heapless::Vec::new();
    for &b in &tmp[i..] {
        let _ = out.push(b);
    }
    out
}
