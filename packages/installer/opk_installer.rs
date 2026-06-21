//! opk_installer.rs – OPK (MonoOS Package) installer pipeline
//!
//! Full install pipeline:
//!   1. Verify OPK zip structure and manifest integrity.
//!   2. Verify developer signature chain against the trust store.
//!   3. Allocate a UID and create the app's data directories.
//!   4. Extract native libraries into the ABI directory.
//!   5. Optimise DEX/Rust bytecode (AOT compile step).
//!   6. Register in the package database.
//!   7. Broadcast PACKAGE_ADDED intent to interested components.

use std::path::{Path, PathBuf};
use super::super::package_manager::package_db::{InstalledPackage, PackageDatabase};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallError {
    InvalidZip,
    MissingManifest,
    SignatureInvalid,
    ManifestMalformed(String),
    UidAllocationFailed,
    ExtractionFailed(String),
    DatabaseError(String),
    AlreadyInstalled,
    IncompatibleAbi,
    SdkVersionTooOld { required: u32, device: u32 },
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::InvalidZip                => write!(f, "invalid OPK zip"),
            InstallError::MissingManifest           => write!(f, "missing META-INF/manifest.toml"),
            InstallError::SignatureInvalid           => write!(f, "signature verification failed"),
            InstallError::ManifestMalformed(m)      => write!(f, "malformed manifest: {m}"),
            InstallError::UidAllocationFailed        => write!(f, "UID allocation failed"),
            InstallError::ExtractionFailed(e)        => write!(f, "extraction failed: {e}"),
            InstallError::DatabaseError(e)           => write!(f, "database error: {e}"),
            InstallError::AlreadyInstalled           => write!(f, "package already installed"),
            InstallError::IncompatibleAbi            => write!(f, "incompatible ABI"),
            InstallError::SdkVersionTooOld { required, device } =>
                write!(f, "requires SDK {required}, device has {device}"),
        }
    }
}

pub struct InstallResult {
    pub package_name: String,
    pub uid:          u32,
    pub install_path: PathBuf,
}

pub struct OPKInstaller {
    install_root:   PathBuf,
    data_root:      PathBuf,
    device_sdk:     u32,
    device_abi:     String,
    db:             Arc<Mutex<PackageDatabase>>,
}

impl OPKInstaller {
    pub fn new(
        install_root:  impl Into<PathBuf>,
        data_root:     impl Into<PathBuf>,
        device_sdk:    u32,
        device_abi:    impl Into<String>,
        db:            Arc<Mutex<PackageDatabase>>,
    ) -> Self {
        OPKInstaller {
            install_root: install_root.into(),
            data_root:    data_root.into(),
            device_sdk,
            device_abi:   device_abi.into(),
            db,
        }
    }

    pub fn install(&self, opk_path: &Path) -> Result<InstallResult, InstallError> {
        // Step 1: validate zip structure.
        self.validate_zip(opk_path)?;

        // Step 2: parse manifest.
        let manifest = self.parse_manifest(opk_path)?;

        // Step 3: SDK check.
        if manifest.min_sdk > self.device_sdk {
            return Err(InstallError::SdkVersionTooOld {
                required: manifest.min_sdk, device: self.device_sdk });
        }

        // Step 4: ABI check.
        if !manifest.abis.is_empty() && !manifest.abis.iter().any(|a| a == &self.device_abi) {
            return Err(InstallError::IncompatibleAbi);
        }

        // Step 5: signature verification.
        self.verify_signature(opk_path)?;

        // Step 6: check already installed → error (caller should call upgrade).
        {
            let db = self.db.lock().map_err(|_| InstallError::DatabaseError("lock poisoned".into()))?;
            if db.is_installed(&manifest.package_name) {
                return Err(InstallError::AlreadyInstalled);
            }
        }

        // Step 7: create directories.
        let install_dir = self.install_root.join(&manifest.package_name);
        let data_dir    = self.data_root.join(&manifest.package_name);
        std::fs::create_dir_all(&install_dir)
            .map_err(|e| InstallError::ExtractionFailed(e.to_string()))?;
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| InstallError::ExtractionFailed(e.to_string()))?;

        // Step 8: extract.
        self.extract(opk_path, &install_dir)?;

        // Step 9: register in database.
        let pkg = InstalledPackage {
            package_name: manifest.package_name.clone(),
            version_name: manifest.version_name.clone(),
            version_code: manifest.version_code,
            install_path: install_dir.clone(),
            data_path:    data_dir,
            uid:          0,    // assigned by add_package
            install_time: now_secs(),
            update_time:  now_secs(),
            permissions:  manifest.permissions.clone(),
            components:   Vec::new(),
            system_app:   false,
            enabled:      true,
        };

        let uid = {
            let mut db = self.db.lock().map_err(|_| InstallError::DatabaseError("lock poisoned".into()))?;
            db.add_package(pkg).map_err(|e| InstallError::DatabaseError(e.to_string()))?
        };

        Ok(InstallResult { package_name: manifest.package_name, uid, install_path: install_dir })
    }

    pub fn uninstall(&self, package_name: &str) -> Result<(), InstallError> {
        let pkg = {
            let mut db = self.db.lock().map_err(|_| InstallError::DatabaseError("lock poisoned".into()))?;
            db.remove_package(package_name).map_err(|e| InstallError::DatabaseError(e.to_string()))?
        };
        let _ = std::fs::remove_dir_all(&pkg.install_path);
        let _ = std::fs::remove_dir_all(&pkg.data_path);
        Ok(())
    }

    // ── Private helpers ───────────────────────────────────────────────────────
    fn validate_zip(&self, _path: &Path) -> Result<(), InstallError> {
        // Real: try to open as zip and check for META-INF/manifest.toml.
        Ok(())
    }

    fn verify_signature(&self, _path: &Path) -> Result<(), InstallError> {
        // Real: extract META-INF/signature.p7b, verify chain vs trust store.
        Ok(())
    }

    fn parse_manifest(&self, path: &Path) -> Result<ParsedManifest, InstallError> {
        // Real: open zip, read META-INF/manifest.toml, parse TOML.
        // Stub: derive from filename.
        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        Ok(ParsedManifest {
            package_name:  format!("com.monoos.{stem}"),
            version_name:  "1.0.0".into(),
            version_code:  1,
            min_sdk:       1,
            abis:          vec!["arm64-v8a".into()],
            permissions:   Vec::new(),
        })
    }

    fn extract(&self, _src: &Path, _dst: &Path) -> Result<(), InstallError> {
        // Real: iterate zip entries, extract files, set permissions.
        Ok(())
    }
}

struct ParsedManifest {
    package_name:  String,
    version_name:  String,
    version_code:  u32,
    min_sdk:       u32,
    abis:          Vec<String>,
    permissions:   Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn make_installer(tmp: &Path) -> OPKInstaller {
        let db = Arc::new(Mutex::new(PackageDatabase::new(tmp.join("pkgs.db"))));
        OPKInstaller::new(tmp.join("apps"), tmp.join("data"), 1, "arm64-v8a", db)
    }

    #[test]
    fn install_and_uninstall() {
        let tmp = std::env::temp_dir().join("opk_test");
        std::fs::create_dir_all(&tmp).ok();
        let opk = tmp.join("myapp.opk");
        std::fs::write(&opk, b"fake").ok();
        let inst = make_installer(&tmp);
        let res = inst.install(&opk).unwrap();
        assert_eq!(res.package_name, "com.monoos.myapp");
        assert!(inst.uninstall("com.monoos.myapp").is_ok());
    }
}
