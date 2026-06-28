//! Unit tests for the MonoOS boot manager.
//!
//! Run with: cargo test -p boot_manager_tests

#[cfg(test)]
mod boot_state_tests {
    // The boot_manager crate types are re-tested here in an isolated
    // harness so coverage tooling can attribute lines correctly.

    // BootCommand round-trips through its string representation.
    #[test]
    fn boot_command_roundtrip() {
        use monoos_boot_manager::boot_state::BootCommand;
        for cmd in [
            BootCommand::Normal,
            BootCommand::BootRecovery,
            BootCommand::ApplyUpdate,
            BootCommand::WipeData,
            BootCommand::Fastboot,
            BootCommand::Diagnostics,
            BootCommand::SafeMode,
        ] {
            let s   = cmd.as_str();
            let cmd2 = BootCommand::from_str(s);
            assert_eq!(cmd, cmd2, "roundtrip failed for {:?}", cmd);
        }
    }

    #[test]
    fn normal_command_has_empty_string() {
        use monoos_boot_manager::boot_state::BootCommand;
        assert_eq!(BootCommand::Normal.as_str(), "");
    }
}

#[cfg(test)]
mod boot_flags_tests {
    use monoos_boot_manager::boot_flags::BootFlags;

    #[test]
    fn factory_defaults_enable_secure_boot() {
        let f = BootFlags::factory_defaults();
        assert!(f.secure_boot_enabled());
    }

    #[test]
    fn oem_unlocked_not_set_by_default() {
        let f = BootFlags::factory_defaults();
        assert!(!f.oem_unlocked());
    }

    #[test]
    fn persistent_bits_roundtrip() {
        let f    = BootFlags::factory_defaults();
        let bits = f.persistent_bits();
        let f2   = BootFlags::from_persistent_bits(bits);
        assert_eq!(f.persistent_bits(), f2.persistent_bits());
    }

    #[test]
    fn set_and_clear_transient_flag() {
        use monoos_boot_manager::boot_flags::TransientFlags;
        let mut f = BootFlags::factory_defaults();
        f.set_transient(TransientFlags::KERNEL_VERIFIED, true);
        assert!(f.kernel_verified());
        f.set_transient(TransientFlags::KERNEL_VERIFIED, false);
        assert!(!f.kernel_verified());
    }
}

#[cfg(test)]
mod kernel_selector_tests {
    use monoos_boot_manager::{
        kernel_selector::{AbControl, KernelSelector, KernelVersion},
        partition_manager::SlotSuffix,
    };

    #[test]
    fn selects_higher_priority_slot() {
        let mut ctrl = AbControl::new_default();
        ctrl.slot_a.priority = 15;
        ctrl.slot_b.priority = 10;
        let mut sel = KernelSelector::new(ctrl, 0);
        assert_eq!(sel.select_slot(), Some(SlotSuffix::A));
    }

    #[test]
    fn exhausted_slot_not_selected() {
        let mut ctrl = AbControl::new_default();
        ctrl.slot_a.tries_remaining = 0;
        ctrl.slot_a.successful_boot = 0;
        ctrl.slot_b.priority        = 14;
        let mut sel = KernelSelector::new(ctrl, 0);
        assert_eq!(sel.select_slot(), Some(SlotSuffix::B));
    }

    #[test]
    fn both_slots_exhausted_returns_none() {
        let mut ctrl = AbControl::new_default();
        ctrl.slot_a.tries_remaining = 0;
        ctrl.slot_a.successful_boot = 0;
        ctrl.slot_b.tries_remaining = 0;
        ctrl.slot_b.successful_boot = 0;
        let mut sel = KernelSelector::new(ctrl, 0);
        assert_eq!(sel.select_slot(), None);
    }

    #[test]
    fn kernel_version_parse() {
        let v = KernelVersion::parse("6.6.42-monoos").unwrap();
        assert_eq!(v.major, 6);
        assert_eq!(v.minor, 6);
        assert_eq!(v.patch, 42);
    }

    #[test]
    fn rollback_check_rejects_old_version() {
        let ctrl = AbControl::new_default();
        let sel  = KernelSelector::new(ctrl, 0);
        let min  = KernelVersion::new(6, 0, 0);
        let old  = KernelVersion::new(5, 15, 0);
        assert!(!sel.passes_rollback_check(old, min));
    }

    #[test]
    fn rollback_check_accepts_newer_or_equal_version() {
        let ctrl = AbControl::new_default();
        let sel  = KernelSelector::new(ctrl, 0);
        let min  = KernelVersion::new(6, 0, 0);
        assert!(sel.passes_rollback_check(min, min));
        assert!(sel.passes_rollback_check(KernelVersion::new(6, 6, 42), min));
    }
}
