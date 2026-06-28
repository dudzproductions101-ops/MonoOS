//! monoos_packages – MonoOS package manager: OPK install pipeline, package
//! database, repository sync, and signature verification.
//!
//! Module files live in their existing subdirectories
//! (`installer/opk_installer.rs`, `package_manager/package_db.rs`, etc.)
//! rather than the Rust-conventional `installer/mod.rs` layout, so each
//! top-level module below is wired in via an explicit `#[path]` attribute
//! to avoid moving/renaming any existing source file.

pub mod installer {
    #[path = "../installer/opk_installer.rs"]
    pub mod opk_installer;
}

pub mod package_manager {
    #[path = "../package_manager/package_db.rs"]
    pub mod package_db;
}

pub mod repositories {
    #[path = "../repositories/repo_manager.rs"]
    pub mod repo_manager;
}

pub mod signatures {
    #[path = "../signatures/signature_verifier.rs"]
    pub mod signature_verifier;
}
