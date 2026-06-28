//! permission_manager.rs – MonoOS Framework Permission Manager
//!
//! The framework-layer permission manager is the bridge between application
//! code and the kernel-level permission syscalls.  Apps call into this
//! via the MonoOS SDK; it validates the request, persists the user's
//! decision, and calls into `KernelBridge` to update the live kernel table.

use std::collections::HashMap;
use std::sync::Arc;

use super::kernel_bridge::KernelBridge;

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

// ─────────────────────────────────────────────────────────────────────────────
//  PermissionManager
// ─────────────────────────────────────────────────────────────────────────────

/// The central permission database, owned by the permission_service.
///
/// Holds an optional `Arc<KernelBridge>`.  When the bridge is present, every
/// grant and revoke is immediately mirrored into the kernel's live process
/// permission table so the LSM kretprobes see real data.  When absent (e.g.
/// running unit tests on a host without `/dev/monoos`) the manager falls back
/// to in-memory-only operation and logs a warning.
pub struct PermissionManager {
    packages: HashMap<String, PackagePermissions>,
    /// Live connection to /dev/monoos.  None until attach_kernel_bridge() is called.
    bridge: Option<Arc<KernelBridge>>,
}

impl PermissionManager {
    pub fn new() -> Self {
        PermissionManager {
            packages: HashMap::new(),
            bridge:   None,
        }
    }

    /// Attach an open `KernelBridge`.  Called by the permission_service
    /// immediately after opening `/dev/monoos`.  Must be called before any
    /// grant/revoke calls to ensure kernel enforcement is live.
    pub fn attach_kernel_bridge(&mut self, bridge: Arc<KernelBridge>) {
        self.bridge = Some(bridge);
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

    /// Apply a user decision, persist it in-memory, and propagate it to the kernel.
    ///
    /// This is the method that makes LSM enforcement live: after this call,
    /// `monoos_proc_has_perm(pid, bit)` in the kernel will return the correct
    /// answer and `/proc/monoos/lsm_audit` will start logging decisions.
    ///
    /// # PID vs UID
    /// The kernel table is keyed by PID.  We use the package UID as a proxy
    /// here.  `permission_service` overrides this for individual process grants
    /// by calling `KernelBridge::grant()` / `KernelBridge::revoke()` directly
    /// with the real PID.
    pub fn apply_grant(
        &mut self,
        package: &str,
        perm: Permission,
        state: GrantState,
    ) -> Result<(), String> {
        let pkg = self.packages.get_mut(package)
            .ok_or_else(|| format!("package not found: {package}"))?;

        pkg.grants.insert(perm, state);

        // ── Kernel bridge call ───────────────────────────────────────────────
        // pid proxy: use the package UID.  The permission_service will call the
        // bridge directly for per-process grants using the real PID.
        let pid = pkg.uid as i32;
        let bit = perm.kernel_bit();

        match &self.bridge {
            Some(bridge) => {
                let result = if state == GrantState::Granted {
                    bridge.grant(pid, bit)
                } else {
                    bridge.revoke(pid, bit)
                };
                if let Err(e) = result {
                    // Log the error but don't propagate — the in-memory grant
                    // succeeded.  The kernel may lag until the device is
                    // available (e.g. during early boot).
                    eprintln!(
                        "[permission_manager] kernel bridge error for {package}/{}: {e}",
                        perm.display_name()
                    );
                }
            }
            None => {
                // No bridge: in-memory only.  Normal during unit testing.
                eprintln!(
                    "[permission_manager] no kernel bridge — grant for {package}/{} is in-memory only",
                    perm.display_name()
                );
            }
        }

        Ok(())
    }

    /// Revoke all permissions for a package (e.g. on uninstall).
    ///
    /// Mirrors each revocation into the kernel table if a bridge is attached.
    pub fn revoke_all(&mut self, package: &str) {
        let pkg = match self.packages.get_mut(package) {
            Some(p) => p,
            None    => return,
        };

        // Collect (perm, uid) before the mutable borrow on pkg ends.
        let to_revoke: Vec<(Permission, u32)> = pkg.grants
            .iter()
            .filter(|(_, state)| **state == GrantState::Granted)
            .map(|(perm, _)| (*perm, pkg.uid))
            .collect();

        // Mark all as denied in the in-memory table.
        for state in pkg.grants.values_mut() {
            *state = GrantState::Denied;
        }

        // Propagate to kernel for every previously-granted permission.
        if let Some(bridge) = &self.bridge {
            for (perm, uid) in to_revoke {
                if let Err(e) = bridge.revoke(uid as i32, perm.kernel_bit()) {
                    eprintln!(
                        "[permission_manager] kernel revoke failed for {package}/{}: {e}",
                        perm.display_name()
                    );
                }
            }
        }
    }

    /// Remove a package entirely (also revokes all in the kernel).
    pub fn unregister_package(&mut self, name: &str) {
        self.revoke_all(name);
        self.packages.remove(name);
    }

    pub fn package_count(&self) -> usize { self.packages.len() }

    pub fn has_bridge(&self) -> bool { self.bridge.is_some() }
}

impl Default for PermissionManager { fn default() -> Self { Self::new() } }

// ─────────────────────────────────────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────────────────────────────────────

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

    #[test]
    fn apply_grant_unknown_package_returns_error() {
        let mut mgr = PermissionManager::new();
        let err = mgr.apply_grant("com.ghost", Permission::Camera, GrantState::Granted);
        assert!(err.is_err());
    }

    #[test]
    fn unregister_package_removes_it() {
        let mut mgr = PermissionManager::new();
        mgr.register_package("com.gone", 10005);
        mgr.unregister_package("com.gone");
        assert!(mgr.package("com.gone").is_none());
        assert_eq!(mgr.package_count(), 0);
    }

    #[test]
    fn no_bridge_by_default() {
        let mgr = PermissionManager::new();
        assert!(!mgr.has_bridge());
    }
}
