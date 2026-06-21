//! privacy_dashboard.rs – MonoOS Privacy Dashboard
//!
//! Aggregates real-time hardware access events from the kernel's
//! /proc/monoos/ ring buffers and presents a consolidated view.

use std::collections::{HashMap, VecDeque};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HardwareResource { Camera, Microphone, Location, Network, Storage, Nfc, Bluetooth }

impl HardwareResource {
    pub fn display_name(self) -> &'static str {
        match self {
            HardwareResource::Camera    => "Camera",
            HardwareResource::Microphone => "Microphone",
            HardwareResource::Location  => "Location",
            HardwareResource::Network   => "Network",
            HardwareResource::Storage   => "Storage",
            HardwareResource::Nfc       => "NFC",
            HardwareResource::Bluetooth => "Bluetooth",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccessEvent {
    pub timestamp_ms: u64,
    pub package_name: String,
    pub uid:          u32,
    pub resource:     HardwareResource,
    pub allowed:      bool,
}

/// 7-day sliding-window access log per resource.
const MAX_EVENTS_PER_RESOURCE: usize = 1000;

pub struct PrivacyDashboard {
    log:     HashMap<HardwareResource, VecDeque<AccessEvent>>,
    /// Map uid → package name (populated from the package service).
    uid_map: HashMap<u32, String>,
}

impl PrivacyDashboard {
    pub fn new() -> Self {
        let mut log = HashMap::new();
        for res in [
            HardwareResource::Camera, HardwareResource::Microphone,
            HardwareResource::Location, HardwareResource::Network,
            HardwareResource::Storage, HardwareResource::Nfc,
            HardwareResource::Bluetooth,
        ] {
            log.insert(res, VecDeque::new());
        }
        PrivacyDashboard { log, uid_map: HashMap::new() }
    }

    pub fn register_uid(&mut self, uid: u32, package: impl Into<String>) {
        self.uid_map.insert(uid, package.into());
    }

    pub fn record(&mut self, event: AccessEvent) {
        let queue = self.log.entry(event.resource).or_default();
        if queue.len() >= MAX_EVENTS_PER_RESOURCE { queue.pop_front(); }
        queue.push_back(event);
    }

    /// Return the most recent events for a resource.
    pub fn recent(&self, resource: HardwareResource, limit: usize) -> Vec<&AccessEvent> {
        self.log.get(&resource)
            .map(|q| q.iter().rev().take(limit).collect())
            .unwrap_or_default()
    }

    /// Which packages accessed a resource in the last `window_secs` seconds?
    pub fn active_users(&self, resource: HardwareResource, window_secs: u64) -> Vec<&str> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let cutoff = now.saturating_sub(window_secs * 1000);
        let mut pkgs: Vec<&str> = self.log.get(&resource)
            .into_iter().flatten()
            .filter(|e| e.timestamp_ms >= cutoff && e.allowed)
            .map(|e| e.package_name.as_str())
            .collect();
        pkgs.dedup();
        pkgs
    }

    /// Aggregated count of blocked accesses per package.
    pub fn blocked_counts(&self) -> HashMap<&str, u32> {
        let mut counts: HashMap<&str, u32> = HashMap::new();
        for queue in self.log.values() {
            for ev in queue {
                if !ev.allowed {
                    *counts.entry(ev.package_name.as_str()).or_insert(0) += 1;
                }
            }
        }
        counts
    }

    pub fn total_events(&self) -> usize {
        self.log.values().map(|q| q.len()).sum()
    }
}

impl Default for PrivacyDashboard { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(pkg: &str, res: HardwareResource, allowed: bool) -> AccessEvent {
        AccessEvent { timestamp_ms: 1_000_000, package_name: pkg.into(),
                      uid: 10001, resource: res, allowed }
    }

    #[test]
    fn record_and_recent() {
        let mut dash = PrivacyDashboard::new();
        dash.record(ev("com.camera", HardwareResource::Camera, true));
        dash.record(ev("com.camera", HardwareResource::Camera, true));
        let r = dash.recent(HardwareResource::Camera, 10);
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn blocked_counts() {
        let mut dash = PrivacyDashboard::new();
        dash.record(ev("com.spy", HardwareResource::Microphone, false));
        dash.record(ev("com.spy", HardwareResource::Camera, false));
        let counts = dash.blocked_counts();
        assert_eq!(counts.get("com.spy"), Some(&2));
    }
}
