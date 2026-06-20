//! kernel_selector.rs – A/B slot selection and kernel version management
//!
//! OneOS uses an A/B (seamless) partition scheme similar to Android A/B OTA.
//! Two complete sets of system partitions exist:
//!   - Slot A: boot_a, system_a, vendor_a, ...
//!   - Slot B: boot_b, system_b, vendor_b, ...
//!
//! The boot manager reads slot metadata from the MISC partition and selects
//! the best bootable slot on each power-on.

use crate::boot_state::{BootStateBlock, BOOT_STATE_MAGIC};
use crate::partition_manager::{PartitionLabel, SlotSuffix};

// ─────────────────────────────────────────────────────────────────────────────
//  Slot metadata (mirrors Android's BootloaderControl / ab_metadata)
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum number of partitions that have per-slot variants.
pub const MAX_AB_PARTITIONS: usize = 16;

/// Number of boot attempts allowed before a slot is marked unbootable.
pub const DEFAULT_TRIES_REMAINING: u8 = 7;

/// Slot health indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SlotStatus {
    /// Successfully booted and marked good.
    Bootable  = 0,
    /// Has not yet successfully completed a full boot.
    Unverified = 1,
    /// Exceeded max boot attempts without success.
    Unbootable = 2,
    /// Slot does not exist (single-slot device).
    Absent    = 3,
}

impl SlotStatus {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => SlotStatus::Bootable,
            1 => SlotStatus::Unverified,
            2 => SlotStatus::Unbootable,
            _ => SlotStatus::Absent,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SlotStatus::Bootable   => "bootable",
            SlotStatus::Unverified => "unverified",
            SlotStatus::Unbootable => "unbootable",
            SlotStatus::Absent     => "absent",
        }
    }
}

/// Per-slot metadata stored inside the MISC partition.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SlotMetadata {
    /// Priority of this slot (higher = preferred).  Range 0–15.
    pub priority:         u8,
    /// Number of remaining boot attempts before marking unbootable.
    pub tries_remaining:  u8,
    /// Whether this slot has been successfully booted at least once.
    pub successful_boot:  u8,
    /// Verity corruption detected on last boot.
    pub verity_corrupted: u8,
}

/// The A/B control block stored in MISC at offset 2048 (after BootStateBlock).
#[derive(Clone)]
#[repr(C)]
pub struct AbControl {
    /// 0x42414F4E ("NOAB" little-endian = "NABO" in ASCII intent).
    pub magic:     u32,
    pub version:   u8,
    pub _reserved: [u8; 11],
    /// Slot A metadata.
    pub slot_a:    SlotMetadata,
    /// Slot B metadata.
    pub slot_b:    SlotMetadata,
    /// CRC32 of the fields above.
    pub crc32:     u32,
}

pub const AB_CONTROL_MAGIC: u32 = 0x42414F4E;

impl AbControl {
    pub fn new_default() -> Self {
        AbControl {
            magic:     AB_CONTROL_MAGIC,
            version:   1,
            _reserved: [0u8; 11],
            slot_a: SlotMetadata {
                priority:         15,
                tries_remaining:  DEFAULT_TRIES_REMAINING,
                successful_boot:  0,
                verity_corrupted: 0,
            },
            slot_b: SlotMetadata {
                priority:         14,
                tries_remaining:  DEFAULT_TRIES_REMAINING,
                successful_boot:  0,
                verity_corrupted: 0,
            },
            crc32: 0,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == AB_CONTROL_MAGIC
        // TODO: add CRC32 check when storage layer is implemented
    }

    pub fn slot_metadata(&self, suffix: SlotSuffix) -> &SlotMetadata {
        match suffix {
            SlotSuffix::A => &self.slot_a,
            SlotSuffix::B => &self.slot_b,
        }
    }

    pub fn slot_metadata_mut(&mut self, suffix: SlotSuffix) -> &mut SlotMetadata {
        match suffix {
            SlotSuffix::A => &mut self.slot_a,
            SlotSuffix::B => &mut self.slot_b,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  KernelVersion
// ─────────────────────────────────────────────────────────────────────────────

/// Semantic version of a kernel image extracted from its embedded version
/// string at /proc/version / `uname -r` format: `major.minor.patch-extra`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct KernelVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u32,
}

impl KernelVersion {
    pub const fn new(major: u16, minor: u16, patch: u32) -> Self {
        KernelVersion { major, minor, patch }
    }

    /// Parse "6.6.42" style string.  Returns None on failure.
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.splitn(3, '.');
        let major = parts.next()?.parse::<u16>().ok()?;
        let minor = parts.next()?.parse::<u16>().ok()?;
        // Strip everything after '-' in the patch field.
        let patch_str = parts.next().unwrap_or("0");
        let patch_clean = patch_str.split('-').next().unwrap_or("0");
        let patch = patch_clean.parse::<u32>().ok()?;
        Some(KernelVersion { major, minor, patch })
    }

    /// Rollback index: packed u64 for comparison.
    pub fn as_rollback_index(&self) -> u64 {
        ((self.major as u64) << 48)
            | ((self.minor as u64) << 32)
            | (self.patch as u64)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  KernelSelector
// ─────────────────────────────────────────────────────────────────────────────

/// Encapsulates the A/B slot selection algorithm.
pub struct KernelSelector {
    pub control: AbControl,
    /// Minimum acceptable kernel rollback index (from verified boot).
    pub min_rollback_index: u64,
}

impl KernelSelector {
    pub fn new(control: AbControl, min_rollback_index: u64) -> Self {
        KernelSelector { control, min_rollback_index }
    }

    /// Select the best slot to boot.
    ///
    /// Algorithm:
    ///   1. Prefer the slot marked `successful_boot = 1` with the highest
    ///      priority that still has `tries_remaining > 0`.
    ///   2. If neither slot has succeeded, choose the one with higher priority
    ///      and decrement its `tries_remaining`.
    ///   3. If a slot is `Unbootable` (tries_remaining == 0, not successful),
    ///      skip it.
    ///   4. If both slots are unbootable, return `None` (triggers recovery).
    pub fn select_slot(&mut self) -> Option<SlotSuffix> {
        let a_ok = self.slot_usable(SlotSuffix::A);
        let b_ok = self.slot_usable(SlotSuffix::B);

        let chosen = match (a_ok, b_ok) {
            (false, false) => return None, // Both dead
            (true,  false) => SlotSuffix::A,
            (false, true)  => SlotSuffix::B,
            (true,  true)  => {
                // Both usable: prefer highest priority, tie-break on A.
                let pa = self.control.slot_a.priority;
                let pb = self.control.slot_b.priority;
                if pb > pa { SlotSuffix::B } else { SlotSuffix::A }
            }
        };

        // Decrement tries_remaining if not yet successful.
        {
            let meta = self.control.slot_metadata_mut(chosen);
            if meta.successful_boot == 0 && meta.tries_remaining > 0 {
                meta.tries_remaining -= 1;
            }
        }

        Some(chosen)
    }

    fn slot_usable(&self, suffix: SlotSuffix) -> bool {
        let meta = self.control.slot_metadata(suffix);
        // Successful boot is always usable (regardless of tries_remaining).
        if meta.successful_boot != 0 {
            return true;
        }
        // Unverified slots are usable while tries remain.
        meta.tries_remaining > 0
    }

    /// Mark `slot` as successfully booted.  Call this from the init process
    /// once the system is sufficiently up.
    pub fn mark_successful(&mut self, slot: SlotSuffix) {
        let meta = self.control.slot_metadata_mut(slot);
        meta.successful_boot  = 1;
        meta.tries_remaining  = DEFAULT_TRIES_REMAINING;
    }

    /// Mark `slot` as unbootable (e.g. after a failed OTA install).
    pub fn mark_unbootable(&mut self, slot: SlotSuffix) {
        let meta = self.control.slot_metadata_mut(slot);
        meta.tries_remaining = 0;
        meta.successful_boot = 0;
    }

    /// Return the slot that is *not* `active` (the "inactive" / update target).
    pub fn inactive_slot(active: SlotSuffix) -> SlotSuffix {
        match active {
            SlotSuffix::A => SlotSuffix::B,
            SlotSuffix::B => SlotSuffix::A,
        }
    }

    /// Check whether a proposed kernel version satisfies rollback protection.
    pub fn passes_rollback_check(&self, version: KernelVersion) -> bool {
        version.as_rollback_index() >= self.min_rollback_index
    }

    /// Return human-readable summary of both slots for boot log output.
    pub fn status_summary(&self) -> ([u8; 128], usize) {
        // Build into a fixed-size buffer (no heap).
        let mut buf = [0u8; 128];
        let msg = b"A=ok B=ok"; // placeholder; real impl formats via write!
        let len = msg.len().min(buf.len());
        buf[..len].copy_from_slice(&msg[..len]);
        (buf, len)
    }
}
