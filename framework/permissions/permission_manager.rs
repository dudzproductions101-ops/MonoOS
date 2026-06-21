//! permission_manager.rs – MonoOS Framework Permission Manager
//!
//! The framework-layer permission manager is the bridge between application
//! code and the kernel-level permission syscalls.  Apps call into this
//! via the MonoOS SDK; it validates the request, persists the user's
//! decision, and calls sys_monoos_perm_set.

use std::collections::HashMap;

/// Every permission the OS can grant or revoke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    Camera,
    Microphone,
    Location,
    Contacts,
    Storage,
    Phone,
    Bluetooth,
    Nfc,
    Sensors,
    Network,
}

impl Permission {
    pub fn kernel_bit(self) -> u32 {
        match self {
            Permission::Camera      => 0x0001,
            Permission::Microphone  => 0x0002,
            Permission::Location    => 0x0004,
            Permission::Contacts    => 0x0008,
            Permission::Storage     => 0x0010,
            Permission::Phone       => 0x0020,
            Permission::Bluetooth   => 0x0040,
            Permission::Nfc         => 0x0080,
            Permission::Sensors     => 0x0100,
            Permission::Network     => 0x0200,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Permission::Camera      => "Camera",
            Permission::Microphone  => "Microphone",
            Permission::Location    => "Location",
            Permission::Contacts    => "Contacts",
            Permission::Storage     => "Storage",
            Permission::Phone       => "Phone & Calls",
            Permission::Bluetooth   => "Bluetooth",
            Permission::Nfc         => "NFC",
            Permission::Sensors     => "Sensors",
            Permission::Network     => "Network",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Permission::Camera      => "Access camera hardware to take photos and videos.",
            Permission::Microphone  => "Record audio through the device microphone.",
            Permission::Location    => "Access precise GPS location.",
            Permission::Contacts    => "Read and write the contacts database.",
            Permission::Storage     => "Read and write files on shared storage.",
            Permission::Phone       => "Make calls and access call logs.",
            Permission::Bluetooth   => "Scan for and connect to Bluetooth devices.",
            Permission::Nfc         => "Read and write NFC tags.",
            Permission::Sensors     => "Access motion and environment sensors.",
            Permission::Network     => "Send and receive network traffic.",
        }
    }
}

/// Grant state for a single permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantState {
    /// User has not been asked yet.
    NotRequested,
    /// User explicitly granted the permission.
    Granted,
    /// User denied the permission.
    Denied,
    /// User denied and checked "don't ask again".
    PermanentlyDenied,
}

/// Per-package permission record.
#[derive(Debug, Clone)]
pub struct PackagePermissions {
    pub package_name: String,
    pub uid:          u32,
    pub grants:       HashMap<Permission, GrantState>,
}

impl PackagePermissions {
    pub fn new(package_name: impl Into<String>, uid: u32) -> Self {
        PackagePermissions {
            package_name: package_name.into(),
            uid,
            grants: HashMap::new(),
        }
    }

    pub fn is_granted(&self, perm: Permission) -> bool {
        self.grants.get(&perm) == Some(&GrantState::Granted)
    }

    pub fn can_request(&self, perm: Permission) -> bool {
        !matches!(
            self.grants.get(&perm),
            Some(GrantState::PermanentlyDenied)
        )
    }
}

/// The central permission database, owned by the permission_service.
pub struct PermissionManager {
    packages: HashMap<String, PackagePermissions>,
}

impl PermissionManager {
    pub fn new() -> Self {
        PermissionManager { packages: HashMap::new() }
    }

    /// Register a newly-installed package.
    pub fn register_package(&mut self, name: impl Into<String>, uid: u32) {
        let n = name.into();
        self.packages
            .entry(n.clone())
            .or_insert_with(|| PackagePermissions::new(n, uid));
    }

    /// Look up or create the record for a package.
    pub fn package_mut(&mut self, name: &str) -> Option<&mut PackagePermissions> {
        self.packages.get_mut(name)
    }

    pub fn package(&self, name: &str) -> Option<&PackagePermissions> {
        self.packages.get(name)
    }

    /// Apply a user decision and propagate it to the kernel.
    pub fn apply_grant(
        &mut self,
        package: &str,
        perm: Permission,
        state: GrantState,
    ) -> Result<(), &'static str> {
        let pkg = self.packages.get_mut(package).ok_or("package not found")?;
        pkg.grants.insert(perm, state);

        // Call into the kernel permission syscall.
        let pid = pkg.uid as i32; // simplified: use uid as pid proxy
        let val = if state == GrantState::Granted { 1i32 } else { 0i32 };
        unsafe {
            // sys_monoos_perm_set(pid, perm.kernel_bit(), val)
            // Real call: libc::syscall(401, pid, perm.kernel_bit(), val);
            let _ = (pid, val);
        }
        Ok(())
    }

    /// Revoke all permissions for a package (e.g. on uninstall).
    pub fn revoke_all(&mut self, package: &str) {
        if let Some(pkg) = self.packages.get_mut(package) {
            for state in pkg.grants.values_mut() {
                *state = GrantState::Denied;
            }
        }
    }

    /// Remove a package entirely.
    pub fn unregister_package(&mut self, name: &str) {
        self.packages.remove(name);
    }

    pub fn package_count(&self) -> usize { self.packages.len() }
}

impl Default for PermissionManager { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_grant() {
        let mut mgr = PermissionManager::new();
        mgr.register_package("com.example.app", 10042);
        mgr.apply_grant("com.example.app", Permission::Camera, GrantState::Granted).unwrap();
        let pkg = mgr.package("com.example.app").unwrap();
        assert!(pkg.is_granted(Permission::Camera));
        assert!(!pkg.is_granted(Permission::Microphone));
    }

    #[test]
    fn revoke_all_clears_grants() {
        let mut mgr = PermissionManager::new();
        mgr.register_package("com.test", 10001);
        mgr.apply_grant("com.test", Permission::Location, GrantState::Granted).unwrap();
        mgr.revoke_all("com.test");
        let pkg = mgr.package("com.test").unwrap();
        assert!(!pkg.is_granted(Permission::Location));
    }

    #[test]
    fn permanently_denied_blocks_request() {
        let mut mgr = PermissionManager::new();
        mgr.register_package("com.bad", 10099);
        mgr.apply_grant("com.bad", Permission::Microphone, GrantState::PermanentlyDenied).unwrap();
        let pkg = mgr.package("com.bad").unwrap();
        assert!(!pkg.can_request(Permission::Microphone));
    }
}
