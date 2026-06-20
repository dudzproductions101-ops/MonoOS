//! backup_restore.rs – User data backup and restore in recovery
//!
//! Provides a lightweight backup format (OneOS Backup Archive, `.oba`)
//! compatible with the OneOS backup service.  In recovery, backups
//! are used to restore user data after a factory reset, or to migrate
//! data to a new device.
//!
//! Archive format (.oba):
//!   [0..4]   Magic: b"OBA1"
//!   [4..8]   Header length (u32 LE)
//!   [8..N]   JSON-encoded manifest (device, timestamp, content list)
//!   [N..]    Sequence of entries:
//!              [entry_header][compressed_payload]
//!   End:     SHA-256 of everything preceding (32 bytes)

// ─────────────────────────────────────────────────────────────────────────────
//  Archive constants
// ─────────────────────────────────────────────────────────────────────────────

pub const OBA_MAGIC: &[u8; 4] = b"OBA1";
pub const OBA_MAX_MANIFEST_LEN: usize = 65536; // 64 KiB
pub const OBA_ENTRY_HEADER_LEN: usize = 128;

// ─────────────────────────────────────────────────────────────────────────────
//  BackupContent – what is included in a backup set
// ─────────────────────────────────────────────────────────────────────────────

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BackupContent: u32 {
        /// /data/data – app private data.
        const APP_DATA       = 1 << 0;
        /// /data/app  – APK files.
        const APP_PACKAGES   = 1 << 1;
        /// Contacts database.
        const CONTACTS       = 1 << 2;
        /// SMS / MMS messages.
        const MESSAGES       = 1 << 3;
        /// Call log.
        const CALL_LOG       = 1 << 4;
        /// /sdcard/DCIM – photos and videos.
        const MEDIA          = 1 << 5;
        /// /sdcard/Documents, Downloads, etc.
        const DOCUMENTS      = 1 << 6;
        /// System settings and Wi-Fi credentials.
        const SETTINGS       = 1 << 7;
        /// Accounts and credentials (encrypted with device key).
        const ACCOUNTS       = 1 << 8;
    }
}

impl Default for BackupContent {
    fn default() -> Self {
        BackupContent::APP_DATA
            | BackupContent::CONTACTS
            | BackupContent::MESSAGES
            | BackupContent::CALL_LOG
            | BackupContent::SETTINGS
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  BackupMetadata
// ─────────────────────────────────────────────────────────────────────────────

/// Metadata for a single backup archive.
#[derive(Debug, Clone, Copy)]
pub struct BackupMetadata {
    /// Unix timestamp (seconds since epoch) when the backup was created.
    pub created_at:    u64,
    /// OneOS version string at time of backup (e.g. "1.2.0").
    pub os_version:    [u8; 32],
    pub os_version_len: usize,
    /// Device model string.
    pub device_model:  [u8; 64],
    pub device_model_len: usize,
    /// Content flags.
    pub content:       BackupContent,
    /// Uncompressed total size in bytes.
    pub total_size:    u64,
    /// Number of individual entries.
    pub entry_count:   u32,
    /// CRC-32 of the entire archive (including manifest).
    pub archive_crc32: u32,
}

impl BackupMetadata {
    pub fn os_version_str(&self) -> &str {
        core::str::from_utf8(&self.os_version[..self.os_version_len]).unwrap_or("")
    }
    pub fn device_model_str(&self) -> &str {
        core::str::from_utf8(&self.device_model[..self.device_model_len]).unwrap_or("")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  BackupEntry  (one file or database within the archive)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct BackupEntry {
    /// Relative path inside the archive (e.g. "app_data/com.example.app/databases/main.db").
    pub path:             [u8; 256],
    pub path_len:         usize,
    /// Compression algorithm (0 = none, 1 = zstd, 2 = lz4).
    pub compression:      u8,
    /// Original size in bytes.
    pub original_size:    u64,
    /// Compressed size in bytes.
    pub compressed_size:  u64,
    /// SHA-256 of the original (uncompressed) data.
    pub sha256:           [u8; 32],
    /// Unix file mode.
    pub mode:             u32,
    /// Modification time (Unix seconds).
    pub mtime:            u64,
}

impl BackupEntry {
    pub fn path_str(&self) -> &str {
        core::str::from_utf8(&self.path[..self.path_len]).unwrap_or("")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  BackupRestoreManager
// ─────────────────────────────────────────────────────────────────────────────

pub struct BackupRestoreManager {
    /// Search path for backup archives (on /sdcard or /cache).
    backup_dirs: [&'static str; 4],
    backup_dir_count: usize,
}

impl BackupRestoreManager {
    pub fn new() -> Self {
        let mut mgr = BackupRestoreManager {
            backup_dirs:      [""; 4],
            backup_dir_count: 0,
        };
        mgr.add_backup_dir("/sdcard/OneOS/backups");
        mgr.add_backup_dir("/cache/backups");
        mgr
    }

    pub fn add_backup_dir(&mut self, dir: &'static str) {
        if self.backup_dir_count < self.backup_dirs.len() {
            self.backup_dirs[self.backup_dir_count] = dir;
            self.backup_dir_count += 1;
        }
    }

    /// List all .oba archives found in the backup directories.
    /// `out` is a caller-provided slice to fill; returns the count.
    pub fn list_backups<'a>(
        &self,
        out: &'a mut [BackupMetadata],
    ) -> usize {
        // In a real implementation: iterate directories, open each .oba,
        // parse the manifest, fill out[].  Stub: return 0.
        let _ = out;
        0
    }

    /// Restore from the most recent backup found in backup_dirs.
    pub fn restore_latest(&self) -> Result<(), &'static str> {
        // 1. List backups sorted by created_at descending.
        // 2. Pick the most recent.
        // 3. Call restore_from_path().
        self.restore_from_path("/sdcard/OneOS/backups/latest.oba", BackupContent::default())
    }

    /// Restore from a specific archive path, including only `content`.
    pub fn restore_from_path(
        &self,
        path:    &str,
        content: BackupContent,
    ) -> Result<(), &'static str> {
        // Step 1: Open and validate archive.
        let _meta = self.parse_archive_header(path)?;

        // Step 2: Pre-flight checks (enough free space, etc.).
        self.preflight_check()?;

        // Step 3: Extract entries matching `content`.
        self.extract_entries(path, content)?;

        // Step 4: Set correct permissions and SELinux labels.
        self.relabel_restored_files()?;

        Ok(())
    }

    /// Create a new backup archive.
    pub fn create_backup(
        &self,
        output_path: &str,
        content:     BackupContent,
    ) -> Result<BackupMetadata, &'static str> {
        // Step 1: Build entry list from the requested content flags.
        // Step 2: Write OBA1 magic + header.
        // Step 3: For each entry: compress + SHA-256 + write header + payload.
        // Step 4: Write trailer CRC-32.
        let _ = (output_path, content);
        Err("backup creation not yet wired to filesystem layer")
    }

    fn parse_archive_header(&self, path: &str) -> Result<BackupMetadata, &'static str> {
        // Read first 8 bytes, check magic and header length.
        // Read manifest JSON (up to OBA_MAX_MANIFEST_LEN).
        // Parse fields into BackupMetadata.
        let _ = path;
        // Stub: return a zeroed metadata.
        Ok(BackupMetadata {
            created_at:       0,
            os_version:       [0u8; 32],
            os_version_len:   0,
            device_model:     [0u8; 64],
            device_model_len: 0,
            content:          BackupContent::default(),
            total_size:       0,
            entry_count:      0,
            archive_crc32:    0,
        })
    }

    fn preflight_check(&self) -> Result<(), &'static str> {
        // Check available space on /data is > total_size * 1.1.
        // Check /data is mounted and writable.
        Ok(())
    }

    fn extract_entries(&self, path: &str, content: BackupContent) -> Result<(), &'static str> {
        // For each entry in the archive:
        //   if entry_content_flag & content != 0:
        //     decompress payload → /data/<path>
        //     verify SHA-256 of decompressed data
        //     set mtime + mode
        let _ = (path, content);
        Ok(())
    }

    fn relabel_restored_files(&self) -> Result<(), &'static str> {
        // Run restorecon -r /data to apply SELinux labels.
        // On OneOS this calls the security framework relabelling service.
        Ok(())
    }

    /// Verify the integrity of a backup archive without extracting it.
    pub fn verify_archive(&self, path: &str) -> Result<bool, &'static str> {
        // 1. Parse header.
        let _meta = self.parse_archive_header(path)?;
        // 2. CRC-32 the entire archive excluding the final 4-byte CRC field.
        // 3. Compare against meta.archive_crc32.
        // Stub: return true.
        Ok(true)
    }
}

impl Default for BackupRestoreManager {
    fn default() -> Self { Self::new() }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_content_includes_contacts() {
        let c = BackupContent::default();
        assert!(c.contains(BackupContent::CONTACTS));
    }

    #[test]
    fn list_backups_returns_zero_with_stub() {
        let mgr = BackupRestoreManager::new();
        let mut out = [];
        assert_eq!(mgr.list_backups(&mut out), 0);
    }

    #[test]
    fn verify_archive_stub_returns_true() {
        let mgr = BackupRestoreManager::new();
        assert_eq!(mgr.verify_archive("/dev/null").unwrap(), true);
    }

    #[test]
    fn restore_latest_runs_without_panic() {
        let mgr = BackupRestoreManager::new();
        // Stub impls make all internal steps succeed.
        let _ = mgr.restore_latest();
    }
}
