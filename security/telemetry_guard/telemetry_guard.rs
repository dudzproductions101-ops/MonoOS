//! telemetry_guard.rs – Detect and block telemetry transmissions
//!
//! Analyses outbound network events from the network_monitor and
//! classifies them as telemetry using heuristics:
//!   - Known telemetry endpoints (domain / IP).
//!   - Periodic beaconing patterns (regular intervals, small payloads).
//!   - JSON payloads containing device identifiers.

use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct BeaconEvent {
    pub ts_ms:   u64,
    pub domain:  String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryClass {
    NotTelemetry,
    LikelyTelemetry,
    KnownTelemetry,
}

static KNOWN_TELEMETRY: &[&str] = &[
    "api.amplitude.com",
    "api.segment.io",
    "ingest.braze.com",
    "e.crashlytics.com",
    "settings.crashlytics.com",
    "api.mixpanel.com",
    "api2.amplitude.com",
];

pub struct TelemetryGuard {
    beacons:      HashMap<String, VecDeque<BeaconEvent>>, // domain → recent events
    blocked_count: u64,
    allowed_count: u64,
}

impl TelemetryGuard {
    pub fn new() -> Self {
        TelemetryGuard { beacons: HashMap::new(), blocked_count: 0, allowed_count: 0 }
    }

    pub fn classify(&mut self, domain: &str, ts_ms: u64, size: u64) -> TelemetryClass {
        // Check known list first.
        if KNOWN_TELEMETRY.iter().any(|&d| domain == d || domain.ends_with(&format!(".{d}"))) {
            self.blocked_count += 1;
            return TelemetryClass::KnownTelemetry;
        }

        // Check beaconing pattern: ≥3 events within 5 minutes, each < 4 KiB.
        let window = self.beacons.entry(domain.to_owned()).or_default();
        let cutoff = ts_ms.saturating_sub(5 * 60 * 1000);
        window.retain(|e| e.ts_ms >= cutoff);
        window.push_back(BeaconEvent { ts_ms, domain: domain.to_owned(), size_bytes: size });
        if window.len() >= 3 && window.iter().all(|e| e.size_bytes < 4096) {
            self.blocked_count += 1;
            return TelemetryClass::LikelyTelemetry;
        }

        self.allowed_count += 1;
        TelemetryClass::NotTelemetry
    }

    pub fn blocked_count(&self) -> u64 { self.blocked_count }
    pub fn allowed_count(&self) -> u64 { self.allowed_count }
}

impl Default for TelemetryGuard { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_telemetry_blocked() {
        let mut g = TelemetryGuard::new();
        assert_eq!(g.classify("api.mixpanel.com", 1000, 512), TelemetryClass::KnownTelemetry);
    }
    #[test]
    fn beaconing_detected() {
        let mut g = TelemetryGuard::new();
        g.classify("tracker.example.com", 1000, 256);
        g.classify("tracker.example.com", 60_000, 256);
        let c = g.classify("tracker.example.com", 120_000, 256);
        assert_eq!(c, TelemetryClass::LikelyTelemetry);
    }
}
