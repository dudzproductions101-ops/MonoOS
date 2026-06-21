//! repo_manager.rs – OneOS Package Repository Manager
//!
//! Manages a list of package repositories (similar to apt sources.list).
//! Fetches index files, resolves packages, and hands download URLs to the
//! installer pipeline.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoProtocol { Https, Tor }

#[derive(Debug, Clone)]
pub struct Repository {
    pub id:        String,
    pub name:      String,
    pub base_url:  String,
    pub protocol:  RepoProtocol,
    pub enabled:   bool,
    pub signed:    bool,   // whether the index must be GPG-signed
    pub public_key: Option<String>,  // GPG public key fingerprint
}

impl Repository {
    pub fn new(id: impl Into<String>, name: impl Into<String>, url: impl Into<String>) -> Self {
        Repository {
            id: id.into(), name: name.into(), base_url: url.into(),
            protocol: RepoProtocol::Https, enabled: true, signed: true,
            public_key: None,
        }
    }

    pub fn index_url(&self) -> String { format!("{}/packages.json", self.base_url.trim_end_matches('/')) }
    pub fn package_url(&self, file: &str) -> String { format!("{}/{}", self.base_url.trim_end_matches('/'), file) }
}

#[derive(Debug, Clone)]
pub struct RemotePackage {
    pub package_name:  String,
    pub version_name:  String,
    pub version_code:  u32,
    pub download_url:  String,
    pub sha256:        String,
    pub size_bytes:    u64,
    pub repo_id:       String,
}

pub struct RepoManager {
    repos:    HashMap<String, Repository>,
    /// Cached index: package_name → list of available versions.
    index:    HashMap<String, Vec<RemotePackage>>,
    fetched:  bool,
}

impl RepoManager {
    pub fn new() -> Self {
        let mut mgr = RepoManager { repos: HashMap::new(), index: HashMap::new(), fetched: false };
        // Default repo: OneOS official.
        mgr.add_repo(Repository::new("oneos-official", "OneOS Official", "https://packages.oneos.io/stable"));
        mgr
    }

    pub fn add_repo(&mut self, repo: Repository) {
        self.repos.insert(repo.id.clone(), repo);
    }

    pub fn remove_repo(&mut self, id: &str) -> bool { self.repos.remove(id).is_some() }

    pub fn enable_repo(&mut self, id: &str, enabled: bool) -> bool {
        self.repos.get_mut(id).map(|r| { r.enabled = enabled; true }).unwrap_or(false)
    }

    /// Refresh index from all enabled repos. Returns error count.
    pub fn update_index(&mut self) -> u32 {
        let mut errors = 0u32;
        for repo in self.repos.values().filter(|r| r.enabled) {
            // Real: HTTP GET repo.index_url(), parse JSON, populate self.index.
            // Stub: no-op.
            let _ = repo.index_url();
        }
        self.fetched = true;
        errors
    }

    /// Find the latest version of a package across all repos.
    pub fn find_latest(&self, package_name: &str) -> Option<&RemotePackage> {
        self.index.get(package_name)
            .and_then(|vs| vs.iter().max_by_key(|p| p.version_code))
    }

    /// Search for packages whose name contains the query string.
    pub fn search(&self, query: &str) -> Vec<&RemotePackage> {
        self.index.values()
            .flat_map(|vs| vs.iter())
            .filter(|p| p.package_name.contains(query))
            .collect()
    }

    pub fn repos(&self) -> Vec<&Repository> { self.repos.values().collect() }
    pub fn repo_count(&self) -> usize { self.repos.len() }
    pub fn index_fetched(&self) -> bool { self.fetched }
}

impl Default for RepoManager { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn default_repo_present() {
        let mgr = RepoManager::new();
        assert_eq!(mgr.repo_count(), 1);
    }
    #[test] fn add_remove_repo() {
        let mut mgr = RepoManager::new();
        mgr.add_repo(Repository::new("test", "Test", "https://test.example.com"));
        assert_eq!(mgr.repo_count(), 2);
        assert!(mgr.remove_repo("test"));
        assert_eq!(mgr.repo_count(), 1);
    }
}
