//! carrier_profiles.rs – Carrier APN and network configuration
//!
//! Stores and looks up carrier-specific settings: APNs, MMSC, MMS
//! proxy, Wi-Fi Calling parameters, and VoLTE configuration.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ApnConfig {
    pub name:       String,
    pub apn:        String,
    pub username:   String,
    pub password:   String,
    pub mmsc:       String,
    pub mms_proxy:  String,
    pub mms_port:   u16,
    pub mcc:        String,
    pub mnc:        String,
    pub auth_type:  u8,   // 0=none,1=PAP,2=CHAP,3=PAP/CHAP
    pub apn_type:   String, // "default,mms,supl"
}

impl ApnConfig {
    pub fn new(name: impl Into<String>, apn: impl Into<String>, mcc: impl Into<String>, mnc: impl Into<String>) -> Self {
        ApnConfig {
            name: name.into(), apn: apn.into(), username: String::new(),
            password: String::new(), mmsc: String::new(), mms_proxy: String::new(),
            mms_port: 80, mcc: mcc.into(), mnc: mnc.into(),
            auth_type: 0, apn_type: "default,mms,supl".into(),
        }
    }
}

pub struct CarrierProfileManager {
    /// mcc+mnc → list of APN configs
    profiles: HashMap<String, Vec<ApnConfig>>,
}

impl CarrierProfileManager {
    pub fn new() -> Self { CarrierProfileManager { profiles: HashMap::new() } }

    pub fn add(&mut self, cfg: ApnConfig) {
        let key = format!("{}{}", cfg.mcc, cfg.mnc);
        self.profiles.entry(key).or_default().push(cfg);
    }

    pub fn lookup(&self, mcc: &str, mnc: &str) -> Vec<&ApnConfig> {
        let key = format!("{mcc}{mnc}");
        self.profiles.get(&key).map(|v| v.iter().collect()).unwrap_or_default()
    }

    pub fn default_apn(&self, mcc: &str, mnc: &str) -> Option<&ApnConfig> {
        self.lookup(mcc, mnc).into_iter().find(|a| a.apn_type.contains("default"))
    }

    pub fn carrier_count(&self) -> usize { self.profiles.len() }
}

impl Default for CarrierProfileManager { fn default() -> Self { Self::new() } }
