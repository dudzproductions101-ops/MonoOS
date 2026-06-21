//! network_stack.rs – MonoOS userspace network stack manager
//!
//! Wraps kernel networking (via netlink sockets) and wpa_supplicant's
//! D-Bus interface to provide a unified API for managing interfaces,
//! routes, and connectivity state.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceKind { Loopback, Wifi, Cellular, Ethernet, Vpn, Tun }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState { Down, Dormant, Up, Connected, NoNetwork }

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name:   String,
    pub kind:   InterfaceKind,
    pub state:  InterfaceState,
    pub ipv4:   Option<Ipv4Addr>,
    pub ipv6:   Option<Ipv6Addr>,
    pub mtu:    u32,
    pub metric: u32,
}

impl NetworkInterface {
    pub fn new(name: impl Into<String>, kind: InterfaceKind) -> Self {
        NetworkInterface {
            name: name.into(), kind, state: InterfaceState::Down,
            ipv4: None, ipv6: None, mtu: 1500, metric: 100,
        }
    }
    pub fn is_connected(&self) -> bool {
        self.state == InterfaceState::Connected && self.ipv4.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct Route {
    pub destination: IpAddr,
    pub prefix_len:  u8,
    pub gateway:     Option<IpAddr>,
    pub interface:   String,
    pub metric:      u32,
}

pub struct NetworkStack {
    interfaces: HashMap<String, NetworkInterface>,
    routes:     Vec<Route>,
    dns_servers: Vec<IpAddr>,
    default_iface: Option<String>,
}

impl NetworkStack {
    pub fn new() -> Self {
        let mut ns = NetworkStack {
            interfaces: HashMap::new(), routes: Vec::new(),
            dns_servers: Vec::new(), default_iface: None,
        };
        // Always add loopback.
        let mut lo = NetworkInterface::new("lo", InterfaceKind::Loopback);
        lo.state = InterfaceState::Up;
        lo.ipv4  = Some(Ipv4Addr::LOCALHOST);
        ns.interfaces.insert("lo".into(), lo);
        ns
    }

    pub fn add_interface(&mut self, iface: NetworkInterface) {
        self.interfaces.insert(iface.name.clone(), iface);
    }

    pub fn set_state(&mut self, name: &str, state: InterfaceState) -> bool {
        if let Some(iface) = self.interfaces.get_mut(name) {
            iface.state = state;
            if state == InterfaceState::Connected && self.default_iface.is_none() {
                self.default_iface = Some(name.to_owned());
            }
            true
        } else { false }
    }

    pub fn assign_ipv4(&mut self, name: &str, addr: Ipv4Addr) -> bool {
        self.interfaces.get_mut(name).map(|i| { i.ipv4 = Some(addr); true }).unwrap_or(false)
    }

    pub fn add_route(&mut self, route: Route) { self.routes.push(route); }

    pub fn set_dns(&mut self, servers: Vec<IpAddr>) { self.dns_servers = servers; }

    pub fn default_interface(&self) -> Option<&NetworkInterface> {
        self.default_iface.as_ref().and_then(|n| self.interfaces.get(n))
    }

    pub fn is_connected(&self) -> bool {
        self.interfaces.values().any(|i| i.is_connected())
    }

    pub fn interfaces(&self) -> impl Iterator<Item = &NetworkInterface> {
        self.interfaces.values()
    }
}

impl Default for NetworkStack { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connected_after_ip_assign() {
        let mut ns = NetworkStack::new();
        ns.add_interface(NetworkInterface::new("wlan0", InterfaceKind::Wifi));
        ns.assign_ipv4("wlan0", Ipv4Addr::new(192, 168, 1, 42));
        ns.set_state("wlan0", InterfaceState::Connected);
        assert!(ns.is_connected());
    }
}
