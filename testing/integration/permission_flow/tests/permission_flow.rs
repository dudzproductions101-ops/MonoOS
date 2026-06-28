//! Integration test: full permission request -> grant -> kernel-bit flow.

use std::collections::HashMap;

// Minimal standalone simulation of the permission pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Perm { Camera, Mic, Location, Storage }

impl Perm {
    fn bit(self) -> u32 {
        match self { Perm::Camera => 1, Perm::Mic => 2, Perm::Location => 4, Perm::Storage => 16 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Grant { NotAsked, Granted, Denied, PermDenied }

struct PermSystem {
    grants:      HashMap<(u32, Perm), Grant>, // (uid, perm) -> state
    kernel_bits: HashMap<u32, u32>,           // uid -> active bitmask
}

impl PermSystem {
    fn new() -> Self { PermSystem { grants: HashMap::new(), kernel_bits: HashMap::new() } }

    fn request(&mut self, uid: u32, perm: Perm, user_grants: bool) {
        match self.grants.get(&(uid, perm)) {
            Some(Grant::PermDenied) => return,
            _ => {}
        }
        let state = if user_grants { Grant::Granted } else { Grant::Denied };
        self.grants.insert((uid, perm), state);
        if state == Grant::Granted {
            *self.kernel_bits.entry(uid).or_insert(0) |= perm.bit();
        } else {
            let bits = self.kernel_bits.entry(uid).or_insert(0);
            *bits &= !perm.bit();
        }
    }

    fn revoke(&mut self, uid: u32, perm: Perm) {
        self.grants.insert((uid, perm), Grant::Denied);
        let bits = self.kernel_bits.entry(uid).or_insert(0);
        *bits &= !perm.bit();
    }

    fn has_perm(&self, uid: u32, perm: Perm) -> bool {
        *self.kernel_bits.get(&uid).unwrap_or(&0) & perm.bit() != 0
    }

    fn grant_state(&self, uid: u32, perm: Perm) -> Grant {
        *self.grants.get(&(uid, perm)).unwrap_or(&Grant::NotAsked)
    }
}

#[test]
fn grant_sets_kernel_bit() {
    let mut sys = PermSystem::new();
    sys.request(10001, Perm::Camera, true);
    assert!(sys.has_perm(10001, Perm::Camera));
}

#[test]
fn deny_clears_kernel_bit() {
    let mut sys = PermSystem::new();
    sys.request(10001, Perm::Mic, true);
    assert!(sys.has_perm(10001, Perm::Mic));
    sys.revoke(10001, Perm::Mic);
    assert!(!sys.has_perm(10001, Perm::Mic));
}

#[test]
fn perm_denied_blocks_future_requests() {
    let mut sys = PermSystem::new();
    sys.grants.insert((10002, Perm::Location), Grant::PermDenied);
    sys.request(10002, Perm::Location, true); // user tries to grant, but system blocks
    assert_eq!(sys.grant_state(10002, Perm::Location), Grant::PermDenied);
    assert!(!sys.has_perm(10002, Perm::Location));
}

#[test]
fn isolated_between_uids() {
    let mut sys = PermSystem::new();
    sys.request(10001, Perm::Storage, true);
    assert!(sys.has_perm(10001, Perm::Storage));
    assert!(!sys.has_perm(10002, Perm::Storage));
}

#[test]
fn multiple_perms_bitmask_correct() {
    let mut sys = PermSystem::new();
    sys.request(10003, Perm::Camera,  true);
    sys.request(10003, Perm::Mic,     true);
    sys.request(10003, Perm::Storage, false);
    let bits = *sys.kernel_bits.get(&10003).unwrap_or(&0);
    assert_eq!(bits & Perm::Camera.bit(),  Perm::Camera.bit());
    assert_eq!(bits & Perm::Mic.bit(),     Perm::Mic.bit());
    assert_eq!(bits & Perm::Storage.bit(), 0);
}
