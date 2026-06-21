//! captive_portal.rs – Captive portal detection and login
//!
//! After Wi-Fi association, probes a known URL to detect captive portals
//! (hotel/airport Wi-Fi login pages) and opens a minimal browser view.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortalState { Unknown, Open, Captive, Error }

pub struct CaptivePortalDetector {
    probe_url:    String,
    expected_code: u16,
    state:        PortalState,
    portal_url:   Option<String>,
    probe_count:  u32,
}

impl CaptivePortalDetector {
    pub fn new() -> Self {
        CaptivePortalDetector {
            probe_url:     "http://connectivity.monoOS.io/probe".into(),
            expected_code: 204,
            state:         PortalState::Unknown,
            portal_url:    None,
            probe_count:   0,
        }
    }

    /// Probe the network.  Returns the updated state.
    /// In production this makes a real HTTP GET; here it is a stub.
    pub fn probe(&mut self, interface: &str) -> PortalState {
        self.probe_count += 1;
        // Stub: assume open network.
        let _ = interface;
        self.state = PortalState::Open;
        self.state
    }

    /// Call when a 302 redirect is observed during the probe.
    pub fn on_redirect(&mut self, location: String) {
        self.state = PortalState::Captive;
        self.portal_url = Some(location);
    }

    pub fn state(&self) -> PortalState { self.state }
    pub fn portal_url(&self) -> Option<&str> { self.portal_url.as_deref() }
    pub fn probe_count(&self) -> u32 { self.probe_count }
}

impl Default for CaptivePortalDetector { fn default() -> Self { Self::new() } }
