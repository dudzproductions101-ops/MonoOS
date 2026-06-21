//! vpn_manager.rs – MonoOS VPN connection manager
//!
//! Manages WireGuard and OpenVPN connections.  Creates a tun device,
//! routes traffic through the VPN tunnel, and notifies the network
//! stack of the VPN interface's connectivity state.

use std::net::{IpAddr, Ipv4Addr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VpnProtocol { WireGuard, OpenVpn }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VpnState { Disconnected, Connecting, Connected, Reconnecting, Error }

#[derive(Debug, Clone)]
pub struct VpnProfile {
    pub name:       String,
    pub protocol:   VpnProtocol,
    pub server:     String,
    pub port:       u16,
    /// WireGuard public key or OpenVPN CA certificate path.
    pub credential: String,
    pub dns_servers: Vec<Ipv4Addr>,
    pub split_tunnel: bool,  // false = all traffic through VPN
}

impl VpnProfile {
    pub fn wireguard(name: impl Into<String>, server: impl Into<String>, port: u16, pubkey: impl Into<String>) -> Self {
        VpnProfile {
            name: name.into(), protocol: VpnProtocol::WireGuard,
            server: server.into(), port,
            credential: pubkey.into(),
            dns_servers: vec![Ipv4Addr::new(10, 64, 64, 11)],
            split_tunnel: false,
        }
    }
}

pub struct VpnManager {
    profiles:        Vec<VpnProfile>,
    active_profile:  Option<usize>,
    state:           VpnState,
    tun_ip:          Option<IpAddr>,
    bytes_sent:      u64,
    bytes_recv:      u64,
}

impl VpnManager {
    pub fn new() -> Self {
        VpnManager { profiles: Vec::new(), active_profile: None,
                     state: VpnState::Disconnected, tun_ip: None,
                     bytes_sent: 0, bytes_recv: 0 }
    }

    pub fn add_profile(&mut self, profile: VpnProfile) -> usize {
        self.profiles.push(profile);
        self.profiles.len() - 1
    }

    pub fn connect(&mut self, profile_index: usize) -> Result<(), &'static str> {
        if profile_index >= self.profiles.len() { return Err("profile not found"); }
        self.state = VpnState::Connecting;
        // Real impl: fork WireGuard/OpenVPN process, create tun0, set routes.
        self.active_profile = Some(profile_index);
        self.tun_ip = Some(IpAddr::from(Ipv4Addr::new(10, 0, 0, 2)));
        self.state = VpnState::Connected;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        // Real impl: send SIGTERM to VPN process, remove routes, delete tun.
        self.state = VpnState::Disconnected;
        self.active_profile = None;
        self.tun_ip = None;
    }

    pub fn state(&self) -> VpnState { self.state }
    pub fn is_connected(&self) -> bool { self.state == VpnState::Connected }
    pub fn tun_ip(&self) -> Option<IpAddr> { self.tun_ip }
    pub fn active_profile(&self) -> Option<&VpnProfile> {
        self.active_profile.and_then(|i| self.profiles.get(i))
    }
    pub fn record_traffic(&mut self, tx: u64, rx: u64) {
        self.bytes_sent += tx; self.bytes_recv += rx;
    }
}

impl Default for VpnManager { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connect_disconnect() {
        let mut mgr = VpnManager::new();
        let idx = mgr.add_profile(VpnProfile::wireguard("Home VPN", "1.2.3.4", 51820, "abc123"));
        mgr.connect(idx).unwrap();
        assert!(mgr.is_connected());
        assert!(mgr.tun_ip().is_some());
        mgr.disconnect();
        assert!(!mgr.is_connected());
    }
}
