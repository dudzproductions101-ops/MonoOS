//! Unit tests for the MonoOS framework permission manager.

// These tests exercise framework/permissions/permission_manager.rs
// behaviour directly by duplicating the relevant types in a thin shim.
// In the real test binary, the framework crate would be a dev-dependency.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission { Camera, Microphone, Location, Storage }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantState { NotRequested, Granted, Denied, PermanentlyDenied }

use std::collections::HashMap;

pub struct PermissionStore {
    grants: HashMap<(String, Permission), GrantState>,
}

impl PermissionStore {
    pub fn new() -> Self { PermissionStore { grants: HashMap::new() } }
    pub fn set(&mut self, pkg: &str, perm: Permission, state: GrantState) {
        self.grants.insert((pkg.to_owned(), perm), state);
    }
    pub fn get(&self, pkg: &str, perm: Permission) -> GrantState {
        *self.grants.get(&(pkg.to_owned(), perm)).unwrap_or(&GrantState::NotRequested)
    }
    pub fn is_granted(&self, pkg: &str, perm: Permission) -> bool {
        self.get(pkg, perm) == GrantState::Granted
    }
    pub fn can_request(&self, pkg: &str, perm: Permission) -> bool {
        self.get(pkg, perm) != GrantState::PermanentlyDenied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_not_requested() {
        let store = PermissionStore::new();
        assert_eq!(store.get("com.app", Permission::Camera), GrantState::NotRequested);
    }

    #[test]
    fn grant_and_check() {
        let mut s = PermissionStore::new();
        s.set("com.app", Permission::Camera, GrantState::Granted);
        assert!(s.is_granted("com.app", Permission::Camera));
        assert!(!s.is_granted("com.app", Permission::Microphone));
    }

    #[test]
    fn permanently_denied_blocks_request() {
        let mut s = PermissionStore::new();
        s.set("com.spy", Permission::Microphone, GrantState::PermanentlyDenied);
        assert!(!s.can_request("com.spy", Permission::Microphone));
        assert!(s.can_request("com.spy", Permission::Camera));
    }

    #[test]
    fn deny_still_allows_request() {
        let mut s = PermissionStore::new();
        s.set("com.app", Permission::Location, GrantState::Denied);
        assert!(s.can_request("com.app", Permission::Location));
    }

    #[test]
    fn isolated_per_package() {
        let mut s = PermissionStore::new();
        s.set("com.a", Permission::Storage, GrantState::Granted);
        assert!(s.is_granted("com.a", Permission::Storage));
        assert!(!s.is_granted("com.b", Permission::Storage));
    }
}
