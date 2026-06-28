//! Unit tests for the MonoOS recovery environment orchestrator.
//!
//! Run with: cargo test -p recovery_tests
//!
//! These exercise `monoos_recovery`'s public surface (`RecoveryAction`,
//! `RecoveryContext`, `RecoveryManager`, and the C FFI entry point) in an
//! isolated host-side harness, the same pattern used by boot_manager_tests
//! and secure_boot_tests.

#[cfg(test)]
mod recovery_action_tests {
    use monoos_recovery::RecoveryAction;

    #[test]
    fn parses_apply_update_cache() {
        assert_eq!(
            RecoveryAction::from_arg("--apply_update=CACHE"),
            RecoveryAction::ApplyOtaFromCache
        );
    }

    #[test]
    fn parses_apply_update_adb() {
        assert_eq!(
            RecoveryAction::from_arg("--apply_update=ADB"),
            RecoveryAction::SideloadFromUsb
        );
    }

    #[test]
    fn parses_wipe_data() {
        assert_eq!(RecoveryAction::from_arg("--wipe_data"), RecoveryAction::WipeData);
    }

    #[test]
    fn parses_wipe_data_and_media() {
        assert_eq!(
            RecoveryAction::from_arg("--wipe_data_and_media"),
            RecoveryAction::WipeDataAndMedia
        );
    }

    #[test]
    fn parses_repair_system() {
        assert_eq!(
            RecoveryAction::from_arg("--repair_system"),
            RecoveryAction::RepairSystem
        );
    }

    #[test]
    fn parses_restore_backup() {
        assert_eq!(
            RecoveryAction::from_arg("--restore_backup"),
            RecoveryAction::RestoreBackup
        );
    }

    #[test]
    fn unrecognized_arg_falls_back_to_interactive_menu() {
        assert_eq!(RecoveryAction::from_arg("--bogus"), RecoveryAction::InteractiveMenu);
        assert_eq!(RecoveryAction::from_arg(""), RecoveryAction::InteractiveMenu);
    }

    #[test]
    fn strips_trailing_nul_bytes() {
        // The MISC partition pads its recovery_arg field with NUL bytes;
        // from_arg must tolerate that, since it's exactly what the C FFI
        // entry point hands it after finding the NUL terminator.
        assert_eq!(
            RecoveryAction::from_arg("--wipe_data\0\0\0"),
            RecoveryAction::WipeData
        );
    }

    #[test]
    fn as_str_is_stable_for_every_variant() {
        // Guards against accidental renames of the on-disk action strings,
        // which would silently break MISC-partition compatibility across
        // OTA updates.
        let expected = [
            (RecoveryAction::InteractiveMenu, "interactive-menu"),
            (RecoveryAction::ApplyOtaFromCache, "apply-ota-cache"),
            (RecoveryAction::SideloadFromUsb, "sideload-usb"),
            (RecoveryAction::WipeData, "wipe-data"),
            (RecoveryAction::WipeDataAndMedia, "wipe-data-and-media"),
            (RecoveryAction::RepairSystem, "repair-system"),
            (RecoveryAction::RestoreBackup, "restore-backup"),
            (RecoveryAction::Reboot, "reboot"),
        ];
        for (action, s) in expected {
            assert_eq!(action.as_str(), s);
        }
    }
}

#[cfg(test)]
mod recovery_manager_tests {
    use monoos_recovery::{RecoveryAction, RecoveryContext, RecoveryManager, RecoveryResult};

    fn run(action: RecoveryAction) -> RecoveryResult {
        let ctx = RecoveryContext::new(action);
        RecoveryManager::new(ctx).run()
    }

    #[test]
    fn interactive_menu_auto_selects_reboot_headless() {
        // With no real key input available, the headless path must resolve
        // to a safe, deterministic outcome rather than hang.
        assert_eq!(run(RecoveryAction::InteractiveMenu), RecoveryResult::UserReboot);
    }

    #[test]
    fn apply_ota_from_cache_succeeds_with_stub_pipeline() {
        assert_eq!(run(RecoveryAction::ApplyOtaFromCache), RecoveryResult::Success);
    }

    #[test]
    fn sideload_from_usb_succeeds_with_stub_pipeline() {
        assert_eq!(run(RecoveryAction::SideloadFromUsb), RecoveryResult::Success);
    }

    #[test]
    fn wipe_data_succeeds() {
        assert_eq!(run(RecoveryAction::WipeData), RecoveryResult::Success);
    }

    #[test]
    fn wipe_data_and_media_succeeds() {
        assert_eq!(run(RecoveryAction::WipeDataAndMedia), RecoveryResult::Success);
    }

    #[test]
    fn repair_system_succeeds() {
        assert_eq!(run(RecoveryAction::RepairSystem), RecoveryResult::Success);
    }

    #[test]
    fn restore_backup_succeeds() {
        assert_eq!(run(RecoveryAction::RestoreBackup), RecoveryResult::Success);
    }

    #[test]
    fn reboot_action_is_user_reboot() {
        assert_eq!(run(RecoveryAction::Reboot), RecoveryResult::UserReboot);
    }

    #[test]
    fn new_context_defaults_to_slot_a() {
        let ctx = RecoveryContext::new(RecoveryAction::Reboot);
        assert_eq!(ctx.active_slot, 0);
        assert!(ctx.has_display);
        assert!(!ctx.adb_enabled);
    }
}

#[cfg(test)]
mod ffi_entry_tests {
    use monoos_recovery::monoos_recovery_main;

    #[test]
    fn null_arg_runs_interactive_menu_and_reboots() {
        // SAFETY: passing a null pointer is the documented "no arg" case.
        let rc = unsafe { monoos_recovery_main(core::ptr::null()) };
        assert_eq!(rc, 0, "null recovery_arg should resolve to reboot (0)");
    }

    #[test]
    fn wipe_data_arg_returns_success_code() {
        let arg = b"--wipe_data\0";
        // SAFETY: `arg` is a valid, NUL-terminated buffer for the duration
        // of this call.
        let rc = unsafe { monoos_recovery_main(arg.as_ptr()) };
        assert_eq!(rc, 0);
    }

    #[test]
    fn unterminated_garbage_does_not_read_past_256_bytes() {
        // monoos_recovery_main caps its scan at 256 bytes even without a
        // NUL terminator, so this must not crash, hang, or read out of
        // bounds (verified here under Miri-free conditions; the cap itself
        // is exercised, not undefined-behavior-checked).
        let buf = [b'x'; 512];
        let rc = unsafe { monoos_recovery_main(buf.as_ptr()) };
        // Garbage input that isn't a recognized flag falls back to the
        // interactive menu, which auto-reboots headless.
        assert_eq!(rc, 0);
    }
}
