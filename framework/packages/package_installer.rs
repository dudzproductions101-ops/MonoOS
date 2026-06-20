//! package_installer.rs – OneOS Framework Package Installer
//!
//! Validates and installs OPK (OneOS Package) files.  An OPK is a
//! ZIP archive containing:
//!   META-INF/manifest.toml   – package metadata
//!   META-INF/signature.p7b   – PKCS#7 signature over the manifest
//!   lib/<abi>/*.so           – native libraries
//!   res/                     – resources (icons, QML, assets)
//!   bin/<executable>         – optional native binary
//!   data/                    – initial data files

use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Minimum and declared SDK versions from the manifest.
#[derive(Debug, Clone)]
pub struct SdkRequirement {
    pub min_sdk:    u32,
    pub target_sdk: u32,
}

/// Supported CPU ABIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abi { Arm64V8a, ArmV7a, X86_64, X86 }

impl Abi {
    pub fn dir_name(self) -> &'static str {
        match self {
            Abi::Arm64V8a => "arm64-v8a",
            Abi::ArmV7a   => "armeabi-v7a",
            Abi::X86_64   => "x86_64",
            Abi::X86      => "x86",
        }
    }
}

/// Package manifest parsed from META-INF/manifest.toml.
#[derive(Debug, Clone)]
pub struct PackageManifest {
    pub package_name:  String,
    pub version_name:  String,
    pub version_code:  u32,
    pub sdk:           SdkRequirement,
    pub label:         String,
    pub permissions:   Vec<String>,
    pub abis:          Vec<Abi>,
    pub entry_binary:  Option<String>,
}

/// Installation result.
#[derive(Debug, Clone)]
pub enum InstallResult {
    Success { install_path: PathBuf },
    AlreadyInstalled,
    SignatureInvalid,
    IncompatibleAbi,
    SdkTooOld { required: u32, device: u32 },
    StorageInsufficient { required_kb: u64, available_kb: u64 },
    ManifestMalformed(String),
    IoError(String),
}

impl InstallResult {
    pub fn is_success(&self) -> bool { matches!(self, InstallResult::Success { .. }) }
}

/// Uninstall result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UninstallResult { Success, NotFound, SystemApp }

/// The package installer.
pub struct PackageInstaller {
    /// Map of installed packages: name → install dir.
    installed:    HashMap<String, PathBuf>,
    install_root: PathBuf,
    device_sdk:   u32,
    device_abi:   Abi,
}

impl PackageInstaller {
    pub fn new(install_root: impl Into<PathBuf>, device_sdk: u32, device_abi: Abi) -> Self {
        PackageInstaller {
            installed: HashMap::new(),
            install_root: install_root.into(),
            device_sdk,
            device_abi,
        }
    }

    /// Install a package from an OPK file path.
    pub fn install(&mut self, opk_path: &Path) -> InstallResult {
        // Step 1: parse manifest (stub: derive from filename).
        let manifest = match self.parse_manifest(opk_path) {
            Ok(m)  => m,
            Err(e) => return InstallResult::ManifestMalformed(e),
        };

        // Step 2: check SDK compatibility.
        if manifest.sdk.min_sdk > self.device_sdk {
            return InstallResult::SdkTooOld {
                required: manifest.sdk.min_sdk,
                device:   self.device_sdk,
            };
        }

        // Step 3: check ABI.
        if !manifest.abis.is_empty() && !manifest.abis.contains(&self.device_abi) {
            return InstallResult::IncompatibleAbi;
        }

        // Step 4: check if already installed (upgrade path).
        if self.installed.contains_key(&manifest.package_name) {
            // Upgrade: remove old version first.
            self.uninstall(&manifest.package_name);
        }

        // Step 5: verify signature.
        if !self.verify_signature(opk_path) {
            return InstallResult::SignatureInvalid;
        }

        // Step 6: extract to install directory.
        let install_dir = self.install_root.join(&manifest.package_name);
        if let Err(e) = self.extract(opk_path, &install_dir) {
            return InstallResult::IoError(e);
        }

        // Step 7: record installation.
        self.installed.insert(manifest.package_name.clone(), install_dir.clone());

        InstallResult::Success { install_path: install_dir }
    }

    pub fn uninstall(&mut self, package: &str) -> UninstallResult {
        match self.installed.remove(package) {
            None => UninstallResult::NotFound,
            Some(dir) => {
                let _ = std::fs::remove_dir_all(&dir);
                UninstallResult::Success
            }
        }
    }

    pub fn is_installed(&self, package: &str) -> bool {
        self.installed.contains_key(package)
    }

    pub fn install_path(&self, package: &str) -> Option<&Path> {
        self.installed.get(package).map(PathBuf::as_path)
    }

    pub fn installed_packages(&self) -> Vec<&str> {
        self.installed.keys().map(String::as_str).collect()
    }

    // ── Private helpers ────────────────────────────────────────────────────────

    fn parse_manifest(&self, opk_path: &Path) -> Result<PackageManifest, String> {
        // Real impl: open ZIP, read META-INF/manifest.toml, parse TOML.
        // Stub: derive name from filename.
        let stem = opk_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        Ok(PackageManifest {
            package_name:  format!("com.oneos.{stem}"),
            version_name:  "1.0.0".into(),
            version_code:  1,
            sdk: SdkRequirement { min_sdk: 1, target_sdk: 1 },
            label:         stem.to_owned(),
            permissions:   Vec::new(),
            abis:          vec![Abi::Arm64V8a],
            entry_binary:  None,
        })
    }

    fn verify_signature(&self, _opk_path: &Path) -> bool {
        // Real impl: extract META-INF/signature.p7b and verify with trust store.
        true
    }

    fn extract(&self, _opk_path: &Path, install_dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(install_dir)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_and_uninstall() {
        let tmp = std::env::temp_dir().join("oneos_pkg_test");
        let mut inst = PackageInstaller::new(&tmp, 1, Abi::Arm64V8a);
        let opk = tmp.join("myapp.opk");
        std::fs::create_dir_all(&tmp).ok();
        std::fs::write(&opk, b"fake opk").ok();
        let res = inst.install(&opk);
        assert!(res.is_success(), "{res:?}");
        assert!(inst.is_installed("com.oneos.myapp"));
        assert_eq!(inst.uninstall("com.oneos.myapp"), UninstallResult::Success);
        assert!(!inst.is_installed("com.oneos.myapp"));
    }
}
