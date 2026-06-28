//! recovery_main.rs – MonoOS recovery environment entry point
//!
//! The recovery environment is a minimal Linux userspace that runs when:
//!   - The boot command in the MISC partition is "boot-recovery".
//!   - The volume-down key is held at power-on.
//!   - The normal boot slot has failed too many times.
//!
//! Recovery responsibilities:
//!   1. Mount the userdata and cache partitions (if available).
//!   2. Apply an OTA update package from cache or sideload.
//!   3. Perform a factory reset (wipe userdata + cache).
//!   4. Repair a corrupted system partition.
//!   5. Present a minimal on-screen menu driven by volume keys + power.
//!
//! This module is compiled as part of the `monoos-recovery` binary that
//! runs as PID 1 in the recovery ramdisk.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod backup_restore;
pub mod factory_reset;
pub mod partition_repair;
pub mod sideload_manager;

use backup_restore::BackupRestoreManager;
use factory_reset::{FactoryResetManager, WipeScope};
use partition_repair::{PartitionRepairManager, RepairTarget};
use sideload_manager::{SideloadManager, SideloadSource};

// ─────────────────────────────────────────────────────────────────────────────
//  Recovery action requested via MISC / kernel cmdline
// ─────────────────────────────────────────────────────────────────────────────

/// The high-level action that recovery should perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Interactive menu driven by hardware keys.
    InteractiveMenu,
    /// Apply OTA package from /cache/update.zip.
    ApplyOtaFromCache,
    /// Accept a package via USB sideload (adb sideload).
    SideloadFromUsb,
    /// Wipe userdata + cache (factory reset).
    WipeData,
    /// Wipe userdata, cache, and internal storage.
    WipeDataAndMedia,
    /// Repair the system partition using a cached image.
    RepairSystem,
    /// Restore from a previously created backup.
    RestoreBackup,
    /// Reboot immediately (no-op in recovery).
    Reboot,
}

impl RecoveryAction {
    /// Parse the recovery argument string written to the MISC partition.
    pub fn from_arg(arg: &str) -> Self {
        match arg.trim_matches('\0').trim() {
            "--apply_update=CACHE"          => RecoveryAction::ApplyOtaFromCache,
            "--apply_update=ADB"            => RecoveryAction::SideloadFromUsb,
            "--wipe_data"                   => RecoveryAction::WipeData,
            "--wipe_data_and_media"         => RecoveryAction::WipeDataAndMedia,
            "--repair_system"               => RecoveryAction::RepairSystem,
            "--restore_backup"              => RecoveryAction::RestoreBackup,
            _                               => RecoveryAction::InteractiveMenu,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RecoveryAction::InteractiveMenu      => "interactive-menu",
            RecoveryAction::ApplyOtaFromCache    => "apply-ota-cache",
            RecoveryAction::SideloadFromUsb      => "sideload-usb",
            RecoveryAction::WipeData             => "wipe-data",
            RecoveryAction::WipeDataAndMedia     => "wipe-data-and-media",
            RecoveryAction::RepairSystem         => "repair-system",
            RecoveryAction::RestoreBackup        => "restore-backup",
            RecoveryAction::Reboot               => "reboot",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  RecoveryResult
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryResult {
    /// Operation completed successfully; reboot to normal.
    Success,
    /// Operation failed; stay in recovery.
    Failed(&'static str),
    /// User selected reboot from menu.
    UserReboot,
    /// User selected power off.
    UserPowerOff,
}

// ─────────────────────────────────────────────────────────────────────────────
//  RecoveryContext
// ─────────────────────────────────────────────────────────────────────────────

/// Runtime context shared across all recovery operations.
pub struct RecoveryContext {
    /// Requested action (from MISC recovery_arg).
    pub action:          RecoveryAction,
    /// True if adb is enabled (USB connected + OEM-unlock or recovery keys).
    pub adb_enabled:     bool,
    /// Device has a display (false on headless targets).
    pub has_display:     bool,
    /// Path to the OTA package (or empty if sideloading).
    pub ota_package_path: &'static str,
    /// Slot suffix that was active before entering recovery.
    pub active_slot:     u8,  // 0 = A, 1 = B
}

impl RecoveryContext {
    pub fn new(action: RecoveryAction) -> Self {
        RecoveryContext {
            action,
            adb_enabled:     false,
            has_display:     true,
            ota_package_path: "/cache/update.zip",
            active_slot:     0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  RecoveryManager – orchestrator
// ─────────────────────────────────────────────────────────────────────────────

pub struct RecoveryManager {
    ctx:     RecoveryContext,
    repair:  PartitionRepairManager,
    sideload: SideloadManager,
    factory: FactoryResetManager,
    backup:  BackupRestoreManager,
}

impl RecoveryManager {
    pub fn new(ctx: RecoveryContext) -> Self {
        RecoveryManager {
            repair:   PartitionRepairManager::new(),
            sideload: SideloadManager::new(),
            factory:  FactoryResetManager::new(),
            backup:   BackupRestoreManager::new(),
            ctx,
        }
    }

    /// Run the requested recovery action to completion.
    pub fn run(&mut self) -> RecoveryResult {
        match self.ctx.action {
            RecoveryAction::InteractiveMenu => self.interactive_menu(),
            RecoveryAction::ApplyOtaFromCache => {
                self.apply_ota(self.ctx.ota_package_path)
            }
            RecoveryAction::SideloadFromUsb => {
                self.sideload_from_usb()
            }
            RecoveryAction::WipeData => {
                match self.factory.wipe(WipeScope::DataAndCache) {
                    Ok(_)  => RecoveryResult::Success,
                    Err(e) => RecoveryResult::Failed(e),
                }
            }
            RecoveryAction::WipeDataAndMedia => {
                match self.factory.wipe(WipeScope::DataCacheAndMedia) {
                    Ok(_)  => RecoveryResult::Success,
                    Err(e) => RecoveryResult::Failed(e),
                }
            }
            RecoveryAction::RepairSystem => {
                match self.repair.repair(RepairTarget::System) {
                    Ok(_)  => RecoveryResult::Success,
                    Err(e) => RecoveryResult::Failed(e),
                }
            }
            RecoveryAction::RestoreBackup => {
                match self.backup.restore_latest() {
                    Ok(_)  => RecoveryResult::Success,
                    Err(e) => RecoveryResult::Failed(e),
                }
            }
            RecoveryAction::Reboot => RecoveryResult::UserReboot,
        }
    }

    /// Minimal interactive menu driven by volume keys.
    /// In a real implementation this renders text on the framebuffer and
    /// reads GPIO key events.  Here we define the structure.
    fn interactive_menu(&mut self) -> RecoveryResult {
        // Menu items and their bound actions.
        const MENU: &[(&str, RecoveryAction)] = &[
            ("Reboot system now",          RecoveryAction::Reboot),
            ("Apply update from ADB",      RecoveryAction::SideloadFromUsb),
            ("Apply update from cache",    RecoveryAction::ApplyOtaFromCache),
            ("Factory reset / wipe data",  RecoveryAction::WipeData),
            ("Wipe cache partition",       RecoveryAction::WipeData),
            ("Repair system partition",    RecoveryAction::RepairSystem),
            ("Restore from backup",        RecoveryAction::RestoreBackup),
            ("Power off",                  RecoveryAction::Reboot),
        ];

        // Placeholder: in a real build this loops reading key events.
        // We auto-select "Reboot" for headless / automated testing.
        let _menu_count = MENU.len();
        RecoveryResult::UserReboot
    }

    fn apply_ota(&mut self, path: &'static str) -> RecoveryResult {
        let src = SideloadSource::CacheFile(path);
        match self.sideload.apply_package(src) {
            Ok(_)  => RecoveryResult::Success,
            Err(e) => RecoveryResult::Failed(e),
        }
    }

    fn sideload_from_usb(&mut self) -> RecoveryResult {
        let src = SideloadSource::AdbSideload;
        match self.sideload.apply_package(src) {
            Ok(_)  => RecoveryResult::Success,
            Err(e) => RecoveryResult::Failed(e),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  C FFI entry point (called from the recovery init script / C stub)
// ─────────────────────────────────────────────────────────────────────────────

/// Recovery entry point called from the C init stub in the recovery ramdisk.
///
/// `recovery_arg`: NUL-terminated string from the MISC recovery_arg field.
/// Returns 0 to reboot, 1 for power-off, negative on unrecoverable failure.
///
/// # Safety
/// `recovery_arg` must be a valid, NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn monoos_recovery_main(recovery_arg: *const u8) -> i32 {
    let arg_str = if recovery_arg.is_null() {
        ""
    } else {
        // Find NUL terminator, cap at 256.
        let mut len = 0usize;
        while len < 256 && *recovery_arg.add(len) != 0 {
            len += 1;
        }
        core::str::from_utf8(core::slice::from_raw_parts(recovery_arg, len))
            .unwrap_or("")
    };

    let action = RecoveryAction::from_arg(arg_str);
    let ctx    = RecoveryContext::new(action);
    let mut mgr = RecoveryManager::new(ctx);

    match mgr.run() {
        RecoveryResult::Success     => 0,
        RecoveryResult::UserReboot  => 0,
        RecoveryResult::UserPowerOff => 1,
        RecoveryResult::Failed(_)   => -1,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Panic handler (required for no_std binary)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
