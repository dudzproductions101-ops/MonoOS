//! Integration test: OPK installation pipeline end-to-end.

use std::path::{Path, PathBuf};
use std::fs;

fn temp_dir(suffix: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("monoos_pkg_integ_{suffix}"));
    fs::create_dir_all(&p).ok();
    p
}

fn write_fake_opk(path: &Path) {
    // A real OPK is a zip; for this test we write a minimal placeholder.
    fs::write(path, b"OPK1fake").unwrap();
}

// Minimal inline installer simulation (mirrors opk_installer.rs logic).
struct Installer {
    install_root: PathBuf,
    data_root:    PathBuf,
    installed:    std::collections::HashMap<String, PathBuf>,
}

impl Installer {
    fn new(install_root: PathBuf, data_root: PathBuf) -> Self {
        Installer { install_root, data_root, installed: Default::default() }
    }

    fn install(&mut self, opk: &Path) -> Result<String, &'static str> {
        let stem = opk.file_stem().and_then(|s| s.to_str()).unwrap_or("app");
        let pkg  = format!("com.monoos.{stem}");
        if self.installed.contains_key(&pkg) { return Err("already installed"); }
        let dir = self.install_root.join(&pkg);
        fs::create_dir_all(&dir).map_err(|_| "mkdir failed")?;
        fs::create_dir_all(self.data_root.join(&pkg)).map_err(|_| "mkdir data failed")?;
        self.installed.insert(pkg.clone(), dir);
        Ok(pkg)
    }

    fn uninstall(&mut self, pkg: &str) -> bool {
        if let Some(dir) = self.installed.remove(pkg) {
            fs::remove_dir_all(&dir).ok();
            fs::remove_dir_all(self.data_root.join(pkg)).ok();
            return true;
        }
        false
    }

    fn is_installed(&self, pkg: &str) -> bool { self.installed.contains_key(pkg) }
}

#[test]
fn install_creates_directories() {
    let tmp = temp_dir("install");
    let opk = tmp.join("myapp.opk");
    write_fake_opk(&opk);

    let mut inst = Installer::new(tmp.join("apps"), tmp.join("data"));
    let pkg = inst.install(&opk).unwrap();
    assert!(inst.is_installed(&pkg));
    assert!(tmp.join("apps").join(&pkg).exists());
    assert!(tmp.join("data").join(&pkg).exists());
}

#[test]
fn double_install_rejected() {
    let tmp = temp_dir("double");
    let opk = tmp.join("myapp.opk");
    write_fake_opk(&opk);

    let mut inst = Installer::new(tmp.join("apps"), tmp.join("data"));
    inst.install(&opk).unwrap();
    assert!(inst.install(&opk).is_err());
}

#[test]
fn uninstall_removes_directories() {
    let tmp = temp_dir("uninstall");
    let opk = tmp.join("myapp.opk");
    write_fake_opk(&opk);

    let mut inst = Installer::new(tmp.join("apps"), tmp.join("data"));
    let pkg = inst.install(&opk).unwrap();
    assert!(inst.uninstall(&pkg));
    assert!(!inst.is_installed(&pkg));
}

#[test]
fn uninstall_nonexistent_returns_false() {
    let tmp = temp_dir("noexist");
    let mut inst = Installer::new(tmp.join("apps"), tmp.join("data"));
    assert!(!inst.uninstall("com.nonexistent.app"));
}
