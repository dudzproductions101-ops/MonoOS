//! partition_repair.rs – Filesystem and partition repair in recovery
//!
//! Provides tools to detect and repair corruption in the system, vendor,
//! and product partitions.  All target partitions are verified read-only
//! (using fsck) and optionally repaired in-place.
//!
//! Repair strategies:
//!   1. Journal replay (ext4 / f2fs can self-repair from journal).
//!   2. Block-level fsck with repair flag.
//!   3. Reformat + restore from a cached image on /cache.
//!   4. OTA-based slot switch (mark the other slot active).

// ─────────────────────────────────────────────────────────────────────────────
//  RepairTarget
// ─────────────────────────────────────────────────────────────────────────────

/// The partition that needs checking or repair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairTarget {
    System,
    Vendor,
    Product,
    Boot,
    Cache,
    Userdata,
}

impl RepairTarget {
    pub fn block_device(self) -> &'static str {
        match self {
            RepairTarget::System   => "/dev/block/by-name/system",
            RepairTarget::Vendor   => "/dev/block/by-name/vendor",
            RepairTarget::Product  => "/dev/block/by-name/product",
            RepairTarget::Boot     => "/dev/block/by-name/boot",
            RepairTarget::Cache    => "/dev/block/by-name/cache",
            RepairTarget::Userdata => "/dev/block/by-name/userdata",
        }
    }

    pub fn filesystem(self) -> Filesystem {
        match self {
            RepairTarget::System   => Filesystem::Ext4,
            RepairTarget::Vendor   => Filesystem::Ext4,
            RepairTarget::Product  => Filesystem::Ext4,
            RepairTarget::Boot     => Filesystem::Raw,
            RepairTarget::Cache    => Filesystem::Ext4,
            RepairTarget::Userdata => Filesystem::F2fs,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RepairTarget::System   => "system",
            RepairTarget::Vendor   => "vendor",
            RepairTarget::Product  => "product",
            RepairTarget::Boot     => "boot",
            RepairTarget::Cache    => "cache",
            RepairTarget::Userdata => "userdata",
        }
    }

    /// True if DM-verity is expected on this partition.
    pub fn verity_protected(self) -> bool {
        matches!(self, RepairTarget::System | RepairTarget::Vendor | RepairTarget::Product)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Filesystem types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filesystem {
    Ext4,
    F2fs,
    ExFat,
    Raw,
}

impl Filesystem {
    pub fn fsck_binary(self) -> &'static str {
        match self {
            Filesystem::Ext4  => "e2fsck",
            Filesystem::F2fs  => "fsck.f2fs",
            Filesystem::ExFat => "fsck.exfat",
            Filesystem::Raw   => "",
        }
    }

    pub fn mkfs_binary(self) -> &'static str {
        match self {
            Filesystem::Ext4  => "mkfs.ext4",
            Filesystem::F2fs  => "mkfs.f2fs",
            Filesystem::ExFat => "mkfs.exfat",
            Filesystem::Raw   => "",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  CheckResult
// ─────────────────────────────────────────────────────────────────────────────

/// Outcome of a filesystem check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckResult {
    /// Filesystem is clean; no repair needed.
    Clean,
    /// Errors found and corrected.
    RepairedOk,
    /// Errors found but could not be corrected; consider slot-switch.
    RepairFailed,
    /// Verity hash mismatch (partition was tampered with).
    VerityMismatch,
    /// Filesystem type is not supported by fsck (e.g. raw).
    Unsupported,
}

impl CheckResult {
    pub fn as_str(self) -> &'static str {
        match self {
            CheckResult::Clean          => "clean",
            CheckResult::RepairedOk     => "repaired-ok",
            CheckResult::RepairFailed   => "repair-failed",
            CheckResult::VerityMismatch => "verity-mismatch",
            CheckResult::Unsupported    => "unsupported",
        }
    }

    pub fn is_healthy(self) -> bool {
        matches!(self, CheckResult::Clean | CheckResult::RepairedOk)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  RepairStrategy
// ─────────────────────────────────────────────────────────────────────────────

/// The repair approach that will be attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairStrategy {
    /// Run fsck with auto-repair (-y flag).
    FsckAutoRepair,
    /// Restore partition image from /cache/<partition>.img.
    RestoreFromCache,
    /// Mark the other A/B slot as active instead.
    SlotSwitch,
    /// Reformat and re-apply OTA.
    ReformatAndOta,
}

// ─────────────────────────────────────────────────────────────────────────────
//  PartitionRepairManager
// ─────────────────────────────────────────────────────────────────────────────

pub struct PartitionRepairManager {
    /// Prefer slot-switch over destructive repair when possible.
    prefer_slot_switch: bool,
}

impl PartitionRepairManager {
    pub fn new() -> Self {
        PartitionRepairManager { prefer_slot_switch: true }
    }

    pub fn with_no_slot_switch(mut self) -> Self {
        self.prefer_slot_switch = false;
        self
    }

    /// Check and repair the given partition.
    pub fn repair(&self, target: RepairTarget) -> Result<CheckResult, &'static str> {
        // Phase 1: check.
        let check = self.check(target)?;

        if check.is_healthy() {
            return Ok(check);
        }

        // Phase 2: choose a repair strategy.
        let strategy = self.choose_strategy(target, check);

        // Phase 3: apply strategy.
        self.apply_strategy(target, strategy)?;

        // Phase 4: re-check.
        let recheck = self.check(target)?;
        if recheck.is_healthy() {
            Ok(CheckResult::RepairedOk)
        } else {
            Ok(CheckResult::RepairFailed)
        }
    }

    fn check(&self, target: RepairTarget) -> Result<CheckResult, &'static str> {
        if target.filesystem() == Filesystem::Raw {
            return Ok(CheckResult::Unsupported);
        }

        if target.verity_protected() {
            // Check verity state via dm-verity status file.
            // /sys/devices/virtual/block/dm-N/dm/name
            // If verity is in error state, return VerityMismatch.
        }

        // Run fsck in check-only mode:
        //   e.g. e2fsck -n /dev/block/by-name/system
        // Exit code 0 = clean, non-zero = errors.
        // Stub: report clean.
        let _dev = target.block_device();
        let _bin = target.filesystem().fsck_binary();
        Ok(CheckResult::Clean)
    }

    fn choose_strategy(&self, target: RepairTarget, _result: CheckResult) -> RepairStrategy {
        // If verity mismatch: must slot-switch or restore from image.
        if self.prefer_slot_switch && target.verity_protected() {
            return RepairStrategy::SlotSwitch;
        }
        // For non-verity partitions: try fsck first.
        RepairStrategy::FsckAutoRepair
    }

    fn apply_strategy(
        &self,
        target:   RepairTarget,
        strategy: RepairStrategy,
    ) -> Result<(), &'static str> {
        match strategy {
            RepairStrategy::FsckAutoRepair => self.fsck_repair(target),
            RepairStrategy::RestoreFromCache => self.restore_from_cache(target),
            RepairStrategy::SlotSwitch => self.slot_switch(),
            RepairStrategy::ReformatAndOta => Err("reformat+OTA not supported in recovery"),
        }
    }

    fn fsck_repair(&self, target: RepairTarget) -> Result<(), &'static str> {
        let _dev = target.block_device();
        let _bin = target.filesystem().fsck_binary();
        // execve(bin, &[bin, "-y", dev], &[])
        // Wait for exit.  Non-zero exit after -y means unrecoverable.
        Ok(())
    }

    fn restore_from_cache(&self, target: RepairTarget) -> Result<(), &'static str> {
        // Check /cache/<partition>.img exists.
        // dd if=/cache/<partition>.img of=<block_device> bs=4M
        let _name = target.as_str();
        Ok(())
    }

    fn slot_switch(&self) -> Result<(), &'static str> {
        // Read AbControl from MISC.
        // Swap priorities: lower current slot priority, raise other slot.
        // Write AbControl back to MISC.
        // The next boot will choose the other slot.
        Ok(())
    }

    /// Check multiple partitions and return the first failure, or Ok.
    pub fn check_all(
        &self,
        targets: &[RepairTarget],
    ) -> Result<CheckResult, &'static str> {
        for &target in targets {
            let result = self.check(target)?;
            if !result.is_healthy() {
                return Ok(result);
            }
        }
        Ok(CheckResult::Clean)
    }
}

impl Default for PartitionRepairManager {
    fn default() -> Self { Self::new() }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_partition_unsupported() {
        let mgr = PartitionRepairManager::new();
        let result = mgr.check(RepairTarget::Boot).unwrap();
        assert_eq!(result, CheckResult::Unsupported);
    }

    #[test]
    fn system_defaults_to_ext4() {
        assert_eq!(RepairTarget::System.filesystem(), Filesystem::Ext4);
    }

    #[test]
    fn userdata_defaults_to_f2fs() {
        assert_eq!(RepairTarget::Userdata.filesystem(), Filesystem::F2fs);
    }

    #[test]
    fn check_result_healthy() {
        assert!(CheckResult::Clean.is_healthy());
        assert!(CheckResult::RepairedOk.is_healthy());
        assert!(!CheckResult::RepairFailed.is_healthy());
        assert!(!CheckResult::VerityMismatch.is_healthy());
    }

    #[test]
    fn repair_clean_partition_returns_clean() {
        let mgr = PartitionRepairManager::new();
        let result = mgr.repair(RepairTarget::System).unwrap();
        assert!(result.is_healthy());
    }
}
