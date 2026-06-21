//! camera_monitor.rs – Real-time camera access monitor
//!
//! Polls /proc/monoos/fs_events for camera device opens, maintains a
//! per-package camera usage log, and triggers a status-bar indicator.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CameraSession {
    pub package:    String,
    pub uid:        u32,
    pub started_ms: u64,
    pub ended_ms:   Option<u64>,
}

impl CameraSession {
    pub fn duration_ms(&self, now_ms: u64) -> u64 {
        self.ended_ms.unwrap_or(now_ms).saturating_sub(self.started_ms)
    }
    pub fn is_active(&self) -> bool { self.ended_ms.is_none() }
}

pub struct CameraMonitor {
    sessions:       Vec<CameraSession>,
    totals_ms:      HashMap<String, u64>,
    indicator_on:   bool,
}

impl CameraMonitor {
    pub fn new() -> Self {
        CameraMonitor { sessions: Vec::new(), totals_ms: HashMap::new(), indicator_on: false }
    }

    pub fn on_camera_open(&mut self, package: impl Into<String>, uid: u32, ts_ms: u64) {
        let pkg = package.into();
        self.sessions.push(CameraSession { package: pkg, uid, started_ms: ts_ms, ended_ms: None });
        self.indicator_on = true;
    }

    pub fn on_camera_close(&mut self, package: &str, ts_ms: u64) {
        for sess in self.sessions.iter_mut().rev() {
            if sess.package == package && sess.is_active() {
                sess.ended_ms = Some(ts_ms);
                let dur = sess.duration_ms(ts_ms);
                *self.totals_ms.entry(package.to_owned()).or_insert(0) += dur;
                break;
            }
        }
        self.indicator_on = self.sessions.iter().any(|s| s.is_active());
    }

    pub fn is_camera_in_use(&self) -> bool { self.indicator_on }
    pub fn active_sessions(&self) -> Vec<&CameraSession> {
        self.sessions.iter().filter(|s| s.is_active()).collect()
    }
    pub fn total_usage_ms(&self, package: &str) -> u64 {
        *self.totals_ms.get(package).unwrap_or(&0)
    }
}

impl Default for CameraMonitor { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn open_close_cycle() {
        let mut m = CameraMonitor::new();
        m.on_camera_open("com.camera", 10001, 1000);
        assert!(m.is_camera_in_use());
        assert_eq!(m.active_sessions().len(), 1);
        m.on_camera_close("com.camera", 2500);
        assert!(!m.is_camera_in_use());
        assert_eq!(m.total_usage_ms("com.camera"), 1500);
    }
}
