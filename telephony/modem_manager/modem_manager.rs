//! modem_manager.rs – MonoOS Modem Manager
//!
//! Manages the cellular baseband modem: power, registration, SIM,
//! and AT command dispatch.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationState { Unregistered, Home, Searching, Denied, Roaming }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkTech { Unknown, Gsm, Wcdma, Lte, NrSa, NrNsa }

#[derive(Debug, Clone)]
pub struct SimInfo {
    pub iccid:  String,
    pub imsi:   String,
    pub mcc:    String,
    pub mnc:    String,
    pub spn:    String,    // Service Provider Name
    pub slot:   u8,
}

#[derive(Debug, Clone)]
pub struct SignalStrength {
    pub rssi_dbm: i32,
    pub rsrp_dbm: i32,
    pub snr_db:   f32,
    pub bars:     u8,      // 0–4
}

pub struct ModemManager {
    powered:       bool,
    registration:  RegistrationState,
    tech:          NetworkTech,
    sim:           Option<SimInfo>,
    signal:        Option<SignalStrength>,
    data_enabled:  bool,
    roaming_data:  bool,
    operator:      String,
}

impl ModemManager {
    pub fn new() -> Self {
        ModemManager {
            powered: false, registration: RegistrationState::Unregistered,
            tech: NetworkTech::Unknown, sim: None, signal: None,
            data_enabled: true, roaming_data: false, operator: String::new(),
        }
    }

    pub fn power_on(&mut self)  { self.powered = true; }
    pub fn power_off(&mut self) { self.powered = false; self.registration = RegistrationState::Unregistered; }

    pub fn on_registered(&mut self, state: RegistrationState, tech: NetworkTech, operator: impl Into<String>) {
        self.registration = state;
        self.tech = tech;
        self.operator = operator.into();
    }

    pub fn on_sim_inserted(&mut self, info: SimInfo) { self.sim = Some(info); }
    pub fn on_signal_update(&mut self, sig: SignalStrength) { self.signal = Some(sig); }

    pub fn send_at(&self, cmd: &str) -> Result<String, &'static str> {
        if !self.powered { return Err("modem not powered"); }
        // Real impl: write to /dev/ttyMODEM0, read response.
        Ok(format!("OK (stub response for: {cmd})"))
    }

    pub fn is_registered(&self) -> bool {
        matches!(self.registration, RegistrationState::Home | RegistrationState::Roaming)
    }
    pub fn sim_present(&self)   -> bool { self.sim.is_some() }
    pub fn operator(&self)      -> &str { &self.operator }
    pub fn registration(&self)  -> RegistrationState { self.registration }
    pub fn tech(&self)          -> NetworkTech { self.tech }
    pub fn signal(&self)        -> Option<&SignalStrength> { self.signal.as_ref() }
}

impl Default for ModemManager { fn default() -> Self { Self::new() } }
