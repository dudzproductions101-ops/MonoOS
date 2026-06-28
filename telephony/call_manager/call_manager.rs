//! call_manager.rs – MonoOS Call Manager
//!
//! Manages voice call lifecycle: dialling, ringing, answering, holding,
//! conferencing, and hanging up.  Interfaces with the modem via AT
//! commands through /dev/ttyMODEM0 and surfaces state to the Phone UI.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallState { Idle, Dialling, Ringing, Active, Holding, Disconnected }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallDirection { Incoming, Outgoing }

#[derive(Debug, Clone)]
pub struct Call {
    pub id:         u32,
    pub number:     String,
    pub direction:  CallDirection,
    pub state:      CallState,
    pub started_at: u64,   // Unix ms
    pub duration_ms: u64,
    pub on_hold:    bool,
    pub muted:      bool,
}

impl Call {
    pub fn new_outgoing(id: u32, number: impl Into<String>, ts: u64) -> Self {
        Call { id, number: number.into(), direction: CallDirection::Outgoing,
               state: CallState::Dialling, started_at: ts, duration_ms: 0,
               on_hold: false, muted: false }
    }
    pub fn new_incoming(id: u32, number: impl Into<String>, ts: u64) -> Self {
        Call { id, number: number.into(), direction: CallDirection::Incoming,
               state: CallState::Ringing, started_at: ts, duration_ms: 0,
               on_hold: false, muted: false }
    }
    pub fn is_active(&self) -> bool { self.state == CallState::Active }
}

pub struct CallManager {
    calls:    HashMap<u32, Call>,
    next_id:  u32,
    muted:    bool,
    speaker:  bool,
}

impl CallManager {
    pub fn new() -> Self {
        CallManager { calls: HashMap::new(), next_id: 1, muted: false, speaker: false }
    }

    pub fn dial(&mut self, number: impl Into<String>, ts: u64) -> u32 {
        let id = self.next_id; self.next_id += 1;
        // Send ATD<number>; to modem (real impl).
        self.calls.insert(id, Call::new_outgoing(id, number, ts));
        id
    }

    pub fn incoming(&mut self, number: impl Into<String>, ts: u64) -> u32 {
        let id = self.next_id; self.next_id += 1;
        self.calls.insert(id, Call::new_incoming(id, number, ts));
        id
    }

    pub fn answer(&mut self, id: u32) -> bool {
        if let Some(c) = self.calls.get_mut(&id) {
            if c.state == CallState::Ringing { c.state = CallState::Active; return true; }
        }
        false
    }

    pub fn hang_up(&mut self, id: u32) -> bool {
        if let Some(c) = self.calls.get_mut(&id) {
            c.state = CallState::Disconnected; return true;
        }
        false
    }

    pub fn hold(&mut self, id: u32) -> bool {
        if let Some(c) = self.calls.get_mut(&id) {
            if c.is_active() { c.state = CallState::Holding; c.on_hold = true; return true; }
        }
        false
    }

    pub fn unhold(&mut self, id: u32) -> bool {
        if let Some(c) = self.calls.get_mut(&id) {
            if c.state == CallState::Holding { c.state = CallState::Active; c.on_hold = false; return true; }
        }
        false
    }

    pub fn set_mute(&mut self, muted: bool) { self.muted = muted; }
    pub fn set_speaker(&mut self, on: bool)  { self.speaker = on; }
    pub fn active_calls(&self) -> Vec<&Call> { self.calls.values().filter(|c| c.is_active()).collect() }
    pub fn call_count(&self) -> usize { self.calls.len() }
}

impl Default for CallManager { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dial_and_hangup() {
        let mut cm = CallManager::new();
        let id = cm.dial("+1555000123", 0);
        assert_eq!(cm.calls[&id].state, CallState::Dialling);
        assert!(cm.hang_up(id));
        assert_eq!(cm.calls[&id].state, CallState::Disconnected);
    }
    #[test]
    fn incoming_answer() {
        let mut cm = CallManager::new();
        let id = cm.incoming("+1555000456", 0);
        assert!(cm.answer(id));
        assert!(cm.calls[&id].is_active());
    }
}
