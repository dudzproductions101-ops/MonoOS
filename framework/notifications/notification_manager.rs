//! notification_manager.rs – MonoOS Notification Manager
//!
//! Handles posting, updating, cancelling, and ranking notifications.
//! Notifications are dispatched to the SystemUI status-bar service and
//! optionally routed to Bluetooth accessories or wearables.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority { Min = 0, Low = 1, Default = 2, High = 3, Max = 4 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility { Public, Private, Secret }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Alarm, Call, Email, Error, Event, Message,
    Navigation, Progress, Promo, Recommendation,
    Service, Social, Status, System, Transport,
}

/// An action button shown in the expanded notification shade.
#[derive(Debug, Clone)]
pub struct NotificationAction {
    pub label:       String,
    pub intent:      String,   // serialised PendingIntent descriptor
    pub remote_input: bool,    // true if this action accepts text input inline
}

/// A single notification.
#[derive(Debug, Clone)]
pub struct Notification {
    pub id:              i32,
    pub package:         String,
    pub channel_id:      String,
    pub title:           String,
    pub body:            String,
    pub ticker:          String,
    pub small_icon:      String,   // resource path
    pub large_icon:      Option<String>,
    pub priority:        Priority,
    pub visibility:      Visibility,
    pub category:        Option<Category>,
    pub actions:         Vec<NotificationAction>,
    pub auto_cancel:     bool,
    pub ongoing:         bool,
    pub posted_at_ms:    u64,
    pub group_key:       Option<String>,
}

impl Notification {
    pub fn new(id: i32, package: impl Into<String>, channel_id: impl Into<String>) -> Self {
        Notification {
            id,
            package: package.into(),
            channel_id: channel_id.into(),
            title: String::new(),
            body:  String::new(),
            ticker: String::new(),
            small_icon: "ic_notification_default".into(),
            large_icon: None,
            priority: Priority::Default,
            visibility: Visibility::Private,
            category: None,
            actions: Vec::new(),
            auto_cancel: true,
            ongoing: false,
            posted_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            group_key: None,
        }
    }
}

/// A notification channel — groups related notifications with shared settings.
#[derive(Debug, Clone)]
pub struct NotificationChannel {
    pub id:               String,
    pub name:             String,
    pub importance:       Priority,
    pub description:      String,
    pub show_badge:       bool,
    pub vibrate:          bool,
    pub light:            bool,
    pub sound_uri:        Option<String>,
    pub user_locked:      bool,   // user has overridden channel settings
}

impl NotificationChannel {
    pub fn new(id: impl Into<String>, name: impl Into<String>, importance: Priority) -> Self {
        NotificationChannel {
            id: id.into(),
            name: name.into(),
            importance,
            description: String::new(),
            show_badge: true,
            vibrate: importance >= Priority::Default,
            light:   importance >= Priority::High,
            sound_uri: None,
            user_locked: false,
        }
    }
}

/// The notification manager — one instance per user, owned by system_server.
pub struct NotificationManager {
    /// Active notifications: (package, id) → Notification
    active:   HashMap<(String, i32), Notification>,
    /// Per-package channels
    channels: HashMap<(String, String), NotificationChannel>,
    /// Packages that have been silenced by the user.
    silenced: std::collections::HashSet<String>,
    /// Total notifications posted since boot.
    total_posted:    u64,
    total_cancelled: u64,
}

impl NotificationManager {
    pub fn new() -> Self {
        NotificationManager {
            active:          HashMap::new(),
            channels:        HashMap::new(),
            silenced:        Default::default(),
            total_posted:    0,
            total_cancelled: 0,
        }
    }

    /// Register a notification channel for a package.
    pub fn create_channel(&mut self, package: &str, channel: NotificationChannel) {
        self.channels.insert((package.to_owned(), channel.id.clone()), channel);
    }

    /// Post or update a notification.  Returns Err if the package is silenced
    /// or the channel does not exist.
    pub fn notify(&mut self, notif: Notification) -> Result<(), &'static str> {
        if self.silenced.contains(&notif.package) {
            return Err("package is silenced");
        }
        let chan_key = (notif.package.clone(), notif.channel_id.clone());
        if !self.channels.contains_key(&chan_key) {
            return Err("channel not registered");
        }
        self.active.insert((notif.package.clone(), notif.id), notif);
        self.total_posted += 1;
        Ok(())
    }

    /// Cancel a notification by package + id.
    pub fn cancel(&mut self, package: &str, id: i32) -> bool {
        let removed = self.active.remove(&(package.to_owned(), id)).is_some();
        if removed { self.total_cancelled += 1; }
        removed
    }

    /// Cancel all notifications for a package.
    pub fn cancel_all(&mut self, package: &str) {
        let keys: Vec<_> = self.active.keys()
            .filter(|(p, _)| p == package)
            .cloned()
            .collect();
        self.total_cancelled += keys.len() as u64;
        for k in keys { self.active.remove(&k); }
    }

    /// Silence a package (no notifications shown until un-silenced).
    pub fn silence_package(&mut self, package: &str) {
        self.silenced.insert(package.to_owned());
        self.cancel_all(package);
    }

    pub fn unsilence_package(&mut self, package: &str) {
        self.silenced.remove(package);
    }

    /// Ranked list of all active notifications (highest priority first).
    pub fn ranked_notifications(&self) -> Vec<&Notification> {
        let mut notifs: Vec<&Notification> = self.active.values().collect();
        notifs.sort_by(|a, b| b.priority.cmp(&a.priority)
            .then(b.posted_at_ms.cmp(&a.posted_at_ms)));
        notifs
    }

    pub fn active_count(&self) -> usize { self.active.len() }
}

impl Default for NotificationManager { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mgr() -> NotificationManager {
        let mut mgr = NotificationManager::new();
        mgr.create_channel("com.app", NotificationChannel::new("msgs", "Messages", Priority::High));
        mgr
    }

    #[test]
    fn post_and_cancel() {
        let mut mgr = make_mgr();
        let n = Notification::new(1, "com.app", "msgs");
        mgr.notify(n).unwrap();
        assert_eq!(mgr.active_count(), 1);
        assert!(mgr.cancel("com.app", 1));
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn silenced_package_blocked() {
        let mut mgr = make_mgr();
        mgr.silence_package("com.app");
        let n = Notification::new(1, "com.app", "msgs");
        assert!(mgr.notify(n).is_err());
    }

    #[test]
    fn no_channel_blocked() {
        let mut mgr = NotificationManager::new();
        let n = Notification::new(1, "com.app", "missing_channel");
        assert!(mgr.notify(n).is_err());
    }
}
