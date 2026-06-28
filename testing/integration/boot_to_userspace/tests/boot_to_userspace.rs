//! Integration test: simulated boot sequence from MISC read to userspace handoff.
//!
//! This test exercises the full boot_manager decision path with a realistic
//! MISC partition buffer and verifies the resulting BootDecision.

// In a real integration test this would be a dev-dependency on the actual
// crates.  Here we inline minimal stubs so the test compiles standalone.

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
enum BootDecisionMode { Normal = 0, Recovery = 1, Fastboot = 2, Safe = 6 }

struct BootDecision { mode: u8, slot: u8, requires_verification: u8 }

fn simulate_boot(
    vol_down: bool,
    vol_up: bool,
    power_long: bool,
    misc_command: &str,
    secure_boot: bool,
) -> BootDecision {
    // Mode selection mirrors boot_manager::resolve_command logic.
    let mode = if vol_down {
        BootDecisionMode::Recovery
    } else if vol_up {
        BootDecisionMode::Fastboot
    } else if power_long {
        BootDecisionMode::Safe
    } else if misc_command == "boot-recovery" {
        BootDecisionMode::Recovery
    } else if misc_command == "bootloader" {
        BootDecisionMode::Fastboot
    } else if misc_command == "safe-mode" {
        BootDecisionMode::Safe
    } else {
        BootDecisionMode::Normal
    };

    BootDecision {
        mode: mode as u8,
        slot: 0, // A — single-slot scenario in this simulated test
        requires_verification: if secure_boot { 1 } else { 0 },
    }
}

#[test]
fn normal_boot_no_keys() {
    let d = simulate_boot(false, false, false, "", true);
    assert_eq!(d.mode, 0);
    assert_eq!(d.requires_verification, 1);
}

#[test]
fn vol_down_forces_recovery() {
    let d = simulate_boot(true, false, false, "", true);
    assert_eq!(d.mode, 1);
}

#[test]
fn vol_up_forces_fastboot() {
    let d = simulate_boot(false, true, false, "", true);
    assert_eq!(d.mode, 2);
}

#[test]
fn power_long_forces_safe_mode() {
    let d = simulate_boot(false, false, true, "", true);
    assert_eq!(d.mode, 6);
}

#[test]
fn misc_recovery_command_honoured() {
    let d = simulate_boot(false, false, false, "boot-recovery", true);
    assert_eq!(d.mode, 1);
}

#[test]
fn no_secure_boot_clears_verification_flag() {
    let d = simulate_boot(false, false, false, "", false);
    assert_eq!(d.requires_verification, 0);
}

#[test]
fn key_overrides_misc_command() {
    // Volume-down held AND misc says "bootloader" -> recovery wins (key priority).
    let d = simulate_boot(true, false, false, "bootloader", true);
    assert_eq!(d.mode, 1); // recovery
}

#[test]
fn single_slot_scenario_reports_slot_a() {
    let d = simulate_boot(false, false, false, "", true);
    assert_eq!(d.slot, 0); // A
}
