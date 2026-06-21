//! esim_manager.rs – MonoOS eSIM / eUICC Manager
//!
//! Implements GSMA SGP.22 RSP (Remote SIM Provisioning) for eSIM profile
//! download, activation, and deletion.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileState { Inactive, Active, Error }

#[derive(Debug, Clone)]
pub struct EsimProfile {
    pub iccid:    String,
    pub name:     String,
    pub provider: String,
    pub state:    ProfileState,
    pub eid:      String,   // eUICC identifier
}

pub struct EsimManager {
    profiles: Vec<EsimProfile>,
    eid:      String,
}

impl EsimManager {
    pub fn new(eid: impl Into<String>) -> Self {
        EsimManager { profiles: Vec::new(), eid: eid.into() }
    }

    /// Download and install a profile from an SM-DP+ server.
    pub fn download_profile(&mut self, activation_code: &str) -> Result<&EsimProfile, &'static str> {
        // Real impl: HTTPS request to SM-DP+, ES8+ protocol, LPA integration.
        let _ = activation_code;
        let p = EsimProfile {
            iccid:    format!("89{:018}", self.profiles.len()),
            name:     "Downloaded Profile".into(),
            provider: "MonoOS Carrier".into(),
            state:    ProfileState::Inactive,
            eid:      self.eid.clone(),
        };
        self.profiles.push(p);
        Ok(self.profiles.last().unwrap())
    }

    pub fn activate(&mut self, iccid: &str) -> bool {
        let mut found = false;
        for p in &mut self.profiles {
            if p.iccid == iccid { p.state = ProfileState::Active; found = true; }
            else if p.state == ProfileState::Active { p.state = ProfileState::Inactive; }
        }
        found
    }

    pub fn delete(&mut self, iccid: &str) -> bool {
        let before = self.profiles.len();
        self.profiles.retain(|p| p.iccid != iccid);
        self.profiles.len() < before
    }

    pub fn active_profile(&self) -> Option<&EsimProfile> {
        self.profiles.iter().find(|p| p.state == ProfileState::Active)
    }

    pub fn profile_count(&self) -> usize { self.profiles.len() }
    pub fn eid(&self) -> &str { &self.eid }
}

impl Default for EsimManager {
    fn default() -> Self { Self::new("89000000000000000000") }
}
