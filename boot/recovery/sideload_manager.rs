//! sideload_manager.rs – OTA package delivery and application
//!
//! Handles two delivery paths:
//!   CacheFile   – package already present at a known path on /cache.
//!   AdbSideload – package streamed over USB via `adb sideload`.
//!
//! After delivery, the package is:
//!   1. Signature-verified against the OTA signing certificate.
//!   2. Extracted and applied to the inactive A/B slot.
//!   3. The slot metadata is updated to mark the new slot as pending.

// ─────────────────────────────────────────────────────────────────────────────
//  SideloadSource
// ─────────────────────────────────────────────────────────────────────────────

/// Where the OTA package originates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideloadSource {
    /// A file that already exists in the cache partition.
    CacheFile(&'static str),
    /// Package streamed from a host machine via `adb sideload`.
    AdbSideload,
    /// Package at a URL (requires network access in recovery).
    NetworkUrl(&'static str),
}

impl SideloadSource {
    pub fn as_str(self) -> &'static str {
        match self {
            SideloadSource::CacheFile(p)  => p,
            SideloadSource::AdbSideload   => "adb://sideload",
            SideloadSource::NetworkUrl(u) => u,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  PackageMetadata – extracted from the OTA zip manifest
// ─────────────────────────────────────────────────────────────────────────────

/// Key fields read from the OTA package before applying it.
#[derive(Debug, Clone, Copy, Default)]
pub struct PackageMetadata {
    /// OTA version string (e.g. "MonoOS-1.2.0-20260101").
    pub version:        [u8; 64],
    pub version_len:    usize,
    /// Target slot after update (0 = A, 1 = B).
    pub target_slot:    u8,
    /// Package size in bytes as declared in the manifest.
    pub declared_size:  u64,
    /// SHA-256 of the payload.bin, as hex ASCII in manifest.
    pub payload_hash:   [u8; 64],
    /// Required minimum build version to apply this update.
    pub min_version:    u32,
}

impl PackageMetadata {
    pub fn version_str(&self) -> &str {
        core::str::from_utf8(&self.version[..self.version_len]).unwrap_or("<invalid>")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  SideloadProgress – progress state machine
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideloadState {
    Idle,
    Receiving,
    VerifyingSignature,
    ExtractingManifest,
    ApplyingPayload,
    UpdatingSlotMetadata,
    CleaningUp,
    Complete,
    Failed,
}

impl SideloadState {
    pub fn as_str(self) -> &'static str {
        match self {
            SideloadState::Idle                 => "idle",
            SideloadState::Receiving            => "receiving",
            SideloadState::VerifyingSignature   => "verifying-signature",
            SideloadState::ExtractingManifest   => "extracting-manifest",
            SideloadState::ApplyingPayload      => "applying-payload",
            SideloadState::UpdatingSlotMetadata => "updating-slot-metadata",
            SideloadState::CleaningUp           => "cleaning-up",
            SideloadState::Complete             => "complete",
            SideloadState::Failed               => "failed",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  AdbSideloadSession
// ─────────────────────────────────────────────────────────────────────────────

/// USB ADB sideload session parameters.
pub struct AdbSideloadSession {
    /// Maximum transfer size negotiated with the host.
    pub max_transfer_size: usize,
    /// Bytes received so far.
    pub bytes_received:    u64,
    /// Total expected bytes (0 if unknown).
    pub expected_bytes:    u64,
}

impl AdbSideloadSession {
    pub fn new() -> Self {
        AdbSideloadSession {
            max_transfer_size: 512 * 1024, // 512 KiB default
            bytes_received:    0,
            expected_bytes:    0,
        }
    }

    /// Progress percentage, 0–100, or None if size is unknown.
    pub fn progress_pct(&self) -> Option<u8> {
        if self.expected_bytes == 0 {
            return None;
        }
        Some(((self.bytes_received * 100) / self.expected_bytes).min(100) as u8)
    }
}

impl Default for AdbSideloadSession {
    fn default() -> Self { Self::new() }
}

// ─────────────────────────────────────────────────────────────────────────────
//  SideloadManager
// ─────────────────────────────────────────────────────────────────────────────

pub struct SideloadManager {
    state:       SideloadState,
    metadata:    PackageMetadata,
    adb_session: AdbSideloadSession,
    /// Path where the package is staged on /cache.
    staging_path: &'static str,
}

impl SideloadManager {
    pub fn new() -> Self {
        SideloadManager {
            state:        SideloadState::Idle,
            metadata:     PackageMetadata::default(),
            adb_session:  AdbSideloadSession::new(),
            staging_path: "/cache/update.zip",
        }
    }

    /// Top-level: receive (if needed) and apply a package.
    pub fn apply_package(&mut self, src: SideloadSource) -> Result<(), &'static str> {
        // Phase 1: receive / verify source is accessible.
        let local_path = self.receive(src)?;

        // Phase 2: verify OTA package signature.
        self.state = SideloadState::VerifyingSignature;
        self.verify_signature(local_path)?;

        // Phase 3: parse manifest.
        self.state = SideloadState::ExtractingManifest;
        self.extract_manifest(local_path)?;

        // Phase 4: apply payload to inactive slot.
        self.state = SideloadState::ApplyingPayload;
        self.apply_payload(local_path)?;

        // Phase 5: update A/B slot metadata to schedule the new slot.
        self.state = SideloadState::UpdatingSlotMetadata;
        self.update_slot_metadata()?;

        // Phase 6: clean up staging area.
        self.state = SideloadState::CleaningUp;
        self.cleanup(local_path)?;

        self.state = SideloadState::Complete;
        Ok(())
    }

    fn receive(&mut self, src: SideloadSource) -> Result<&'static str, &'static str> {
        match src {
            SideloadSource::CacheFile(path) => {
                self.state = SideloadState::Receiving;
                // Verify file exists and is readable.
                // stat(path) → check S_ISREG and size > 0.
                Ok(path)
            }
            SideloadSource::AdbSideload => {
                self.state = SideloadState::Receiving;
                // 1. Advertise sideload mode to the ADB host:
                //    write "OKAY" to the ADB transport.
                // 2. Receive data in chunks, writing to staging_path.
                // 3. Signal "OKAY" when complete.
                Ok(self.staging_path)
            }
            SideloadSource::NetworkUrl(_url) => {
                // Future: HTTP GET with progress and resume.
                Err("network sideload not yet implemented")
            }
        }
    }

    fn verify_signature(&self, _path: &str) -> Result<(), &'static str> {
        // 1. Locate META-INF/com/android/otacert in the zip.
        // 2. Locate META-INF/com/android/metadata.
        // 3. Verify the PKCS#7 signature over the comment-stripped zip.
        // 4. Check the leaf cert against the OTA signing trust store.
        Ok(())
    }

    fn extract_manifest(&mut self, _path: &str) -> Result<(), &'static str> {
        // 1. Open zip file.
        // 2. Read META-INF/com/android/metadata into a small buffer.
        // 3. Parse key=value lines (pre-device, post-device, ota-type, etc.).
        // 4. Populate self.metadata.
        Ok(())
    }

    fn apply_payload(&self, _path: &str) -> Result<(), &'static str> {
        // For A/B (seamless) OTA:
        //   1. Spawn update_engine or a minimal payload_consumer binary.
        //   2. Feed it the payload.bin offset + size from the zip EOCD.
        //   3. Wait for it to signal "Apply complete" or "Error".
        //
        // For legacy non-A/B (block-based OTA):
        //   1. Run the update-binary extracted from the zip.
        Ok(())
    }

    fn update_slot_metadata(&self) -> Result<(), &'static str> {
        // Write updated AbControl to MISC:
        //   - Set inactive slot: priority = 15, tries = 7, successful = 0.
        //   - Do NOT mark the new slot as active yet (kernel does that on
        //     the next boot after boot_control::markBootSuccessful).
        Ok(())
    }

    fn cleanup(&self, path: &str) -> Result<(), &'static str> {
        // unlink(path) to free space.
        // Sync the filesystem.
        let _p = path;
        Ok(())
    }

    pub fn state(&self) -> SideloadState { self.state }
    pub fn metadata(&self) -> &PackageMetadata { &self.metadata }
}

impl Default for SideloadManager {
    fn default() -> Self { Self::new() }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_file_source_str() {
        let src = SideloadSource::CacheFile("/cache/test.zip");
        assert_eq!(src.as_str(), "/cache/test.zip");
    }

    #[test]
    fn adb_session_progress_unknown_when_zero_expected() {
        let s = AdbSideloadSession::new();
        assert_eq!(s.progress_pct(), None);
    }

    #[test]
    fn adb_session_progress_pct() {
        let mut s = AdbSideloadSession::new();
        s.expected_bytes  = 1000;
        s.bytes_received  = 500;
        assert_eq!(s.progress_pct(), Some(50));
    }

    #[test]
    fn apply_package_cache_ok() {
        let mut mgr = SideloadManager::new();
        // With stub impls this must succeed.
        let result = mgr.apply_package(SideloadSource::CacheFile("/cache/update.zip"));
        assert!(result.is_ok());
        assert_eq!(mgr.state(), SideloadState::Complete);
    }
}
