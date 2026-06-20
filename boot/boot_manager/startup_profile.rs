//! startup_profile.rs – Define which services and features are started for
//! each boot mode, and collect timing metrics for the boot sequence.

use crate::boot_state::BootCommand;

// ─────────────────────────────────────────────────────────────────────────────
//  ServiceSet – which system services to launch
// ─────────────────────────────────────────────────────────────────────────────

/// A bitmask of system services / features that should be activated during
/// a particular boot profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceSet(u64);

impl ServiceSet {
    // Core services (always present in normal mode)
    pub const INIT:             ServiceSet = ServiceSet(1 << 0);
    pub const DBUS:             ServiceSet = ServiceSet(1 << 1);
    pub const UDEV:             ServiceSet = ServiceSet(1 << 2);
    pub const LOGD:             ServiceSet = ServiceSet(1 << 3);
    pub const CRYPTO:           ServiceSet = ServiceSet(1 << 4);
    pub const SYSTEM_SERVER:    ServiceSet = ServiceSet(1 << 5);
    pub const SURFACE_FLINGER:  ServiceSet = ServiceSet(1 << 6); // Wayland compositor
    pub const INPUT:            ServiceSet = ServiceSet(1 << 7);
    pub const AUDIO:            ServiceSet = ServiceSet(1 << 8);
    pub const WIFI:             ServiceSet = ServiceSet(1 << 9);
    pub const TELEPHONY:        ServiceSet = ServiceSet(1 << 10);
    pub const BLUETOOTH:        ServiceSet = ServiceSet(1 << 11);
    pub const GPS:              ServiceSet = ServiceSet(1 << 12);
    pub const CAMERA:           ServiceSet = ServiceSet(1 << 13);
    pub const NFC:              ServiceSet = ServiceSet(1 << 14);
    pub const SENSOR:           ServiceSet = ServiceSet(1 << 15);
    pub const NETWORK:          ServiceSet = ServiceSet(1 << 16);
    pub const ADB:              ServiceSet = ServiceSet(1 << 17);
    pub const UPDATE_ENGINE:    ServiceSet = ServiceSet(1 << 18);
    pub const PACKAGE_MANAGER:  ServiceSet = ServiceSet(1 << 19);
    pub const ACCOUNT_SERVICE:  ServiceSet = ServiceSet(1 << 20);
    pub const PRIVACY_GUARD:    ServiceSet = ServiceSet(1 << 21);
    pub const BACKUP:           ServiceSet = ServiceSet(1 << 22);
    pub const CAST:             ServiceSet = ServiceSet(1 << 23);
    pub const HOTSPOT:          ServiceSet = ServiceSet(1 << 24);
    pub const EMERGENCY_CALLS:  ServiceSet = ServiceSet(1 << 25);
    pub const THIRD_PARTY_APPS: ServiceSet = ServiceSet(1 << 26);

    pub const EMPTY: ServiceSet = ServiceSet(0);

    pub fn contains(self, other: ServiceSet) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn insert(self, other: ServiceSet) -> ServiceSet {
        ServiceSet(self.0 | other.0)
    }

    pub fn remove(self, other: ServiceSet) -> ServiceSet {
        ServiceSet(self.0 & !other.0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  KernelArgs – kernel command-line additions per profile
// ─────────────────────────────────────────────────────────────────────────────

/// Additional kernel cmdline tokens appended for a specific startup profile.
/// Each string is a static `&str` slice (no heap allocation).
#[derive(Debug, Clone, Copy)]
pub struct KernelArgs {
    pub args:  &'static [&'static str],
}

// ─────────────────────────────────────────────────────────────────────────────
//  StartupProfile
// ─────────────────────────────────────────────────────────────────────────────

/// A startup profile fully describes what runs, how, and with which kernel
/// arguments for a given combination of BootCommand + BootFlags.
#[derive(Debug, Clone, Copy)]
pub struct StartupProfile {
    /// Human-readable name for logging.
    pub name:             &'static str,
    /// The systemd/init target unit to activate.
    pub init_target:      &'static str,
    /// Services included in this profile.
    pub services:         ServiceSet,
    /// Extra kernel cmdline tokens for this mode.
    pub kernel_args:      KernelArgs,
    /// Whether the display must be usable before pivoting to userspace.
    pub require_display:  bool,
    /// Whether network drivers are started early (needed for net-root).
    pub early_network:    bool,
    /// Whether adb/fastboot are allowed in this profile.
    pub adb_allowed:      bool,
    /// Whether third-party app services run.
    pub apps_allowed:     bool,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Built-in profiles
// ─────────────────────────────────────────────────────────────────────────────

/// Standard full-featured boot profile.
pub const PROFILE_NORMAL: StartupProfile = StartupProfile {
    name:            "normal",
    init_target:     "graphical.target",
    services: ServiceSet(
        ServiceSet::INIT.0
            | ServiceSet::DBUS.0
            | ServiceSet::UDEV.0
            | ServiceSet::LOGD.0
            | ServiceSet::CRYPTO.0
            | ServiceSet::SYSTEM_SERVER.0
            | ServiceSet::SURFACE_FLINGER.0
            | ServiceSet::INPUT.0
            | ServiceSet::AUDIO.0
            | ServiceSet::WIFI.0
            | ServiceSet::TELEPHONY.0
            | ServiceSet::BLUETOOTH.0
            | ServiceSet::GPS.0
            | ServiceSet::CAMERA.0
            | ServiceSet::SENSOR.0
            | ServiceSet::NETWORK.0
            | ServiceSet::UPDATE_ENGINE.0
            | ServiceSet::PACKAGE_MANAGER.0
            | ServiceSet::ACCOUNT_SERVICE.0
            | ServiceSet::PRIVACY_GUARD.0
            | ServiceSet::EMERGENCY_CALLS.0
            | ServiceSet::THIRD_PARTY_APPS.0
    ),
    kernel_args: KernelArgs {
        args: &["oneos.mode=normal", "quiet", "loglevel=3"],
    },
    require_display: true,
    early_network:   false,
    adb_allowed:     false,
    apps_allowed:    true,
};

/// Recovery profile: minimal, ADB-enabled, no third-party apps.
pub const PROFILE_RECOVERY: StartupProfile = StartupProfile {
    name:            "recovery",
    init_target:     "recovery.target",
    services: ServiceSet(
        ServiceSet::INIT.0
            | ServiceSet::UDEV.0
            | ServiceSet::LOGD.0
            | ServiceSet::CRYPTO.0
            | ServiceSet::INPUT.0
            | ServiceSet::SURFACE_FLINGER.0
            | ServiceSet::ADB.0
            | ServiceSet::NETWORK.0
    ),
    kernel_args: KernelArgs {
        args: &[
            "oneos.mode=recovery",
            "systemd.unit=recovery.target",
            "ro",
            "loglevel=7",
        ],
    },
    require_display: true,
    early_network:   false,
    adb_allowed:     true,
    apps_allowed:    false,
};

/// Fastboot / bootloader profile: extremely minimal, USB only.
pub const PROFILE_FASTBOOT: StartupProfile = StartupProfile {
    name:            "fastboot",
    init_target:     "fastboot.target",
    services: ServiceSet(
        ServiceSet::INIT.0
            | ServiceSet::UDEV.0
            | ServiceSet::LOGD.0
            | ServiceSet::ADB.0
    ),
    kernel_args: KernelArgs {
        args: &["oneos.mode=fastboot", "loglevel=7"],
    },
    require_display: false,
    early_network:   false,
    adb_allowed:     true,
    apps_allowed:    false,
};

/// Diagnostics profile: read-only rootfs, verbose logging, all sensors.
pub const PROFILE_DIAGNOSTICS: StartupProfile = StartupProfile {
    name:            "diagnostics",
    init_target:     "diagnostic.target",
    services: ServiceSet(
        ServiceSet::INIT.0
            | ServiceSet::DBUS.0
            | ServiceSet::UDEV.0
            | ServiceSet::LOGD.0
            | ServiceSet::SENSOR.0
            | ServiceSet::AUDIO.0
            | ServiceSet::CAMERA.0
            | ServiceSet::NETWORK.0
            | ServiceSet::ADB.0
    ),
    kernel_args: KernelArgs {
        args: &[
            "oneos.mode=diagnostic",
            "loglevel=7",
            "ro",
            "systemd.unit=diagnostic.target",
        ],
    },
    require_display: true,
    early_network:   true,
    adb_allowed:     true,
    apps_allowed:    false,
};

/// Safe mode: no third-party apps, no vendor drivers beyond those in ROM.
pub const PROFILE_SAFE: StartupProfile = StartupProfile {
    name:            "safe",
    init_target:     "graphical.target",
    services: ServiceSet(
        ServiceSet::INIT.0
            | ServiceSet::DBUS.0
            | ServiceSet::UDEV.0
            | ServiceSet::LOGD.0
            | ServiceSet::CRYPTO.0
            | ServiceSet::SYSTEM_SERVER.0
            | ServiceSet::SURFACE_FLINGER.0
            | ServiceSet::INPUT.0
            | ServiceSet::TELEPHONY.0
            | ServiceSet::EMERGENCY_CALLS.0
            | ServiceSet::PRIVACY_GUARD.0
    ),
    kernel_args: KernelArgs {
        args: &["oneos.mode=safe", "oneos.safe_mode=1", "loglevel=4"],
    },
    require_display: true,
    early_network:   false,
    adb_allowed:     false,
    apps_allowed:    false,
};

// ─────────────────────────────────────────────────────────────────────────────
//  Profile selection
// ─────────────────────────────────────────────────────────────────────────────

/// Select the appropriate startup profile from a boot command.
pub fn select_profile(cmd: BootCommand, adb_enabled: bool) -> &'static StartupProfile {
    let profile = match cmd {
        BootCommand::Normal       => &PROFILE_NORMAL,
        BootCommand::BootRecovery => &PROFILE_RECOVERY,
        BootCommand::ApplyUpdate  => &PROFILE_RECOVERY,
        BootCommand::WipeData     => &PROFILE_RECOVERY,
        BootCommand::Fastboot     => &PROFILE_FASTBOOT,
        BootCommand::Diagnostics  => &PROFILE_DIAGNOSTICS,
        BootCommand::SafeMode     => &PROFILE_SAFE,
    };
    // Note: in a real impl we'd clone and patch adb_allowed based on the
    // persistent flag; here we return the static ref.
    profile
}

// ─────────────────────────────────────────────────────────────────────────────
//  Boot timing metrics
// ─────────────────────────────────────────────────────────────────────────────

/// Microsecond timestamps for boot phases, populated as each phase completes.
/// A value of 0 means "not yet reached".
#[derive(Debug, Clone, Copy, Default)]
pub struct BootTimings {
    pub power_on_us:         u64,
    pub bootloader_start_us: u64,
    pub cpu_init_us:         u64,
    pub memory_map_us:       u64,
    pub kernel_load_us:      u64,
    pub initramfs_load_us:   u64,
    pub secure_boot_us:      u64,
    pub handoff_us:          u64,
}

impl BootTimings {
    pub fn new() -> Self {
        BootTimings::default()
    }

    /// Duration from power-on to Linux handoff, in milliseconds.
    pub fn total_ms(&self) -> u64 {
        if self.handoff_us > self.power_on_us {
            (self.handoff_us - self.power_on_us) / 1000
        } else {
            0
        }
    }
}
