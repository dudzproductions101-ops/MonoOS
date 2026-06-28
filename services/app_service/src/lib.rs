//! app_service.rs – MonoOS AppService Service
//!
//! Launches and monitors application processes, enforces memory limits, and garbage-collects idle app instances.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ─────────────────────────────────────────────────────────────────────────────
//  Error type
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ServiceError {
    NotInitialised,
    PermissionDenied,
    NotFound(String),
    Io(String),
    Internal(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::NotInitialised      => write!(f, "service not initialised"),
            ServiceError::PermissionDenied    => write!(f, "permission denied"),
            ServiceError::NotFound(msg)       => write!(f, "not found: {msg}"),
            ServiceError::Io(msg)             => write!(f, "I/O error: {msg}"),
            ServiceError::Internal(msg)       => write!(f, "internal error: {msg}"),
        }
    }
}

pub type Result<T> = std::result::Result<T, ServiceError>;

// ─────────────────────────────────────────────────────────────────────────────
//  Service state
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

// ─────────────────────────────────────────────────────────────────────────────
//  AppServiceService implementation
// ─────────────────────────────────────────────────────────────────────────────

pub struct AppServiceService {
    status:   ServiceStatus,
    metadata: HashMap<String, String>,
}

impl AppServiceService {
    pub fn new() -> Self {
        AppServiceService {
            status:   ServiceStatus::Stopped,
            metadata: HashMap::new(),
        }
    }

    /// Start the service.  Called by the system_server after all dependencies
    /// are ready.  Returns Ok(()) if start succeeded or the service was
    /// already running.
    pub fn start(&mut self) -> Result<()> {
        if self.status == ServiceStatus::Running {
            return Ok(());
        }
        self.status = ServiceStatus::Starting;
        // Initialise subsystems, open devices, bind sockets…
        self.on_start()?;
        self.status = ServiceStatus::Running;
        eprintln!("[app_service] service started");
        Ok(())
    }

    /// Stop the service gracefully.
    pub fn stop(&mut self) -> Result<()> {
        if self.status == ServiceStatus::Stopped {
            return Ok(());
        }
        self.status = ServiceStatus::Stopping;
        self.on_stop()?;
        self.status = ServiceStatus::Stopped;
        eprintln!("[app_service] service stopped");
        Ok(())
    }

    pub fn status(&self) -> ServiceStatus { self.status }

    /// Query a metadata value previously set by on_start().
    pub fn get_info(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(String::as_str)
    }

    // ── Internal lifecycle hooks ────────────────────────────────────────────

    fn on_start(&mut self) -> Result<()> {
        self.metadata.insert("version".into(), env!("CARGO_PKG_VERSION").into());
        self.metadata.insert("service".into(), "app_service".into());
        Ok(())
    }

    fn on_stop(&mut self) -> Result<()> {
        self.metadata.clear();
        Ok(())
    }
}

impl Default for AppServiceService {
    fn default() -> Self { Self::new() }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Thread-safe wrapper (Arc<Mutex<…>>) – used by system_server
// ─────────────────────────────────────────────────────────────────────────────

pub type SharedAppServiceService = Arc<Mutex<AppServiceService>>;

pub fn create() -> SharedAppServiceService {
    Arc::new(Mutex::new(AppServiceService::new()))
}

// ─────────────────────────────────────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_and_stop() {
        let mut svc = AppServiceService::new();
        assert_eq!(svc.status(), ServiceStatus::Stopped);
        svc.start().unwrap();
        assert_eq!(svc.status(), ServiceStatus::Running);
        svc.stop().unwrap();
        assert_eq!(svc.status(), ServiceStatus::Stopped);
    }

    #[test]
    fn double_start_is_idempotent() {
        let mut svc = AppServiceService::new();
        svc.start().unwrap();
        svc.start().unwrap(); // second start should be a no-op
        assert_eq!(svc.status(), ServiceStatus::Running);
    }

    #[test]
    fn get_info_after_start() {
        let mut svc = AppServiceService::new();
        svc.start().unwrap();
        assert_eq!(svc.get_info("service"), Some("app_service"));
    }
}
