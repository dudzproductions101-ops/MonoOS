//! microphone_monitor.rs – Real-time microphone access monitor

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MicSession {
    pub package:    String,
    pub uid:        u32,
    pub started_ms: u64,
    pub ended_ms:   Option<u64>,
}

impl MicSession {
    pub fn is_active(&self) -> bool { self.ended_ms.is_none() }
    pub fn duration_ms(&self, now_ms: u64) -> u64 {
        self.ended_ms.unwrap_or(now_ms).saturating_sub(self.started_ms)
    }
}

pub struct MicrophoneMonitor {
    sessions:      Vec<MicSession>,
    totals_ms:     HashMap<String, u64>,
    indicator_on:  bool,
}

impl MicrophoneMonitor {
    pub fn new() -> Self {
        MicrophoneMonitor { sessions: Vec::new(), totals_ms: HashMap::new(), indicator_on: false }
    }

    pub fn on_mic_open(&mut self, package: impl Into<String>, uid: u32, ts_ms: u64) {
        self.sessions.push(MicSession { package: package.into(), uid, started_ms: ts_ms, ended_ms: None });
        self.indicator_on = true;
    }

    pub fn on_mic_close(&mut self, package: &str, ts_ms: u64) {
        for sess in self.sessions.iter_mut().rev() {
            if sess.package == package && sess.is_active() {
                sess.ended_ms = Some(ts_ms);
                *self.totals_ms.entry(package.to_owned()).or_insert(0) += sess.duration_ms(ts_ms);
                break;
            }
        }
        self.indicator_on = self.sessions.iter().any(|s| s.is_active());
    }

    pub fn is_mic_in_use(&self) -> bool { self.indicator_on }
    pub fn total_usage_ms(&self, package: &str) -> u64 {
        *self.totals_ms.get(package).unwrap_or(&0)
    }
}

impl Default for MicrophoneMonitor { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indicator_on_while_mic_open() {
        let mut mon = MicrophoneMonitor::new();
        assert!(!mon.is_mic_in_use());
        mon.on_mic_open("com.example.app", 10001, 1000);
        assert!(mon.is_mic_in_use());
    }

    #[test]
    fn indicator_off_after_close() {
        let mut mon = MicrophoneMonitor::new();
        mon.on_mic_open("com.example.app", 10001, 1000);
        mon.on_mic_close("com.example.app", 1500);
        assert!(!mon.is_mic_in_use());
    }

    #[test]
    fn indicator_stays_on_while_any_app_holds_mic() {
        let mut mon = MicrophoneMonitor::new();
        mon.on_mic_open("com.app.a", 10001, 1000);
        mon.on_mic_open("com.app.b", 10002, 1100);
        mon.on_mic_close("com.app.a", 1500);
        // App B is still recording, so the system-wide indicator stays on.
        assert!(mon.is_mic_in_use());
        mon.on_mic_close("com.app.b", 1600);
        assert!(!mon.is_mic_in_use());
    }

    #[test]
    fn total_usage_accumulates_across_sessions() {
        let mut mon = MicrophoneMonitor::new();
        mon.on_mic_open("com.example.app", 10001, 0);
        mon.on_mic_close("com.example.app", 500);
        mon.on_mic_open("com.example.app", 10001, 1000);
        mon.on_mic_close("com.example.app", 1300);
        assert_eq!(mon.total_usage_ms("com.example.app"), 800);
    }

    #[test]
    fn total_usage_isolated_per_package() {
        let mut mon = MicrophoneMonitor::new();
        mon.on_mic_open("com.app.a", 10001, 0);
        mon.on_mic_close("com.app.a", 1000);
        assert_eq!(mon.total_usage_ms("com.app.a"), 1000);
        assert_eq!(mon.total_usage_ms("com.app.b"), 0);
    }

    #[test]
    fn close_without_matching_open_is_a_safe_no_op() {
        let mut mon = MicrophoneMonitor::new();
        mon.on_mic_close("com.never.opened", 100);
        assert!(!mon.is_mic_in_use());
        assert_eq!(mon.total_usage_ms("com.never.opened"), 0);
    }

    #[test]
    fn session_duration_for_active_session_uses_now() {
        let session = MicSession { package: "p".into(), uid: 1, started_ms: 100, ended_ms: None };
        assert!(session.is_active());
        assert_eq!(session.duration_ms(500), 400);
    }
}
