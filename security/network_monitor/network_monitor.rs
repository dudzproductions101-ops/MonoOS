//! network_monitor.rs – Per-app network usage and anomaly monitor

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct AppNetworkStats {
    pub package:    String,
    pub uid:        u32,
    pub tx_bytes:   u64,
    pub rx_bytes:   u64,
    pub conn_count: u32,
    pub blocked:    u32,
}

pub struct NetworkMonitor {
    stats:          HashMap<u32, AppNetworkStats>,
    total_blocked:  u64,
}

impl NetworkMonitor {
    pub fn new() -> Self {
        NetworkMonitor { stats: HashMap::new(), total_blocked: 0 }
    }

    pub fn record_packet(&mut self, uid: u32, package: &str, tx: u64, rx: u64, blocked: bool) {
        let s = self.stats.entry(uid).or_insert_with(|| AppNetworkStats {
            package: package.to_owned(), uid, ..Default::default()
        });
        s.tx_bytes += tx;
        s.rx_bytes += rx;
        s.conn_count += 1;
        if blocked { s.blocked += 1; self.total_blocked += 1; }
    }

    pub fn stats_for(&self, uid: u32) -> Option<&AppNetworkStats> { self.stats.get(&uid) }
    pub fn all_stats(&self) -> Vec<&AppNetworkStats> { self.stats.values().collect() }
    pub fn total_blocked(&self) -> u64 { self.total_blocked }
}

impl Default for NetworkMonitor { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_packet_accumulates_bytes_and_count() {
        let mut mon = NetworkMonitor::new();
        mon.record_packet(10001, "com.example.app", 100, 200, false);
        mon.record_packet(10001, "com.example.app", 50, 75, false);
        let stats = mon.stats_for(10001).expect("stats should exist");
        assert_eq!(stats.tx_bytes, 150);
        assert_eq!(stats.rx_bytes, 275);
        assert_eq!(stats.conn_count, 2);
        assert_eq!(stats.blocked, 0);
    }

    #[test]
    fn blocked_packets_increment_both_app_and_total_counters() {
        let mut mon = NetworkMonitor::new();
        mon.record_packet(10001, "com.example.app", 10, 0, true);
        mon.record_packet(10001, "com.example.app", 10, 0, false);
        mon.record_packet(10002, "com.other.app", 10, 0, true);

        assert_eq!(mon.stats_for(10001).unwrap().blocked, 1);
        assert_eq!(mon.stats_for(10002).unwrap().blocked, 1);
        assert_eq!(mon.total_blocked(), 2);
    }

    #[test]
    fn stats_isolated_per_uid() {
        let mut mon = NetworkMonitor::new();
        mon.record_packet(10001, "com.app.a", 100, 0, false);
        mon.record_packet(10002, "com.app.b", 999, 0, false);
        assert_eq!(mon.stats_for(10001).unwrap().tx_bytes, 100);
        assert_eq!(mon.stats_for(10002).unwrap().tx_bytes, 999);
    }

    #[test]
    fn unknown_uid_returns_none() {
        let mon = NetworkMonitor::new();
        assert!(mon.stats_for(99999).is_none());
    }

    #[test]
    fn all_stats_returns_every_tracked_app() {
        let mut mon = NetworkMonitor::new();
        mon.record_packet(10001, "com.app.a", 1, 1, false);
        mon.record_packet(10002, "com.app.b", 1, 1, false);
        mon.record_packet(10003, "com.app.c", 1, 1, false);
        assert_eq!(mon.all_stats().len(), 3);
    }

    #[test]
    fn package_name_is_recorded_on_first_packet() {
        let mut mon = NetworkMonitor::new();
        mon.record_packet(10001, "com.example.app", 1, 1, false);
        assert_eq!(mon.stats_for(10001).unwrap().package, "com.example.app");
    }
}
