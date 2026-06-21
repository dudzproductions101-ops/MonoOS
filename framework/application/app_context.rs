//! app_context.rs – MonoOS Application Context
//!
//! Every MonoOS application receives an AppContext on startup.  It is the
//! single gateway through which apps access system services, resources,
//! preferences, and permissions.  Modelled after Android's Context.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// A lightweight handle to a system service.
pub trait SystemService: Send + Sync {
    fn service_name(&self) -> &'static str;
}

/// Application metadata parsed from the package manifest.
#[derive(Debug, Clone)]
pub struct AppManifest {
    pub package_name:  String,
    pub version_name:  String,
    pub version_code:  u32,
    pub min_sdk:       u32,
    pub target_sdk:    u32,
    pub permissions:   Vec<String>,
    pub label:         String,
    pub icon_resource: String,
}

impl AppManifest {
    pub fn new(package_name: impl Into<String>) -> Self {
        AppManifest {
            package_name:  package_name.into(),
            version_name:  "1.0.0".into(),
            version_code:  1,
            min_sdk:       1,
            target_sdk:    1,
            permissions:   Vec::new(),
            label:         String::new(),
            icon_resource: String::new(),
        }
    }
}

/// Shared preferences — a simple persistent key-value store per app.
pub struct SharedPreferences {
    store: HashMap<String, String>,
    path:  PathBuf,
    dirty: bool,
}

impl SharedPreferences {
    pub fn open(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let store = Self::load(&path).unwrap_or_default();
        SharedPreferences { store, path, dirty: false }
    }

    fn load(path: &Path) -> Option<HashMap<String, String>> {
        let raw = std::fs::read_to_string(path).ok()?;
        let mut map = HashMap::new();
        for line in raw.lines() {
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.to_owned(), v.to_owned());
            }
        }
        Some(map)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.store.get(key).map(String::as_str)
    }

    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.store.get(key).map(|v| v == "true").unwrap_or(default)
    }

    pub fn get_i64(&self, key: &str, default: i64) -> i64 {
        self.store.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.store.insert(key.into(), value.into());
        self.dirty = true;
    }

    pub fn commit(&mut self) -> std::io::Result<()> {
        if !self.dirty { return Ok(()); }
        let contents: String = self.store.iter()
            .map(|(k, v)| format!("{k}={v}
"))
            .collect();
        std::fs::write(&self.path, contents)?;
        self.dirty = false;
        Ok(())
    }
}

/// The application context — passed to every Activity, Service, and Receiver.
pub struct AppContext {
    pub manifest:       AppManifest,
    pub uid:            u32,
    pub data_dir:       PathBuf,
    pub cache_dir:      PathBuf,
    pub files_dir:      PathBuf,
    services:           HashMap<&'static str, Arc<dyn SystemService>>,
    prefs_cache:        HashMap<String, Arc<Mutex<SharedPreferences>>>,
}

impl AppContext {
    pub fn new(manifest: AppManifest, uid: u32, data_root: impl AsRef<Path>) -> Self {
        let base = data_root.as_ref().join(&manifest.package_name);
        AppContext {
            data_dir:    base.clone(),
            cache_dir:   base.join("cache"),
            files_dir:   base.join("files"),
            manifest,
            uid,
            services:    HashMap::new(),
            prefs_cache: HashMap::new(),
        }
    }

    /// Bind a system service into this context (called by the system server).
    pub fn bind_service(&mut self, svc: Arc<dyn SystemService>) {
        self.services.insert(svc.service_name(), svc);
    }

    /// Retrieve a bound system service by name.
    pub fn get_service(&self, name: &str) -> Option<Arc<dyn SystemService>> {
        self.services.get(name).cloned()
    }

    /// Open (or retrieve cached) shared preferences for the given name.
    pub fn shared_prefs(&mut self, name: &str) -> Arc<Mutex<SharedPreferences>> {
        let path = self.data_dir.join("shared_prefs").join(format!("{name}.conf"));
        Arc::clone(
            self.prefs_cache
                .entry(name.to_owned())
                .or_insert_with(|| Arc::new(Mutex::new(SharedPreferences::open(&path))))
        )
    }

    pub fn package_name(&self) -> &str { &self.manifest.package_name }
    pub fn version_code(&self) -> u32  { self.manifest.version_code }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_reflects_package() {
        let manifest = AppManifest::new("com.example.myapp");
        let ctx = AppContext::new(manifest, 10001, "/data/data");
        assert!(ctx.data_dir.to_str().unwrap().contains("com.example.myapp"));
    }

    #[test]
    fn shared_prefs_set_get() {
        let manifest = AppManifest::new("com.test");
        let ctx_dir = std::env::temp_dir().join("monoos_test_prefs");
        std::fs::create_dir_all(&ctx_dir).ok();
        let mut ctx = AppContext::new(manifest, 10002, &ctx_dir);
        let prefs = ctx.shared_prefs("main");
        let mut p = prefs.lock().unwrap();
        p.set("first_run", "false");
        assert_eq!(p.get("first_run"), Some("false"));
    }
}
