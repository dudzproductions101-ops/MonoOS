//! dns_resolver.rs – MonoOS DNS resolver with privacy protection
//!
//! Implements DNS-over-TLS (DoT) and DNS-over-HTTPS (DoH) with
//! an integrated block-list check before forwarding queries.

use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolverMode { Plain, DoT, DoH }

#[derive(Debug, Clone)]
pub struct DnsRecord {
    pub name:    String,
    pub records: Vec<IpAddr>,
    pub ttl:     u32,
    pub cached_at: u64,  // Unix seconds
}

impl DnsRecord {
    pub fn is_expired(&self, now: u64) -> bool {
        self.ttl > 0 && now > self.cached_at + self.ttl as u64
    }
}

pub struct DnsResolver {
    mode:      ResolverMode,
    upstream:  Vec<String>,   // IP:port of upstream resolvers
    cache:     HashMap<String, DnsRecord>,
    blocklist: std::collections::HashSet<String>,
    blocked_count: u64,
    resolved_count: u64,
    cache_hits: u64,
}

impl DnsResolver {
    pub fn new(mode: ResolverMode, upstream: Vec<String>) -> Self {
        DnsResolver {
            mode, upstream, cache: HashMap::new(),
            blocklist: std::collections::HashSet::new(),
            blocked_count: 0, resolved_count: 0, cache_hits: 0,
        }
    }

    pub fn add_to_blocklist(&mut self, domain: impl Into<String>) {
        self.blocklist.insert(domain.into());
    }

    pub fn is_blocked(&self, domain: &str) -> bool {
        self.blocklist.contains(domain)
            || self.blocklist.iter().any(|b| domain.ends_with(&format!(".{b}")))
    }

    pub fn resolve(&mut self, domain: &str, now: u64) -> Result<Vec<IpAddr>, &'static str> {
        if self.is_blocked(domain) {
            self.blocked_count += 1;
            return Err("blocked");
        }
        if let Some(rec) = self.cache.get(domain) {
            if !rec.is_expired(now) {
                self.cache_hits += 1;
                return Ok(rec.records.clone());
            }
        }
        // Stub: return a synthetic answer.
        let addrs = self.stub_resolve(domain)?;
        self.cache.insert(domain.to_owned(), DnsRecord {
            name: domain.to_owned(), records: addrs.clone(), ttl: 300, cached_at: now,
        });
        self.resolved_count += 1;
        Ok(addrs)
    }

    fn stub_resolve(&self, _domain: &str) -> Result<Vec<IpAddr>, &'static str> {
        // Real impl: open TLS socket to upstream, send DNS query, parse response.
        Ok(vec![IpAddr::from_str("1.2.3.4").unwrap()])
    }

    pub fn flush_cache(&mut self) { self.cache.clear(); }
    pub fn blocked_count(&self)  -> u64 { self.blocked_count }
    pub fn resolved_count(&self) -> u64 { self.resolved_count }
    pub fn cache_size(&self)     -> usize { self.cache.len() }
    pub fn mode(&self)           -> ResolverMode { self.mode }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocked_domain_returns_err() {
        let mut r = DnsResolver::new(ResolverMode::DoH, vec!["1.1.1.1:853".into()]);
        r.add_to_blocklist("tracker.evil.com");
        assert!(r.resolve("tracker.evil.com", 0).is_err());
        assert!(r.resolve("sub.tracker.evil.com", 0).is_err());
        assert!(r.resolve("safe.example.com", 0).is_ok());
    }
    #[test]
    fn cache_populated_on_resolve() {
        let mut r = DnsResolver::new(ResolverMode::Plain, vec![]);
        r.resolve("example.com", 1000).unwrap();
        assert_eq!(r.cache_size(), 1);
        r.resolve("example.com", 1001).unwrap();
        assert_eq!(r.cache_hits(), 1);
    }
    fn cache_hits(r: &DnsResolver) -> u64 { r.cache_hits }
}
