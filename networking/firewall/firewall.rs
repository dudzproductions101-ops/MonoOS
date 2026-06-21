//! firewall.rs – MonoOS userspace firewall rule manager
//!
//! Manages per-app and global firewall rules, persisting them to
//! /etc/monoos/firewall.conf and pushing them to the kernel via
//! the monoos_net kernel module.

use std::collections::HashMap;
use std::net::IpAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol { Tcp, Udp, Any }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action { Allow, Drop, Reject }

#[derive(Debug, Clone)]
pub struct FirewallRule {
    pub id:       u32,
    pub uid:      Option<u32>,      // None = global
    pub dst_ip:   Option<IpAddr>,
    pub dst_port: Option<u16>,
    pub proto:    Protocol,
    pub action:   Action,
    pub comment:  String,
}

pub struct Firewall {
    rules:    Vec<FirewallRule>,
    next_id:  u32,
    default:  Action,   // default policy for outbound
}

impl Firewall {
    pub fn new() -> Self {
        Firewall { rules: Vec::new(), next_id: 1, default: Action::Allow }
    }

    pub fn set_default_policy(&mut self, action: Action) { self.default = action; }

    pub fn add_rule(&mut self, mut rule: FirewallRule) -> u32 {
        rule.id = self.next_id;
        self.next_id += 1;
        self.rules.push(rule);
        self.next_id - 1
    }

    pub fn remove_rule(&mut self, id: u32) -> bool {
        let before = self.rules.len();
        self.rules.retain(|r| r.id != id);
        self.rules.len() < before
    }

    pub fn evaluate(&self, uid: u32, dst: IpAddr, port: u16, proto: Protocol) -> Action {
        for rule in &self.rules {
            if let Some(rule_uid) = rule.uid {
                if rule_uid != uid { continue; }
            }
            if let Some(ip) = rule.dst_ip {
                if ip != dst { continue; }
            }
            if let Some(p) = rule.dst_port {
                if p != port { continue; }
            }
            if rule.proto != Protocol::Any && rule.proto != proto { continue; }
            return rule.action;
        }
        self.default
    }

    pub fn rules_for_uid(&self, uid: u32) -> Vec<&FirewallRule> {
        self.rules.iter().filter(|r| r.uid == Some(uid) || r.uid.is_none()).collect()
    }

    pub fn block_uid(&mut self, uid: u32) -> u32 {
        self.add_rule(FirewallRule {
            id: 0, uid: Some(uid), dst_ip: None, dst_port: None,
            proto: Protocol::Any, action: Action::Drop,
            comment: format!("block uid {uid}"),
        })
    }

    pub fn rule_count(&self) -> usize { self.rules.len() }
}

impl Default for Firewall { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use std::str::FromStr;

    #[test]
    fn block_uid_drops_all() {
        let mut fw = Firewall::new();
        fw.block_uid(10001);
        let ip = IpAddr::from_str("8.8.8.8").unwrap();
        assert_eq!(fw.evaluate(10001, ip, 443, Protocol::Tcp), Action::Drop);
        assert_eq!(fw.evaluate(10002, ip, 443, Protocol::Tcp), Action::Allow);
    }
}
