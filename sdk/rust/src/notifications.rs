//! notifications.rs – Safe wrapper around `monoos_notifications.h`.

use crate::result::{check, MonoOsResult};
use crate::sys;
use std::ffi::CString;
use std::os::raw::c_char;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Min,
    Low,
    Default,
    High,
    Max,
}

impl From<Priority> for sys::MonoOS_NotifPriority {
    fn from(p: Priority) -> Self {
        match p {
            Priority::Min => sys::MonoOS_NotifPriority::Min,
            Priority::Low => sys::MonoOS_NotifPriority::Low,
            Priority::Default => sys::MonoOS_NotifPriority::Default,
            Priority::High => sys::MonoOS_NotifPriority::High,
            Priority::Max => sys::MonoOS_NotifPriority::Max,
        }
    }
}

fn fill_fixed(dst: &mut [c_char], src: &str) -> MonoOsResult<()> {
    let c = CString::new(src).map_err(|_| crate::result::MonoOsError::InvalidArg)?;
    let bytes = c.as_bytes_with_nul();
    if bytes.len() > dst.len() {
        return Err(crate::result::MonoOsError::InvalidArg);
    }
    for (i, b) in bytes.iter().enumerate() {
        dst[i] = *b as c_char;
    }
    Ok(())
}

/// A notification channel. Must be created once before posting any
/// notification that references it.
pub struct Channel {
    pub id: String,
    pub name: String,
    pub description: String,
    pub importance: Priority,
    pub vibrate: bool,
    pub show_badge: bool,
}

impl Channel {
    pub fn create(&self) -> MonoOsResult<()> {
        let mut raw = sys::MonoOS_NotifChannel {
            id: [0; 64],
            name: [0; 128],
            description: [0; 256],
            importance: self.importance.into(),
            vibrate: self.vibrate,
            show_badge: self.show_badge,
        };
        fill_fixed(&mut raw.id, &self.id)?;
        fill_fixed(&mut raw.name, &self.name)?;
        fill_fixed(&mut raw.description, &self.description)?;
        check(unsafe { sys::monoos_notif_create_channel(&raw) })
    }
}

/// Delete a channel and all of its active notifications.
pub fn delete_channel(channel_id: &str) -> MonoOsResult<()> {
    let c = CString::new(channel_id).map_err(|_| crate::result::MonoOsError::InvalidArg)?;
    check(unsafe { sys::monoos_notif_delete_channel(c.as_ptr()) })
}

/// A notification to post or update.
pub struct Notification {
    pub id: i32,
    pub channel_id: String,
    pub title: String,
    pub body: String,
    pub ticker: String,
    pub priority: Priority,
    pub auto_cancel: bool,
    pub ongoing: bool,
    pub badge_count: u32,
}

impl Notification {
    /// Post (or update, if `id` matches an existing notification) this
    /// notification. Requires [`crate::permissions::Permission::Notifications`]
    /// and a previously-created channel matching `channel_id`.
    pub fn post(&self) -> MonoOsResult<()> {
        let mut raw = sys::MonoOS_Notification {
            id: self.id,
            channel_id: [0; 64],
            title: [0; 256],
            body: [0; 512],
            ticker: [0; 256],
            priority: self.priority.into(),
            auto_cancel: self.auto_cancel,
            ongoing: self.ongoing,
            badge_count: self.badge_count,
        };
        fill_fixed(&mut raw.channel_id, &self.channel_id)?;
        fill_fixed(&mut raw.title, &self.title)?;
        fill_fixed(&mut raw.body, &self.body)?;
        fill_fixed(&mut raw.ticker, &self.ticker)?;
        check(unsafe { sys::monoos_notif_post(&raw) })
    }
}

/// Cancel a previously posted notification by id.
pub fn cancel(notif_id: i32) -> MonoOsResult<()> {
    check(unsafe { sys::monoos_notif_cancel(notif_id) })
}

/// Cancel every notification posted by this app.
pub fn cancel_all() {
    unsafe { sys::monoos_notif_cancel_all() }
}
