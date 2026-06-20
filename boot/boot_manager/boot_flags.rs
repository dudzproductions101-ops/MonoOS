//! boot_flags.rs – Bit-field flags that influence the boot process
//!
//! Flags are persisted in the BootStateBlock's reserved region (at a
//! well-known byte offset) so they survive reboots without requiring a
//! separate partition.  They are also set transiently via the kernel
//! command line for flags that apply only to the current session.

use core::fmt;

// ─────────────────────────────────────────────────────────────────────────────
//  Persistent boot flags (stored in MISC partition)
// ─────────────────────────────────────────────────────────────────────────────

bitflags::bitflags! {
    /// Flags stored in the MISC partition that persist across reboots.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PersistentFlags: u32 {
        /// Secure Boot enforcement is active.
        const SECURE_BOOT       = 1 << 0;
        /// DM-Verity is enforcing (not just logging) on the system partition.
        const VERITY_ENFORCING  = 1 << 1;
        /// OEM-unlock has been granted; bootloader is unlocked.
        const OEM_UNLOCKED      = 1 << 2;
        /// A pending OTA slot-switch has been scheduled.
        const OTA_PENDING       = 1 << 3;
        /// Roll-back protection is active.
        const ROLLBACK_PROTECT  = 1 << 4;
        /// Hardware-backed key storage is required (TEE / SE).
        const HW_KEY_REQUIRED   = 1 << 5;
        /// Factory test mode has been completed.
        const FACTORY_TESTED    = 1 << 6;
        /// The first-boot setup wizard has not yet been completed.
        const FIRST_BOOT        = 1 << 7;
        /// USB debugging (ADB) is persistently enabled.
        const ADB_ENABLED       = 1 << 8;
        /// Developer options are unlocked.
        const DEV_OPTIONS       = 1 << 9;
        /// Privacy mode: all radios off at boot until user authenticates.
        const PRIVACY_BOOT      = 1 << 10;
        /// Emergency contact boot (minimal UI, call/SMS only).
        const EMERGENCY_MODE    = 1 << 11;
    }
}

impl Default for PersistentFlags {
    /// Sane defaults for a freshly provisioned device.
    fn default() -> Self {
        PersistentFlags::SECURE_BOOT
            | PersistentFlags::VERITY_ENFORCING
            | PersistentFlags::ROLLBACK_PROTECT
            | PersistentFlags::HW_KEY_REQUIRED
            | PersistentFlags::FIRST_BOOT
    }
}

impl fmt::Display for PersistentFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PersistentFlags({:#010x})", self.bits())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Transient boot flags (valid only for current boot session)
// ─────────────────────────────────────────────────────────────────────────────

bitflags::bitflags! {
    /// Flags derived at boot time from hardware GPIO, kernel command-line
    /// arguments, or secure enclave state.  Not persisted.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TransientFlags: u32 {
        /// Hardware volume-down key was held at power-on (→ recovery).
        const KEY_VOL_DOWN_HELD     = 1 << 0;
        /// Hardware volume-up key was held at power-on (→ fastboot).
        const KEY_VOL_UP_HELD       = 1 << 1;
        /// Power key was held > 10 s (→ forced reboot into safe mode).
        const KEY_POWER_LONG        = 1 << 2;
        /// Battery level too low to apply an OTA update safely.
        const BATTERY_TOO_LOW       = 1 << 3;
        /// AC / USB charger is connected.
        const CHARGER_PRESENT       = 1 << 4;
        /// Kernel signature was verified successfully by the bootloader.
        const KERNEL_VERIFIED       = 1 << 5;
        /// Initramfs signature was verified successfully.
        const INITRAMFS_VERIFIED    = 1 << 6;
        /// TEE / secure world is accessible and responded to probe.
        const TEE_AVAILABLE         = 1 << 7;
        /// RAMDUMP mode requested (capture kernel crash dump).
        const RAMDUMP_REQUESTED     = 1 << 8;
        /// The watchdog triggered the last reset.
        const WDT_RESET             = 1 << 9;
        /// Panic or hard lockup caused the last reset.
        const PANIC_RESET           = 1 << 10;
        /// Thermal shutdown caused the last reset.
        const THERMAL_RESET         = 1 << 11;
        /// USB is connected and enumerated.
        const USB_CONNECTED         = 1 << 12;
    }
}

impl Default for TransientFlags {
    fn default() -> Self {
        TransientFlags::empty()
    }
}

impl fmt::Display for TransientFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TransientFlags({:#010x})", self.bits())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Combined BootFlags accessor
// ─────────────────────────────────────────────────────────────────────────────

/// Aggregated view of both persistent and transient flags for the current boot.
#[derive(Debug, Clone, Copy)]
pub struct BootFlags {
    pub persistent: PersistentFlags,
    pub transient:  TransientFlags,
}

impl BootFlags {
    /// Construct from stored persistent flags (transient flags start empty).
    pub fn new(persistent: PersistentFlags) -> Self {
        BootFlags {
            persistent,
            transient: TransientFlags::default(),
        }
    }

    /// Convenience: create with factory defaults.
    pub fn factory_defaults() -> Self {
        BootFlags::new(PersistentFlags::default())
    }

    // ── persistent flag helpers ──────────────────────────────────────────────

    pub fn secure_boot_enabled(&self) -> bool {
        self.persistent.contains(PersistentFlags::SECURE_BOOT)
    }

    pub fn verity_enforcing(&self) -> bool {
        self.persistent.contains(PersistentFlags::VERITY_ENFORCING)
    }

    pub fn oem_unlocked(&self) -> bool {
        self.persistent.contains(PersistentFlags::OEM_UNLOCKED)
    }

    pub fn ota_pending(&self) -> bool {
        self.persistent.contains(PersistentFlags::OTA_PENDING)
    }

    pub fn is_first_boot(&self) -> bool {
        self.persistent.contains(PersistentFlags::FIRST_BOOT)
    }

    pub fn privacy_boot_enabled(&self) -> bool {
        self.persistent.contains(PersistentFlags::PRIVACY_BOOT)
    }

    pub fn rollback_protection(&self) -> bool {
        self.persistent.contains(PersistentFlags::ROLLBACK_PROTECT)
    }

    // ── transient flag helpers ───────────────────────────────────────────────

    pub fn recovery_key_held(&self) -> bool {
        self.transient.contains(TransientFlags::KEY_VOL_DOWN_HELD)
    }

    pub fn fastboot_key_held(&self) -> bool {
        self.transient.contains(TransientFlags::KEY_VOL_UP_HELD)
    }

    pub fn kernel_verified(&self) -> bool {
        self.transient.contains(TransientFlags::KERNEL_VERIFIED)
    }

    pub fn initramfs_verified(&self) -> bool {
        self.transient.contains(TransientFlags::INITRAMFS_VERIFIED)
    }

    pub fn tee_available(&self) -> bool {
        self.transient.contains(TransientFlags::TEE_AVAILABLE)
    }

    pub fn watchdog_reset(&self) -> bool {
        self.transient.contains(TransientFlags::WDT_RESET)
    }

    pub fn panic_reset(&self) -> bool {
        self.transient.contains(TransientFlags::PANIC_RESET)
    }

    pub fn battery_too_low(&self) -> bool {
        self.transient.contains(TransientFlags::BATTERY_TOO_LOW)
    }

    pub fn charger_present(&self) -> bool {
        self.transient.contains(TransientFlags::CHARGER_PRESENT)
    }

    // ── mutation helpers ─────────────────────────────────────────────────────

    pub fn set_persistent(&mut self, flag: PersistentFlags, value: bool) {
        if value {
            self.persistent.insert(flag);
        } else {
            self.persistent.remove(flag);
        }
    }

    pub fn set_transient(&mut self, flag: TransientFlags, value: bool) {
        if value {
            self.transient.insert(flag);
        } else {
            self.transient.remove(flag);
        }
    }

    /// Mark both kernel and initramfs as verified.
    pub fn mark_images_verified(&mut self) {
        self.transient.insert(TransientFlags::KERNEL_VERIFIED);
        self.transient.insert(TransientFlags::INITRAMFS_VERIFIED);
    }

    /// Both images verified → secure session.
    pub fn is_trusted_boot(&self) -> bool {
        self.kernel_verified() && self.initramfs_verified() && self.secure_boot_enabled()
    }

    /// Serialize persistent flags to a u32 for MISC partition storage.
    pub fn persistent_bits(&self) -> u32 {
        self.persistent.bits()
    }

    /// Deserialize persistent flags from a stored u32.
    pub fn from_persistent_bits(bits: u32) -> Self {
        let p = PersistentFlags::from_bits_truncate(bits);
        BootFlags::new(p)
    }
}

impl fmt::Display for BootFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BootFlags {{ {} | {} }}", self.persistent, self.transient)
    }
}
