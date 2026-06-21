//! sms_manager.rs – MonoOS SMS/MMS Manager
//!
//! Sends, receives, and stores SMS and MMS messages.
//! Interfaces with the modem via AT commands (SMS) and
//! the carrier's MMSC for MMS.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType { Sms, Mms }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStatus { Draft, Queued, Sent, Delivered, Failed, Received }

#[derive(Debug, Clone)]
pub struct Message {
    pub id:          u64,
    pub thread_id:   u64,
    pub address:     String,
    pub body:        String,
    pub msg_type:    MessageType,
    pub status:      MessageStatus,
    pub timestamp_ms: u64,
    pub is_read:     bool,
    pub attachments: Vec<String>,   // MMS content URIs
}

pub struct SmsManager {
    messages:    Vec<Message>,
    threads:     HashMap<String, u64>,  // address → thread_id
    next_msg_id: u64,
    next_thread: u64,
    sent_count:  u64,
    recv_count:  u64,
}

impl SmsManager {
    pub fn new() -> Self {
        SmsManager { messages: Vec::new(), threads: HashMap::new(),
                     next_msg_id: 1, next_thread: 1,
                     sent_count: 0, recv_count: 0 }
    }

    fn get_or_create_thread(&mut self, address: &str) -> u64 {
        if let Some(&tid) = self.threads.get(address) { return tid; }
        let tid = self.next_thread; self.next_thread += 1;
        self.threads.insert(address.to_owned(), tid);
        tid
    }

    pub fn send_sms(&mut self, to: impl Into<String>, body: impl Into<String>, ts: u64) -> u64 {
        let to = to.into();
        let tid = self.get_or_create_thread(&to);
        let id  = self.next_msg_id; self.next_msg_id += 1;
        // Real impl: AT+CMGS="<to>"<body>
        self.messages.push(Message {
            id, thread_id: tid, address: to, body: body.into(),
            msg_type: MessageType::Sms, status: MessageStatus::Sent,
            timestamp_ms: ts, is_read: true, attachments: Vec::new(),
        });
        self.sent_count += 1;
        id
    }

    pub fn receive_sms(&mut self, from: impl Into<String>, body: impl Into<String>, ts: u64) -> u64 {
        let from = from.into();
        let tid  = self.get_or_create_thread(&from);
        let id   = self.next_msg_id; self.next_msg_id += 1;
        self.messages.push(Message {
            id, thread_id: tid, address: from, body: body.into(),
            msg_type: MessageType::Sms, status: MessageStatus::Received,
            timestamp_ms: ts, is_read: false, attachments: Vec::new(),
        });
        self.recv_count += 1;
        id
    }

    pub fn mark_read(&mut self, msg_id: u64) {
        if let Some(m) = self.messages.iter_mut().find(|m| m.id == msg_id) {
            m.is_read = true;
        }
    }

    pub fn delete(&mut self, msg_id: u64) -> bool {
        let before = self.messages.len();
        self.messages.retain(|m| m.id != msg_id);
        self.messages.len() < before
    }

    pub fn thread_messages(&self, thread_id: u64) -> Vec<&Message> {
        self.messages.iter().filter(|m| m.thread_id == thread_id).collect()
    }

    pub fn unread_count(&self) -> usize { self.messages.iter().filter(|m| !m.is_read).count() }
    pub fn total_sent(&self)   -> u64   { self.sent_count }
    pub fn total_received(&self) -> u64 { self.recv_count }
}

impl Default for SmsManager { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn send_receive_flow() {
        let mut mgr = SmsManager::new();
        mgr.send_sms("+1555001", "hello", 1000);
        let id = mgr.receive_sms("+1555001", "hi back", 2000);
        assert_eq!(mgr.unread_count(), 1);
        mgr.mark_read(id);
        assert_eq!(mgr.unread_count(), 0);
    }
}
