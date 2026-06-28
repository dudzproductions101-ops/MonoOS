//! media_session.rs – MonoOS Framework Media Session
//!
//! A MediaSession represents a single media playback context (music player,
//! video player, podcast).  It is registered with the MediaSessionManager
//! so that the system can route media keys, show the lock screen player,
//! and dispatch Bluetooth AVRCP commands.

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    None, Stopped, Paused, Playing, Buffering, Error,
    FastForwarding, Rewinding, Skipping,
}

#[derive(Debug, Clone)]
pub struct MediaMetadata {
    pub title:       String,
    pub artist:      String,
    pub album:       String,
    pub duration_ms: u64,
    pub art_uri:     Option<String>,
    pub track_num:   Option<u32>,
}

impl Default for MediaMetadata {
    fn default() -> Self {
        MediaMetadata {
            title: String::new(), artist: String::new(),
            album: String::new(), duration_ms: 0,
            art_uri: None, track_num: None,
        }
    }
}

pub trait MediaController: Send + Sync {
    fn play(&self);
    fn pause(&self);
    fn stop(&self);
    fn skip_next(&self);
    fn skip_prev(&self);
    fn seek_to(&self, position_ms: u64);
    fn set_volume(&self, volume: f32); // 0.0–1.0
}

pub struct MediaSession {
    pub tag:       String,
    pub package:   String,
    state:         PlaybackState,
    metadata:      MediaMetadata,
    position_ms:   u64,
    volume:        f32,
    controller:    Option<Arc<dyn MediaController>>,
    callbacks:     Vec<Box<dyn Fn(&MediaSession) + Send>>,
}

impl MediaSession {
    pub fn new(tag: impl Into<String>, package: impl Into<String>) -> Self {
        MediaSession {
            tag: tag.into(), package: package.into(),
            state: PlaybackState::None,
            metadata: MediaMetadata::default(),
            position_ms: 0, volume: 1.0,
            controller: None, callbacks: Vec::new(),
        }
    }

    pub fn set_controller(&mut self, ctrl: Arc<dyn MediaController>) {
        self.controller = Some(ctrl);
    }

    pub fn set_state(&mut self, state: PlaybackState) {
        self.state = state;
        self.notify();
    }

    pub fn set_metadata(&mut self, meta: MediaMetadata) {
        self.metadata = meta;
        self.notify();
    }

    pub fn set_position(&mut self, pos_ms: u64) { self.position_ms = pos_ms; }
    pub fn state(&self)      -> PlaybackState  { self.state }
    pub fn metadata(&self)   -> &MediaMetadata { &self.metadata }
    pub fn position_ms(&self) -> u64           { self.position_ms }
    pub fn volume(&self)     -> f32            { self.volume }

    /// Dispatch a media key event (from headset or lock screen).
    pub fn dispatch_key(&self, key: MediaKey) {
        let ctrl = match self.controller.as_ref() { Some(c) => c, None => return };
        match key {
            MediaKey::Play       => ctrl.play(),
            MediaKey::Pause      => ctrl.pause(),
            MediaKey::PlayPause  => if self.state == PlaybackState::Playing {
                ctrl.pause() } else { ctrl.play() },
            MediaKey::Stop       => ctrl.stop(),
            MediaKey::SkipNext   => ctrl.skip_next(),
            MediaKey::SkipPrev   => ctrl.skip_prev(),
            MediaKey::VolumeUp   => ctrl.set_volume((self.volume + 0.1).min(1.0)),
            MediaKey::VolumeDown => ctrl.set_volume((self.volume - 0.1).max(0.0)),
        }
    }

    pub fn on_state_change(&mut self, cb: impl Fn(&MediaSession) + Send + 'static) {
        self.callbacks.push(Box::new(cb));
    }

    fn notify(&self) {
        for cb in &self.callbacks { cb(self); }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MediaKey {
    Play, Pause, PlayPause, Stop, SkipNext, SkipPrev, VolumeUp, VolumeDown,
}

/// Global registry of active media sessions.
pub struct MediaSessionManager {
    sessions: Vec<Arc<Mutex<MediaSession>>>,
}

impl MediaSessionManager {
    pub fn new() -> Self { MediaSessionManager { sessions: Vec::new() } }

    pub fn register(&mut self, session: Arc<Mutex<MediaSession>>) {
        self.sessions.push(session);
    }

    pub fn unregister(&mut self, tag: &str) {
        self.sessions.retain(|s| s.lock().map(|s| s.tag != tag).unwrap_or(true));
    }

    /// Return the session that has focus (currently playing, or most recent).
    pub fn active_session(&self) -> Option<Arc<Mutex<MediaSession>>> {
        self.sessions.iter()
            .find(|s| s.lock().map(|s| s.state() == PlaybackState::Playing).unwrap_or(false))
            .or_else(|| self.sessions.last())
            .cloned()
    }

    pub fn dispatch_global_key(&self, key: MediaKey) {
        if let Some(s) = self.active_session() {
            if let Ok(sess) = s.lock() { sess.dispatch_key(key); }
        }
    }
}

impl Default for MediaSessionManager { fn default() -> Self { Self::new() } }
