//! tracker_blocker.rs – MonoOS Tracker Blocker
//!
//! Maintains a block-list of known tracker domains and IP ranges.
//! Communicates with the kernel netfilter module via a Netlink socket
//! to push the list into kernel space.  Also intercepts DNS responses
//! to return NXDOMAIN for blocked domains.

use std::collections::HashSet;
use std::net::Ipv4Addr;

/// A single block-list entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlockEntry {
    Domain(String),
    IpV4Range { addr: Ipv4Addr, prefix_len: u8 },
}

impl BlockEntry {
    pub fn domain(d: impl Into<String>) -> Self { BlockEntry::Domain(d.into()) }

    pub fn matches_domain(&self, query: &str) -> bool {
        match self {
            BlockEntry::Domain(d) => query == d || query.ends_with(&format!(".{d}")),
            _ => false,
        }
    }
}

/// The main tracker block-list.
pub struct TrackerBlocker {
    entries:       HashSet<BlockEntry>,
    block_count:   u64,
    allow_count:   u64,
    /// Per-package overrides: packages that have user-granted network access
    /// to otherwise-blocked entries.
    exceptions:    HashSet<String>,
}

/// Built-in baseline tracker list (very short subset; full list loaded from
/// /etc/monoos/trackers.conf at runtime).
static BUILTIN_TRACKERS: &[&str] = &[
    "googletagmanager.com",
    "google-analytics.com",
    "doubleclick.net",
    "facebook.com",
    "connect.facebook.net",
    "analytics.twitter.com",
    "static.ads-twitter.com",
    "pixel.advertising.com",
    "scorecardresearch.com",
    "omtrdc.net",
    "adobedtm.com",
    "segment.io",
    "segment.com",
    "mixpanel.com",
    "amplitude.com",
    "braze.com",
    "mparticle.com",
    "appsflyer.com",
    "adjust.com",
    "branch.io",
    "kochava.com",
];

impl TrackerBlocker {
    pub fn new() -> Self {
        let mut blocker = TrackerBlocker {
            entries: HashSet::new(),
            block_count: 0,
            allow_count: 0,
            exceptions: HashSet::new(),
        };
        for &domain in BUILTIN_TRACKERS {
            blocker.entries.insert(BlockEntry::domain(domain));
        }
        blocker
    }

    /// Add a domain or IP range to the block list.
    pub fn add(&mut self, entry: BlockEntry) { self.entries.insert(entry); }

    /// Remove an entry.
    pub fn remove(&mut self, entry: &BlockEntry) -> bool { self.entries.remove(entry) }

    /// Add a per-package exception (user chose to allow trackers for this app).
    pub fn add_exception(&mut self, package: impl Into<String>) {
        self.exceptions.insert(package.into());
    }

    pub fn remove_exception(&mut self, package: &str) {
        self.exceptions.remove(package);
    }

    /// Should this DNS query be blocked?
    pub fn should_block_domain(&mut self, domain: &str, package: &str) -> bool {
        if self.exceptions.contains(package) {
            self.allow_count += 1;
            return false;
        }
        let blocked = self.entries.iter().any(|e| e.matches_domain(domain));
        if blocked { self.block_count += 1; } else { self.allow_count += 1; }
        blocked
    }

    /// Should this IPv4 packet destination be blocked?
    pub fn should_block_ip(&mut self, ip: Ipv4Addr, package: &str) -> bool {
        if self.exceptions.contains(package) { return false; }
        let blocked = self.entries.iter().any(|e| {
            if let BlockEntry::IpV4Range { addr, prefix_len } = e {
                let mask = if *prefix_len == 0 { 0u32 } else {
                    u32::MAX << (32 - prefix_len)
                };
                let net  = u32::from(*addr) & mask;
                let dest = u32::from(ip) & mask;
                net == dest
            } else { false }
        });
        if blocked { self.block_count += 1; } else { self.allow_count += 1; }
        blocked
    }

    pub fn entry_count(&self)  -> usize { self.entries.len() }
    pub fn block_count(&self)  -> u64   { self.block_count }
    pub fn allow_count(&self)  -> u64   { self.allow_count }
    pub fn exception_count(&self) -> usize { self.exceptions.len() }
}

impl Default for TrackerBlocker { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_blocks() {
        let mut tb = TrackerBlocker::new();
        assert!(tb.should_block_domain("googletagmanager.com", "com.app"));
        assert!(tb.should_block_domain("sub.googletagmanager.com", "com.app"));
        assert!(!tb.should_block_domain("google.com", "com.app"));
    }

    #[test]
    fn exception_bypasses_block() {
        let mut tb = TrackerBlocker::new();
        tb.add_exception("com.trusted");
        assert!(!tb.should_block_domain("googletagmanager.com", "com.trusted"));
    }

    #[test]
    fn custom_domain_blocked() {
        let mut tb = TrackerBlocker::new();
        tb.add(BlockEntry::domain("evil-tracker.io"));
        assert!(tb.should_block_domain("evil-tracker.io", "com.app"));
        assert!(tb.should_block_domain("cdn.evil-tracker.io", "com.app"));
    }
}
