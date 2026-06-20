//! boot_state.rs – Persistent and transient boot state for OneOS
//!
//! Boot state is stored in a small region of non-volatile memory (typically
//! an NVRAM partition or the last sector of the MISC partition, matching the
//! Android BCB layout used by many ARM SoCs).
//!
//! The on-disk structure is a 2 KiB block: 
//!   - 32 bytes: magic + version + CRC32
//!   - 256 bytes: boot command string (NUL-terminated)
//!   - 256 bytes: status message from last operation
//!   - 256 bytes: recovery argument string
//!   - remainder: reserved / zero

use core::fmt;

/// Magic bytes that identify a valid BootStateBlock on-disk.
pub const BOOT_STATE_MAGIC: u32 = 0x4F4E4553; // "ONES"

/// Current schema version.
pub const BOOT_STATE_VERSION: u16 = 1;

/// On-disk block size for the boot state.
pub const BOOT_STATE_BLOCK_SIZE: usize = 2048;

/// Length of string fields within the block.
pub const BOOT_COMMAND_LEN: usize = 256;
pub const BOOT_STATUS_LEN: usize = 256;
pub const BOOT_RECOVERY_ARG_LEN: usize = 256;

// ─────────────────────────────────────────────────────────────────────────────
//  BootCommand
// ─────────────────────────────────────────────────────────────────────────────

/// The boot command written into the BCB-style block by updaters / recovery
/// tools to influence what happens on the *next* reboot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BootCommand {
    /// Normal boot into the main OS. Default.
    Normal = 0,
    /// Boot into the recovery partition.
    BootRecovery = 1,
    /// Apply an OTA update from the cache or sideload location.
    ApplyUpdate = 2,
    /// Perform a factory reset during the next recovery boot.
    WipeData = 3,
    /// Boot into the bootloader / fastboot mode.
    Fastboot = 4,
    /// Boot with a diagnostics target (minimal, read-only rootfs).
    Diagnostics = 5,
    /// Safe mode: disable third-party kernel modules and services.
    SafeMode = 6,
}

impl BootCommand {
    /// Parse from the on-disk NUL-terminated string representation.
    pub fn from_str(s: &str) -> Self {
        match s.trim_matches('\0').trim() {
            "boot-recovery"  => BootCommand::BootRecovery,
            "apply-update"   => BootCommand::ApplyUpdate,
            "wipe-data"      => BootCommand::WipeData,
            "bootloader"     => BootCommand::Fastboot,
            "diagnostics"    => BootCommand::Diagnostics,
            "safe-mode"      => BootCommand::SafeMode,
            _                => BootCommand::Normal,
        }
    }

    /// Return the canonical string used when writing to the BCB block.
    pub fn as_str(self) -> &'static str {
        match self {
            BootCommand::Normal      => "",
            BootCommand::BootRecovery => "boot-recovery",
            BootCommand::ApplyUpdate  => "apply-update",
            BootCommand::WipeData     => "wipe-data",
            BootCommand::Fastboot     => "bootloader",
            BootCommand::Diagnostics  => "diagnostics",
            BootCommand::SafeMode     => "safe-mode",
        }
    }
}

impl fmt::Display for BootCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  BootStateBlock  (raw on-disk layout)
// ─────────────────────────────────────────────────────────────────────────────

/// The raw, byte-for-byte layout of the boot state block as persisted to
/// the MISC / NVRAM partition.  Must remain `repr(C)` and fixed-size.
#[repr(C)]
#[derive(Clone)]
pub struct BootStateBlock {
    /// 0x4F4E4553 ("ONES") when valid.
    pub magic:        u32,
    /// Schema version (currently 1).
    pub version:      u16,
    /// CRC32 of bytes [8..BOOT_STATE_BLOCK_SIZE].
    pub crc32:        u32,
    /// Padding to 8-byte boundary.
    pub _pad:         [u8; 2],
    /// NUL-terminated boot command string.
    pub command:      [u8; BOOT_COMMAND_LEN],
    /// NUL-terminated status message written by recovery.
    pub status:       [u8; BOOT_STATUS_LEN],
    /// NUL-terminated argument to recovery (e.g. OTA package path).
    pub recovery_arg: [u8; BOOT_RECOVERY_ARG_LEN],
    /// Number of consecutive boot failures since last successful boot.
    pub fail_count:   u8,
    /// Maximum failures allowed before falling back to recovery.
    pub fail_limit:   u8,
    /// Reserved for future use.
    pub _reserved:    [u8; 1030],
}

static_assert_size!(BootStateBlock, BOOT_STATE_BLOCK_SIZE);

impl BootStateBlock {
    /// Create a blank block with the correct magic and version.
    pub fn new_blank() -> Self {
        let mut b = BootStateBlock {
            magic:        BOOT_STATE_MAGIC,
            version:      BOOT_STATE_VERSION,
            crc32:        0,
            _pad:         [0u8; 2],
            command:      [0u8; BOOT_COMMAND_LEN],
            status:       [0u8; BOOT_STATUS_LEN],
            recovery_arg: [0u8; BOOT_RECOVERY_ARG_LEN],
            fail_count:   0,
            fail_limit:   3,
            _reserved:    [0u8; 1030],
        };
        b.update_crc();
        b
    }

    /// Return true if the magic and CRC32 are valid.
    pub fn is_valid(&self) -> bool {
        if self.magic != BOOT_STATE_MAGIC || self.version != BOOT_STATE_VERSION {
            return false;
        }
        let saved = self.crc32;
        let computed = self.compute_crc();
        saved == computed
    }

    /// Compute CRC32 over bytes [8..BOOT_STATE_BLOCK_SIZE].
    pub fn compute_crc(&self) -> u32 {
        // CRC32 (IEEE 802.3 polynomial 0xEDB88320) computed over bytes
        // starting at offset 8 (after magic/version/crc32/_pad fields).
        let raw = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                BOOT_STATE_BLOCK_SIZE,
            )
        };
        crc32_ieee(&raw[8..])
    }

    /// Recompute and store the CRC32 in self.crc32.
    pub fn update_crc(&mut self) {
        self.crc32 = 0;
        self.crc32 = self.compute_crc();
    }

    /// Read the command field as a Rust &str (up to the first NUL).
    pub fn command_str(&self) -> &str {
        nul_str(&self.command)
    }

    /// Read the status field as a Rust &str.
    pub fn status_str(&self) -> &str {
        nul_str(&self.status)
    }

    /// Read the recovery_arg field as a Rust &str.
    pub fn recovery_arg_str(&self) -> &str {
        nul_str(&self.recovery_arg)
    }

    /// Write `cmd` into the command field.
    pub fn set_command(&mut self, cmd: &str) {
        write_nul(&mut self.command, cmd);
        self.update_crc();
    }

    /// Write `msg` into the status field.
    pub fn set_status(&mut self, msg: &str) {
        write_nul(&mut self.status, msg);
        self.update_crc();
    }

    /// Write `arg` into the recovery_arg field.
    pub fn set_recovery_arg(&mut self, arg: &str) {
        write_nul(&mut self.recovery_arg, arg);
        self.update_crc();
    }

    /// Clear the command field (signals "normal boot" to the bootloader).
    pub fn clear_command(&mut self) {
        self.command = [0u8; BOOT_COMMAND_LEN];
        self.update_crc();
    }

    /// Increment the failure counter.  Returns the new count.
    pub fn increment_fail(&mut self) -> u8 {
        if self.fail_count < u8::MAX {
            self.fail_count += 1;
        }
        self.update_crc();
        self.fail_count
    }

    /// Reset the failure counter after a successful boot.
    pub fn clear_fail_count(&mut self) {
        self.fail_count = 0;
        self.update_crc();
    }

    /// Returns true if the failure count has reached the limit.
    pub fn failed_too_many_times(&self) -> bool {
        self.fail_count >= self.fail_limit
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  BootState  (higher-level accessor)
// ─────────────────────────────────────────────────────────────────────────────

/// Higher-level representation of the current boot state, derived from the
/// on-disk [`BootStateBlock`] after parsing.
#[derive(Debug, Clone)]
pub struct BootState {
    pub command:       BootCommand,
    pub status_msg:    heapless::String<BOOT_STATUS_LEN>,
    pub recovery_arg:  heapless::String<BOOT_RECOVERY_ARG_LEN>,
    pub fail_count:    u8,
    pub fail_limit:    u8,
    /// True if the on-disk block was present and valid.
    pub block_valid:   bool,
}

impl BootState {
    /// Parse a [`BootState`] from a validated [`BootStateBlock`].
    pub fn from_block(block: &BootStateBlock) -> Self {
        BootState {
            command:      BootCommand::from_str(block.command_str()),
            status_msg:   heapless::String::from(block.status_str()),
            recovery_arg: heapless::String::from(block.recovery_arg_str()),
            fail_count:   block.fail_count,
            fail_limit:   block.fail_limit,
            block_valid:  true,
        }
    }

    /// Construct a default state used when the MISC partition is absent or
    /// the block is corrupt.
    pub fn default_normal() -> Self {
        BootState {
            command:      BootCommand::Normal,
            status_msg:   heapless::String::new(),
            recovery_arg: heapless::String::new(),
            fail_count:   0,
            fail_limit:   3,
            block_valid:  false,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Utility helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Interpret a fixed-size byte array as a NUL-terminated string.
fn nul_str(buf: &[u8]) -> &str {
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    core::str::from_utf8(&buf[..len]).unwrap_or("")
}

/// Write a string into a fixed-size byte array, NUL-terminating it.
fn write_nul(buf: &mut [u8], s: &str) {
    let bytes = s.as_bytes();
    let copy = bytes.len().min(buf.len().saturating_sub(1));
    buf[..copy].copy_from_slice(&bytes[..copy]);
    buf[copy] = 0;
    // Zero remaining bytes
    for b in &mut buf[copy + 1..] {
        *b = 0;
    }
}

/// Minimal CRC32 (IEEE/Ethernet) without a lookup table (size-optimised for
/// the bootloader environment where flash is constrained).
fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = if crc & 1 != 0 { 0xEDB8_8320u32 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    crc ^ 0xFFFF_FFFF
}

// Compile-time size assertion (requires the `static_assertions` crate, or
// replace with a manual const assertion).
macro_rules! static_assert_size {
    ($t:ty, $n:expr) => {
        const _: [(); $n] = [(); core::mem::size_of::<$t>()];
    };
}
use static_assert_size;
