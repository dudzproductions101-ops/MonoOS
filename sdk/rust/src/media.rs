//! media.rs – Safe wrapper around `monoos_media.h`.

use crate::result::{check, MonoOsResult};
use crate::sys;
use std::ffi::CString;
use std::os::raw::c_void;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Idle,
    Prepared,
    Started,
    Paused,
    Stopped,
    Complete,
    Error,
}

impl From<sys::MonoOS_PlayerState> for PlayerState {
    fn from(s: sys::MonoOS_PlayerState) -> Self {
        match s {
            sys::MonoOS_PlayerState::Idle => PlayerState::Idle,
            sys::MonoOS_PlayerState::Prepared => PlayerState::Prepared,
            sys::MonoOS_PlayerState::Started => PlayerState::Started,
            sys::MonoOS_PlayerState::Paused => PlayerState::Paused,
            sys::MonoOS_PlayerState::Stopped => PlayerState::Stopped,
            sys::MonoOS_PlayerState::Complete => PlayerState::Complete,
            sys::MonoOS_PlayerState::Error => PlayerState::Error,
        }
    }
}

/// Listener callbacks for player events. Each field is optional.
#[derive(Default)]
pub struct PlayerListener {
    pub on_state: Option<Box<dyn Fn(PlayerState)>>,
    pub on_position: Option<Box<dyn Fn(u64)>>,
    pub on_error: Option<Box<dyn Fn(i32, &str)>>,
    pub on_complete: Option<Box<dyn Fn()>>,
}

/// A media player instance. Destroyed automatically on drop.
pub struct Player {
    raw: *mut sys::MonoOS_Player,
    // Keeps the listener (and its trampoline context) alive for as long as
    // the player exists; the C side holds a raw pointer into this box.
    _listener_ctx: Option<Box<PlayerListener>>,
}

// See Context's safety note: no documented thread-affinity in monoos_media.h.
unsafe impl Send for Player {}

impl Player {
    pub fn new() -> Self {
        let raw = unsafe { sys::monoos_player_create() };
        Player { raw, _listener_ctx: None }
    }

    pub fn set_uri(&mut self, uri: &str) -> MonoOsResult<()> {
        let c = CString::new(uri).map_err(|_| crate::result::MonoOsError::InvalidArg)?;
        check(unsafe { sys::monoos_player_set_uri(self.raw, c.as_ptr()) })
    }

    pub fn prepare(&mut self) -> MonoOsResult<()> {
        check(unsafe { sys::monoos_player_prepare(self.raw) })
    }

    pub fn start(&mut self) -> MonoOsResult<()> {
        check(unsafe { sys::monoos_player_start(self.raw) })
    }

    pub fn pause(&mut self) -> MonoOsResult<()> {
        check(unsafe { sys::monoos_player_pause(self.raw) })
    }

    pub fn stop(&mut self) -> MonoOsResult<()> {
        check(unsafe { sys::monoos_player_stop(self.raw) })
    }

    pub fn seek(&mut self, pos_ms: u64) -> MonoOsResult<()> {
        check(unsafe { sys::monoos_player_seek(self.raw, pos_ms) })
    }

    pub fn set_volume(&mut self, volume: f32) -> MonoOsResult<()> {
        check(unsafe { sys::monoos_player_set_volume(self.raw, volume.clamp(0.0, 1.0)) })
    }

    pub fn set_rate(&mut self, rate: f32) -> MonoOsResult<()> {
        check(unsafe { sys::monoos_player_set_rate(self.raw, rate) })
    }

    pub fn set_looping(&mut self, looping: bool) {
        unsafe { sys::monoos_player_set_looping(self.raw, looping) }
    }

    pub fn position(&self) -> u64 {
        unsafe { sys::monoos_player_position(self.raw) }
    }

    pub fn duration(&self) -> u64 {
        unsafe { sys::monoos_player_duration(self.raw) }
    }

    pub fn state(&self) -> PlayerState {
        unsafe { sys::monoos_player_state(self.raw) }.into()
    }

    /// Register event listener callbacks. Replaces any previously set
    /// listener. Pass `PlayerListener::default()` (all `None`) to clear it.
    pub fn set_listener(&mut self, listener: PlayerListener) {
        let boxed = Box::new(listener);
        let ctx_ptr = boxed.as_ref() as *const PlayerListener as *mut c_void;
        self._listener_ctx = Some(boxed);

        let raw_listener = sys::MonoOS_PlayerListener {
            on_state: Some(on_state_trampoline),
            on_position: Some(on_position_trampoline),
            on_error: Some(on_error_trampoline),
            on_complete: Some(on_complete_trampoline),
        };
        unsafe { sys::monoos_player_set_listener(self.raw, &raw_listener, ctx_ptr) };
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        unsafe { sys::monoos_player_destroy(self.raw) };
    }
}

extern "C" fn on_state_trampoline(_player: *mut sys::MonoOS_Player, state: sys::MonoOS_PlayerState, user: *mut c_void) {
    if user.is_null() {
        return;
    }
    let listener = unsafe { &*(user as *const PlayerListener) };
    if let Some(cb) = &listener.on_state {
        cb(state.into());
    }
}

extern "C" fn on_position_trampoline(_player: *mut sys::MonoOS_Player, pos_ms: u64, user: *mut c_void) {
    if user.is_null() {
        return;
    }
    let listener = unsafe { &*(user as *const PlayerListener) };
    if let Some(cb) = &listener.on_position {
        cb(pos_ms);
    }
}

extern "C" fn on_error_trampoline(
    _player: *mut sys::MonoOS_Player,
    code: std::os::raw::c_int,
    msg: *const std::os::raw::c_char,
    user: *mut c_void,
) {
    if user.is_null() {
        return;
    }
    let listener = unsafe { &*(user as *const PlayerListener) };
    if let Some(cb) = &listener.on_error {
        let msg_str = if msg.is_null() {
            String::new()
        } else {
            unsafe { std::ffi::CStr::from_ptr(msg) }.to_string_lossy().into_owned()
        };
        cb(code, &msg_str);
    }
}

extern "C" fn on_complete_trampoline(_player: *mut sys::MonoOS_Player, user: *mut c_void) {
    if user.is_null() {
        return;
    }
    let listener = unsafe { &*(user as *const PlayerListener) };
    if let Some(cb) = &listener.on_complete {
        cb();
    }
}
