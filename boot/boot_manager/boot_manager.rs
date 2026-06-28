//! boot_manager.rs – Top-level Rust boot manager orchestrator
//!
//! This module is the Rust-side entry point called by the C bootloader
//! via FFI.  It coordinates:
//!   1. Reading persistent boot state (BCB + A/B control) from MISC.
//!   2. Detecting GPIO key combos for manual mode override.
//!   3. Selecting the boot slot (A/B) and kernel image.
//!   4. Delegating to secure boot verification.
//!   5. Communicating the final boot mode back to the C loader.

#![cfg_attr(not(feature = "std"), no_std)]

// Re-export sub-modules so the C FFI shim can be in this file.
pub mod boot_flags;
pub mod boot_state;
pub mod kernel_selector;
pub mod partition_manager;
pub mod startup_profile;

use boot_flags::{BootFlags, PersistentFlags, TransientFlags};
use boot_state::{BootCommand, BootState, BootStateBlock};
use kernel_selector::{AbControl, KernelSelector};
use partition_manager::{PartitionLabel, PartitionManager, SlotSuffix};
use startup_profile::{select_profile, BootTimings, StartupProfile};

// ─────────────────────────────────────────────────────────────────────────────
//  Error type
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootError {
    /// GPT could not be parsed.
    InvalidGpt,
    /// MISC partition not found in GPT.
    MiscNotFound,
    /// Both A/B slots are unbootable.
    NoBootableSlot,
    /// Secure boot verification rejected the image.
    VerificationFailed,
    /// Battery level too low for a safe OTA.
    BatteryLow,
    /// A required partition is absent.
    PartitionMissing,
    /// Internal logic error (should not happen).
    Internal,
}

impl BootError {
    pub fn as_str(self) -> &'static str {
        match self {
            BootError::InvalidGpt          => "Invalid GPT",
            BootError::MiscNotFound        => "MISC partition not found",
            BootError::NoBootableSlot      => "No bootable A/B slot",
            BootError::VerificationFailed  => "Secure boot verification failed",
            BootError::BatteryLow          => "Battery too low",
            BootError::PartitionMissing    => "Required partition missing",
            BootError::Internal            => "Internal boot manager error",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  BootDecision – the output of the boot manager
// ─────────────────────────────────────────────────────────────────────────────

/// The final decision produced by [`BootManager::decide`].  The C loader
/// reads this struct to know what to load and how to configure the kernel.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BootDecision {
    /// Which boot mode to activate.
    pub mode:                 u8,  // maps to BootCommand discriminant
    /// The selected slot (0 = A, 1 = B, 0xFF = single-slot).
    pub slot:                 u8,
    /// Byte offset on the boot device of the kernel partition.
    pub kernel_partition_offset: u64,
    /// Byte size of the kernel partition.
    pub kernel_partition_size:   u64,
    /// Byte offset of the initramfs partition (0 if embedded in kernel).
    pub initrd_partition_offset: u64,
    /// Byte size of the initramfs partition.
    pub initrd_partition_size:   u64,
    /// Rollback index of the selected slot's kernel.
    pub rollback_index:          u64,
    /// True (1) if secure boot verification has been scheduled.
    pub requires_verification:   u8,
}

impl BootDecision {
    pub fn slot_suffix(&self) -> SlotSuffix {
        if self.slot == 1 { SlotSuffix::B } else { SlotSuffix::A }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  BootManager
// ─────────────────────────────────────────────────────────────────────────────

pub struct BootManager {
    pub flags:    BootFlags,
    pub state:    BootState,
    pub timings:  BootTimings,
    pub selector: KernelSelector,
    pub parts:    PartitionManager,
}

impl BootManager {
    /// Construct from raw binary data read from the MISC partition.
    ///
    /// `misc_buf` must be at least `BOOT_STATE_BLOCK_SIZE + sizeof(AbControl)`
    /// bytes (i.e. 2048 + 32 = 2080 bytes).
    ///
    /// `persistent_flags_bits` is the u32 stored in the device's secure
    /// factory data partition or TEE storage.
    pub fn from_misc(
        misc_buf:               &[u8],
        persistent_flags_bits:  u32,
        min_rollback_index:     u64,
    ) -> Self {
        // Parse BootStateBlock
        let (state, _block_valid) = if misc_buf.len() >= core::mem::size_of::<BootStateBlock>() {
            let block = unsafe {
                &*(misc_buf.as_ptr() as *const BootStateBlock)
            };
            if block.is_valid() {
                (BootState::from_block(block), true)
            } else {
                (BootState::default_normal(), false)
            }
        } else {
            (BootState::default_normal(), false)
        };

        // Parse AbControl (starts at offset BOOT_STATE_BLOCK_SIZE)
        let ab_offset = core::mem::size_of::<BootStateBlock>();
        let ab_ctrl = if misc_buf.len() >= ab_offset + core::mem::size_of::<AbControl>() {
            let ctrl = unsafe {
                *(misc_buf.as_ptr().add(ab_offset) as *const AbControl)
            };
            if ctrl.is_valid() { ctrl } else { AbControl::new_default() }
        } else {
            AbControl::new_default()
        };

        let flags    = BootFlags::from_persistent_bits(persistent_flags_bits);
        let selector = KernelSelector::new(ab_ctrl, min_rollback_index);
        let parts    = PartitionManager::new_empty();

        BootManager { flags, state, timings: BootTimings::new(), selector, parts }
    }

    /// Detect hardware key combos and update transient flags accordingly.
    ///
    /// In a real driver this reads GPIO registers; here the caller supplies
    /// the raw key state.
    pub fn apply_key_state(&mut self, vol_down: bool, vol_up: bool, power_long: bool) {
        self.flags.set_transient(TransientFlags::KEY_VOL_DOWN_HELD, vol_down);
        self.flags.set_transient(TransientFlags::KEY_VOL_UP_HELD,   vol_up);
        self.flags.set_transient(TransientFlags::KEY_POWER_LONG,    power_long);
    }

    /// Override boot command based on hardware key presses.
    /// Keys take priority over the MISC BCB content.
    pub fn resolve_command(&self) -> BootCommand {
        if self.flags.recovery_key_held() {
            return BootCommand::BootRecovery;
        }
        if self.flags.fastboot_key_held() {
            return BootCommand::Fastboot;
        }
        if self.flags.transient.contains(TransientFlags::KEY_POWER_LONG) {
            return BootCommand::SafeMode;
        }
        // Too many boot failures → forced recovery.
        if self.state.fail_count >= self.state.fail_limit {
            return BootCommand::BootRecovery;
        }
        self.state.command
    }

    /// Produce the final [`BootDecision`].
    pub fn decide(&mut self) -> Result<BootDecision, BootError> {
        let command = self.resolve_command();

        // OTA update: verify battery is sufficient.
        if command == BootCommand::ApplyUpdate && self.flags.battery_too_low() {
            return Err(BootError::BatteryLow);
        }

        let slot = match command {
            BootCommand::Normal | BootCommand::ApplyUpdate | BootCommand::SafeMode => {
                match self.selector.select_slot() {
                    Some(s) => s,
                    None    => return Err(BootError::NoBootableSlot),
                }
            }
            BootCommand::BootRecovery | BootCommand::WipeData => {
                // Recovery doesn't use A/B; it has its own dedicated partition.
                SlotSuffix::A // ignored for recovery
            }
            BootCommand::Fastboot => SlotSuffix::A,
            BootCommand::Diagnostics => {
                self.selector.select_slot().unwrap_or(SlotSuffix::A)
            }
        };

        // Look up partition offsets.
        let (kern_off, kern_sz) = self.find_kernel_partition(command, slot)?;
        let (init_off, init_sz) = self.find_initrd_partition(command, slot);

        let slot_u8 = match slot { SlotSuffix::A => 0, SlotSuffix::B => 1 };

        let decision = BootDecision {
            mode:                    command as u8,
            slot:                    slot_u8,
            kernel_partition_offset: kern_off,
            kernel_partition_size:   kern_sz,
            initrd_partition_offset: init_off,
            initrd_partition_size:   init_sz,
            rollback_index:          self.selector.min_rollback_index,
            requires_verification:   if self.flags.secure_boot_enabled() { 1 } else { 0 },
        };

        Ok(decision)
    }

    /// Return the [`StartupProfile`] that applies to the current boot
    /// command. Used by init/userspace to decide which services and kernel
    /// arguments to apply once the kernel hands off to userspace.
    pub fn profile(&self) -> StartupProfile {
        select_profile(self.resolve_command(), self.flags.persistent.contains(PersistentFlags::ADB_ENABLED))
    }

    fn find_kernel_partition(
        &self,
        cmd:  BootCommand,
        slot: SlotSuffix,
    ) -> Result<(u64, u64), BootError> {
        let label = match cmd {
            BootCommand::BootRecovery | BootCommand::WipeData => PartitionLabel::Recovery,
            _ => PartitionLabel::Boot(Some(slot)),
        };
        match self.parts.find_by_label(label) {
            Some(e) => Ok((
                self.parts.partition_offset(e),
                self.parts.partition_size(e),
            )),
            None => {
                // Fallback: try boot without slot suffix (single-slot device).
                match self.parts.find_by_label(PartitionLabel::Boot(None)) {
                    Some(e) => Ok((
                        self.parts.partition_offset(e),
                        self.parts.partition_size(e),
                    )),
                    None => Err(BootError::PartitionMissing),
                }
            }
        }
    }

    fn find_initrd_partition(&self, _cmd: BootCommand, _slot: SlotSuffix) -> (u64, u64) {
        // On most ARM SoCs the initramfs is embedded in the kernel image;
        // return (0, 0) to signal "no separate initrd partition".
        (0, 0)
    }

    /// Mark the currently booted slot as successful.  Called by the init
    /// process once it has reached a stable state.
    pub fn mark_current_boot_successful(&mut self, slot: SlotSuffix) {
        self.selector.mark_successful(slot);
    }

    /// Serialize the in-memory `AbControl` (with a freshly recomputed CRC32)
    /// into the MISC buffer at its fixed offset (`size_of::<BootStateBlock>()`).
    ///
    /// Returns `Err(BootError::Internal)` if `misc_buf` is too small to hold
    /// the BootStateBlock + AbControl region.
    pub fn flush_ab_control(&mut self, misc_buf: &mut [u8]) -> Result<(), BootError> {
        self.selector.control.update_crc();

        let ab_offset = core::mem::size_of::<BootStateBlock>();
        let ab_size = core::mem::size_of::<AbControl>();
        if misc_buf.len() < ab_offset + ab_size {
            return Err(BootError::Internal);
        }

        // SAFETY: AbControl is #[repr(C)] and we just checked the buffer is
        // large enough to hold it at `ab_offset`.
        unsafe {
            let dst = misc_buf.as_mut_ptr().add(ab_offset) as *mut AbControl;
            core::ptr::write(dst, self.selector.control.clone());
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  C FFI interface
// ─────────────────────────────────────────────────────────────────────────────
//
// The following #[no_mangle] extern "C" functions are called from
// boot_main.c / the C bootloader.

/// Called by the C bootloader to run the Rust boot manager.
///
/// # Safety
/// `misc_buf` must point to at least `misc_len` readable bytes.
/// `decision_out` must point to a writable `BootDecision`.
/// Returns 0 on success, non-zero on error.
#[no_mangle]
pub unsafe extern "C" fn monoos_boot_manager_run(
    misc_buf:              *const u8,
    misc_len:              usize,
    persistent_flags_bits: u32,
    min_rollback_index:    u64,
    vol_down_held:         u8,
    vol_up_held:           u8,
    power_long_held:       u8,
    decision_out:          *mut BootDecision,
) -> i32 {
    if misc_buf.is_null() || decision_out.is_null() {
        return -1;
    }

    let buf = core::slice::from_raw_parts(misc_buf, misc_len);
    let mut mgr = BootManager::from_misc(buf, persistent_flags_bits, min_rollback_index);

    mgr.apply_key_state(vol_down_held != 0, vol_up_held != 0, power_long_held != 0);

    match mgr.decide() {
        Ok(decision) => {
            *decision_out = decision;
            0
        }
        Err(e) => {
            // Encode the error as a negative integer.
            -(e as i32 + 1)
        }
    }
}

/// Called by the C loader after the OS has confirmed a successful boot.
///
/// # Safety
/// `misc_buf` must be writable and `misc_len` bytes large.
#[no_mangle]
pub unsafe extern "C" fn monoos_mark_boot_successful(
    misc_buf:  *mut u8,
    misc_len:  usize,
    slot_index: u8,
) -> i32 {
    if misc_buf.is_null() || misc_len < core::mem::size_of::<BootStateBlock>() {
        return -1;
    }
    let slot = if slot_index == 1 { SlotSuffix::B } else { SlotSuffix::A };

    let buf = core::slice::from_raw_parts(misc_buf, misc_len);
    let mut mgr = BootManager::from_misc(buf, 0, 0);
    mgr.mark_current_boot_successful(slot);

    let out = core::slice::from_raw_parts_mut(misc_buf, misc_len);
    match mgr.flush_ab_control(out) {
        Ok(()) => 0,
        Err(e) => -(e as i32 + 1),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Panic handler (required for the freestanding no_std build)
// ─────────────────────────────────────────────────────────────────────────────
//
// boot_manager and secure_boot are both linked as staticlibs into the final
// bootloader.elf by build/scripts/build_bootloader.sh. Only one
// `#[panic_handler]` may exist in a single link unit, so boot_manager owns
// it here; secure_boot suppresses its own copy via the
// `external_panic_handler` feature, which build_bootloader.sh enables when
// compiling secure_boot for that combined link. Built standalone (e.g.
// `cargo build -p monoos-secure-boot` during development), secure_boot
// keeps its own handler so it remains independently compilable.
//
// On a real device this should ideally signal failure (e.g. write a crash
// marker to MISC and trigger a watchdog reset) rather than hang forever;
// `loop {}` is the safe minimal default until that hook is wired up.
#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
