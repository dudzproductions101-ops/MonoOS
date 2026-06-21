//! package_db.rs – MonoOS Package Database
//!
//! Persistent SQLite-backed store of all installed packages, their
//! manifests, granted permissions, and component registrations
//! (activities, services, broadcast receivers, content providers).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentKind { Activity, Service, Receiver, Provider }

#[derive(Debug, Clone)]
pub struct ComponentInfo {
    pub name:     String,
    pub kind:     ComponentKind,
    pub exported: bool,
    pub enabled:  bool,
}

#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub package_name:   String,
    pub version_name:   String,
    pub version_code:   u32,
    pub install_path:   PathBuf,
    pub data_path:      PathBuf,
    pub uid:            u32,
    pub install_time:   u64,    // Unix seconds
    pub update_time:    u64,
    pub permissions:    Vec<String>,
    pub components:     Vec<ComponentInfo>,
    pub system_app:     bool,
    pub enabled:        bool,
}

impl InstalledPackage {
    pub fn find_component(&self, name: &str) -> Option<&ComponentInfo> {
        self.components.iter().find(|c| c.name == name)
    }
}

pub struct PackageDatabase {
    packages:  HashMap<String, InstalledPackage>,
    uid_map:   HashMap<u32, String>,   // uid → package_name
    next_uid:  u32,
    db_path:   PathBuf,
}

impl PackageDatabase {
    pub fn new(db_path: impl Into<PathBuf>) -> Self {
        PackageDatabase {
            packages:  HashMap::new(),
            uid_map:   HashMap::new(),
            next_uid:  10000,   // app UIDs start at 10000
            db_path:   db_path.into(),
        }
    }

    pub fn load(&mut self) -> Result<(), &'static str> {
        // Real impl: open SQLite file at self.db_path and populate maps.
        // Stub: start empty.
        Ok(())
    }

    pub fn save(&self) -> Result<(), &'static str> {
        // Real impl: serialize to SQLite.
        Ok(())
    }

    /// Path to the on-disk database file (used by [`load`]/[`save`] once
    /// SQLite persistence is implemented).
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn add_package(&mut self, mut pkg: InstalledPackage) -> Result<u32, &'static str> {
        if self.packages.contains_key(&pkg.package_name) {
            return Err("package already installed; call update_package instead");
        }
        let uid = self.next_uid;
        self.next_uid += 1;
        pkg.uid = uid;
        self.uid_map.insert(uid, pkg.package_name.clone());
        self.packages.insert(pkg.package_name.clone(), pkg);
        self.save()?;
        Ok(uid)
    }

    pub fn update_package(&mut self, pkg: InstalledPackage) -> Result<(), &'static str> {
        let existing = self.packages.get_mut(&pkg.package_name)
            .ok_or("package not found")?;
        let uid = existing.uid;
        *existing = pkg;
        existing.uid = uid;     // preserve uid across updates
        self.save()
    }

    pub fn remove_package(&mut self, name: &str) -> Result<InstalledPackage, &'static str> {
        let pkg = self.packages.remove(name).ok_or("package not found")?;
        self.uid_map.remove(&pkg.uid);
        self.save()?;
        Ok(pkg)
    }

    pub fn get(&self, name: &str) -> Option<&InstalledPackage> {
        self.packages.get(name)
    }

    pub fn get_by_uid(&self, uid: u32) -> Option<&InstalledPackage> {
        let name = self.uid_map.get(&uid)?;
        self.packages.get(name)
    }

    pub fn all_packages(&self) -> Vec<&InstalledPackage> {
        self.packages.values().collect()
    }

    pub fn find_by_component(&self, kind: &ComponentKind, name: &str) -> Vec<&InstalledPackage> {
        self.packages.values()
            .filter(|p| p.components.iter().any(|c| &c.kind == kind && c.name == name && c.exported))
            .collect()
    }

    pub fn is_installed(&self, name: &str) -> bool { self.packages.contains_key(name) }
    pub fn count(&self) -> usize { self.packages.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> InstalledPackage {
        InstalledPackage {
            package_name: "com.example.app".into(), version_name: "1.0".into(),
            version_code: 1, install_path: PathBuf::from("/data/app/com.example.app"),
            data_path: PathBuf::from("/data/data/com.example.app"),
            uid: 0, install_time: 0, update_time: 0,
            permissions: vec!["CAMERA".into()],
            components: vec![ComponentInfo { name: "MainActivity".into(), kind: ComponentKind::Activity, exported: true, enabled: true }],
            system_app: false, enabled: true,
        }
    }

    #[test] fn add_and_get() {
        let mut db = PackageDatabase::new("/tmp/test.db");
        let uid = db.add_package(sample()).unwrap();
        assert!(uid >= 10000);
        assert!(db.is_installed("com.example.app"));
        assert!(db.get_by_uid(uid).is_some());
    }

    #[test] fn remove() {
        let mut db = PackageDatabase::new("/tmp/test.db");
        db.add_package(sample()).unwrap();
        let pkg = db.remove_package("com.example.app").unwrap();
        assert_eq!(pkg.package_name, "com.example.app");
        assert!(!db.is_installed("com.example.app"));
    }

    #[test] fn find_by_activity() {
        let mut db = PackageDatabase::new("/tmp/test.db");
        db.add_package(sample()).unwrap();
        let results = db.find_by_component(&ComponentKind::Activity, "MainActivity");
        assert_eq!(results.len(), 1);
    }
}
