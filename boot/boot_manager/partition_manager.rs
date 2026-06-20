//! partition_manager.rs – GPT partition table parsing and slot management
//!
//! Parses a GUID Partition Table (GPT) from the boot device and exposes
//! the partition entries needed by the kernel selector and secure boot layers.
//!
//! Supports both single-slot and A/B (slot _a / _b) partition naming.

use core::fmt;

// ─────────────────────────────────────────────────────────────────────────────
//  Slot suffix
// ─────────────────────────────────────────────────────────────────────────────

/// The suffix appended to partition names for A/B slot addressing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotSuffix {
    A,
    B,
}

impl SlotSuffix {
    pub fn as_str(self) -> &'static str {
        match self {
            SlotSuffix::A => "_a",
            SlotSuffix::B => "_b",
        }
    }
}

impl fmt::Display for SlotSuffix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Well-known partition labels
// ─────────────────────────────────────────────────────────────────────────────

/// Strongly-typed partition label for the partitions the boot manager
/// needs to find.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionLabel {
    /// BCB / boot-control block (always single-slot).
    Misc,
    /// Primary kernel + initramfs (A or B).
    Boot(Option<SlotSuffix>),
    /// dtb / device-tree blobs (A or B).
    Dtb(Option<SlotSuffix>),
    /// Verified Boot 2.0 vbmeta (A or B).
    VbMeta(Option<SlotSuffix>),
    /// Root filesystem (A or B).
    System(Option<SlotSuffix>),
    /// Vendor BSP partition (A or B).
    Vendor(Option<SlotSuffix>),
    /// Recovery kernel+ramdisk (single-slot on most devices).
    Recovery,
    /// Persistent data / userdata.
    Userdata,
    /// Fastboot / firmware update staging area.
    Cache,
    /// Persistent storage across factory resets (OEM).
    Persist,
    /// EFI System Partition (x86 targets).
    Esp,
}

impl PartitionLabel {
    /// Build the partition name string into a fixed-size buffer.
    /// Returns the number of bytes written (excluding NUL).
    pub fn write_name(self, buf: &mut [u8]) -> usize {
        let base: &str = match self {
            PartitionLabel::Misc          => "misc",
            PartitionLabel::Boot(_)       => "boot",
            PartitionLabel::Dtb(_)        => "dtb",
            PartitionLabel::VbMeta(_)     => "vbmeta",
            PartitionLabel::System(_)     => "system",
            PartitionLabel::Vendor(_)     => "vendor",
            PartitionLabel::Recovery      => "recovery",
            PartitionLabel::Userdata      => "userdata",
            PartitionLabel::Cache         => "cache",
            PartitionLabel::Persist       => "persist",
            PartitionLabel::Esp           => "esp",
        };

        let suffix: &str = match self {
            PartitionLabel::Boot(Some(s))
            | PartitionLabel::Dtb(Some(s))
            | PartitionLabel::VbMeta(Some(s))
            | PartitionLabel::System(Some(s))
            | PartitionLabel::Vendor(Some(s)) => s.as_str(),
            _ => "",
        };

        let total = base.len() + suffix.len();
        let copy  = total.min(buf.len().saturating_sub(1));
        let base_copy = base.len().min(copy);
        buf[..base_copy].copy_from_slice(base.as_bytes().split_at(base_copy).0);
        let suffix_copy = (copy - base_copy).min(suffix.len());
        buf[base_copy..base_copy + suffix_copy]
            .copy_from_slice(&suffix.as_bytes()[..suffix_copy]);
        if copy < buf.len() {
            buf[copy] = 0;
        }
        copy
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  GPT structures
// ─────────────────────────────────────────────────────────────────────────────

/// GUID (16 bytes, mixed endianness per UEFI spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl Guid {
    pub const ZERO: Guid = Guid { data1: 0, data2: 0, data3: 0, data4: [0u8; 8] };

    /// Parse from a 16-byte little-endian slice.
    pub fn from_bytes(b: &[u8; 16]) -> Self {
        Guid {
            data1: u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            data2: u16::from_le_bytes([b[4], b[5]]),
            data3: u16::from_le_bytes([b[6], b[7]]),
            data4: [b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]],
        }
    }
}

/// GPT header (at LBA 1 on the disk).
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct GptHeader {
    /// "EFI PART"
    pub signature:             [u8; 8],
    pub revision:              u32,
    pub header_size:           u32,
    pub header_crc32:          u32,
    pub reserved:              u32,
    pub my_lba:                u64,
    pub alternate_lba:         u64,
    pub first_usable_lba:      u64,
    pub last_usable_lba:       u64,
    pub disk_guid:             [u8; 16],
    pub partition_entry_lba:   u64,
    pub num_partition_entries: u32,
    pub sizeof_partition_entry: u32,
    pub partition_entry_array_crc32: u32,
}

impl GptHeader {
    pub const SIGNATURE: &'static [u8; 8] = b"EFI PART";
    pub const REVISION_1_0: u32 = 0x00010000;

    pub fn is_valid_signature(&self) -> bool {
        &self.signature == Self::SIGNATURE
    }
}

/// GPT partition entry (128 bytes each by default).
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct GptPartitionEntry {
    pub type_guid:       [u8; 16],
    pub unique_guid:     [u8; 16],
    pub starting_lba:    u64,
    pub ending_lba:      u64,
    pub attributes:      u64,
    /// UTF-16LE partition name (72 bytes = 36 UTF-16 code units).
    pub name:            [u8; 72],
}

impl GptPartitionEntry {
    pub const EMPTY: GptPartitionEntry = GptPartitionEntry {
        type_guid:    [0u8; 16],
        unique_guid:  [0u8; 16],
        starting_lba: 0,
        ending_lba:   0,
        attributes:   0,
        name:         [0u8; 72],
    };

    pub fn is_used(&self) -> bool {
        self.type_guid.iter().any(|&b| b != 0)
    }

    /// Compare partition name (UTF-16LE in entry) against an ASCII label.
    pub fn name_matches_ascii(&self, ascii: &str) -> bool {
        let name = &self.name;
        let ascii_bytes = ascii.as_bytes();
        if ascii_bytes.len() * 2 > name.len() {
            return false;
        }
        for (i, &ab) in ascii_bytes.iter().enumerate() {
            if name[i * 2] != ab || name[i * 2 + 1] != 0 {
                return false;
            }
        }
        // Check terminator
        let end = ascii_bytes.len() * 2;
        end < name.len() && name[end] == 0
    }

    pub fn lba_count(&self) -> u64 {
        if self.ending_lba >= self.starting_lba {
            self.ending_lba - self.starting_lba + 1
        } else {
            0
        }
    }

    pub fn byte_size(&self, lba_size: u64) -> u64 {
        self.lba_count() * lba_size
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  PartitionManager
// ─────────────────────────────────────────────────────────────────────────────

pub const MAX_PARTITIONS: usize = 128;
pub const LBA_SIZE: u64 = 512;

/// Holds a parsed view of the GPT, ready for label-based lookups.
pub struct PartitionManager {
    pub disk_guid:   [u8; 16],
    pub lba_size:    u64,
    entries:         [GptPartitionEntry; MAX_PARTITIONS],
    count:           usize,
}

impl PartitionManager {
    /// Create an empty manager (no partitions).
    pub fn new_empty() -> Self {
        PartitionManager {
            disk_guid: [0u8; 16],
            lba_size:  LBA_SIZE,
            entries:   [GptPartitionEntry::EMPTY; MAX_PARTITIONS],
            count:     0,
        }
    }

    /// Parse GPT entries from a raw buffer.
    ///
    /// `gpt_header_buf` must contain at least one LBA starting at LBA 1.
    /// `entries_buf` must contain the partition entry array.
    pub fn parse(
        &mut self,
        header_buf:  &[u8],
        entries_buf: &[u8],
    ) -> Result<(), &'static str> {
        if header_buf.len() < core::mem::size_of::<GptHeader>() {
            return Err("GPT header buffer too small");
        }

        // Safety: buffer is large enough, alignment comes from the outer
        // caller which should provide a 512-byte aligned read buffer.
        let hdr = unsafe {
            &*(header_buf.as_ptr() as *const GptHeader)
        };

        if !hdr.is_valid_signature() {
            return Err("GPT signature mismatch");
        }

        self.disk_guid = hdr.disk_guid;

        let entry_size = hdr.sizeof_partition_entry as usize;
        if entry_size < core::mem::size_of::<GptPartitionEntry>() {
            return Err("GPT entry size too small");
        }

        let n = (hdr.num_partition_entries as usize).min(MAX_PARTITIONS);
        self.count = 0;

        for i in 0..n {
            let off = i * entry_size;
            if off + core::mem::size_of::<GptPartitionEntry>() > entries_buf.len() {
                break;
            }
            let entry = unsafe {
                *(entries_buf.as_ptr().add(off) as *const GptPartitionEntry)
            };
            if entry.is_used() {
                self.entries[self.count] = entry;
                self.count += 1;
            }
        }

        Ok(())
    }

    /// Find a partition by ASCII name.  Returns None if not present.
    pub fn find_by_name(&self, name: &str) -> Option<&GptPartitionEntry> {
        self.entries[..self.count]
            .iter()
            .find(|e| e.name_matches_ascii(name))
    }

    /// Find a partition by PartitionLabel.
    pub fn find_by_label(&self, label: PartitionLabel) -> Option<&GptPartitionEntry> {
        let mut name_buf = [0u8; 64];
        let len = label.write_name(&mut name_buf);
        let name_str = core::str::from_utf8(&name_buf[..len]).unwrap_or("");
        self.find_by_name(name_str)
    }

    /// Return the starting byte offset of a partition.
    pub fn partition_offset(&self, entry: &GptPartitionEntry) -> u64 {
        entry.starting_lba * self.lba_size
    }

    /// Return the byte size of a partition.
    pub fn partition_size(&self, entry: &GptPartitionEntry) -> u64 {
        entry.byte_size(self.lba_size)
    }

    pub fn count(&self) -> usize {
        self.count
    }

    /// Iterate over all valid partitions.
    pub fn entries(&self) -> &[GptPartitionEntry] {
        &self.entries[..self.count]
    }
}
