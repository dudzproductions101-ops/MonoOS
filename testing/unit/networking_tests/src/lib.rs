//! Unit tests for MonoOS networking layer.

#[cfg(test)]
mod firewall_tests {
    use std::net::IpAddr;
    use std::str::FromStr;

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Action { Allow, Drop }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Proto { Tcp, Udp, Any }

    struct Rule { uid: Option<u32>, dst: Option<IpAddr>, port: Option<u16>, proto: Proto, action: Action }
    struct Fw { rules: Vec<Rule>, default: Action }

    impl Fw {
        fn new() -> Self { Fw { rules: vec![], default: Action::Allow } }
        fn add(&mut self, r: Rule) { self.rules.push(r); }
        fn eval(&self, uid: u32, ip: IpAddr, port: u16, proto: Proto) -> Action {
            for r in &self.rules {
                if r.uid.map_or(false, |u| u != uid)    { continue; }
                if r.dst.map_or(false, |d| d != ip)     { continue; }
                if r.port.map_or(false, |p| p != port)  { continue; }
                if r.proto != Proto::Any && r.proto != proto { continue; }
                return r.action;
            }
            self.default
        }
    }

    #[test]
    fn default_allow() {
        let fw = Fw::new();
        assert_eq!(fw.eval(10001, IpAddr::from_str("8.8.8.8").unwrap(), 443, Proto::Tcp), Action::Allow);
    }

    #[test]
    fn uid_block_rule() {
        let mut fw = Fw::new();
        fw.add(Rule { uid: Some(10001), dst: None, port: None, proto: Proto::Any, action: Action::Drop });
        assert_eq!(fw.eval(10001, IpAddr::from_str("8.8.8.8").unwrap(), 80, Proto::Tcp), Action::Drop);
        assert_eq!(fw.eval(10002, IpAddr::from_str("8.8.8.8").unwrap(), 80, Proto::Tcp), Action::Allow);
    }

    #[test]
    fn port_specific_rule() {
        let mut fw = Fw::new();
        fw.add(Rule { uid: None, dst: None, port: Some(25), proto: Proto::Tcp, action: Action::Drop });
        assert_eq!(fw.eval(10001, IpAddr::from_str("1.2.3.4").unwrap(), 25, Proto::Tcp), Action::Drop);
        assert_eq!(fw.eval(10001, IpAddr::from_str("1.2.3.4").unwrap(), 443, Proto::Tcp), Action::Allow);
    }

    #[test]
    fn protocol_specific_rule_does_not_match_other_protocol() {
        // A UDP-only block rule (e.g. blocking plaintext DNS-over-UDP)
        // must not affect TCP traffic on the same port, and vice versa.
        let mut fw = Fw::new();
        fw.add(Rule { uid: None, dst: None, port: Some(53), proto: Proto::Udp, action: Action::Drop });
        assert_eq!(fw.eval(10001, IpAddr::from_str("1.1.1.1").unwrap(), 53, Proto::Udp), Action::Drop);
        assert_eq!(fw.eval(10001, IpAddr::from_str("1.1.1.1").unwrap(), 53, Proto::Tcp), Action::Allow);
    }
}

#[cfg(test)]
mod dns_tests {
    use std::collections::HashSet;

    struct Resolver {
        blocklist: HashSet<String>,
        cache: std::collections::HashMap<String, Vec<String>>,
    }

    impl Resolver {
        fn new() -> Self { Resolver { blocklist: HashSet::new(), cache: Default::default() } }
        fn block(&mut self, d: &str) { self.blocklist.insert(d.to_owned()); }
        fn is_blocked(&self, d: &str) -> bool {
            self.blocklist.contains(d)
                || self.blocklist.iter().any(|b| d.ends_with(&format!(".{b}")))
        }
        fn resolve(&mut self, domain: &str) -> Result<Vec<String>, &'static str> {
            if self.is_blocked(domain) { return Err("blocked"); }
            Ok(self.cache.get(domain).cloned().unwrap_or_else(|| vec!["1.2.3.4".into()]))
        }
    }

    #[test]
    fn blocked_domain_rejected() {
        let mut r = Resolver::new();
        r.block("evil.io");
        assert!(r.resolve("evil.io").is_err());
        assert!(r.resolve("sub.evil.io").is_err());
        assert!(r.resolve("safe.com").is_ok());
    }

    #[test]
    fn unblocked_resolves() {
        let mut r = Resolver::new();
        let addrs = r.resolve("example.com").unwrap();
        assert!(!addrs.is_empty());
    }
}
