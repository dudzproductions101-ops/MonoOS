//! media_player.rs – MonoOS Media Player
//!
//! A GStreamer-backed media player that handles local file playback,
//! HTTP progressive streaming, and HLS/DASH adaptive streaming.
//! Exposes a simple Rust API consumed by the multimedia framework.

use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Idle,
    Initialised,
    Prepared,
    Started,
    Paused,
    Stopped,
    PlaybackComplete,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekMode { Closest, Previous, Next }

#[derive(Debug, Clone)]
pub struct MediaInfo {
    pub uri:         String,
    pub duration:    Duration,
    pub width:       u32,
    pub height:      u32,
    pub has_video:   bool,
    pub has_audio:   bool,
    pub title:       Option<String>,
    pub artist:      Option<String>,
}

pub trait PlayerListener: Send {
    fn on_state_changed(&self, old: PlayerState, new: PlayerState);
    fn on_position_changed(&self, pos_ms: u64);
    fn on_error(&self, code: i32, msg: &str);
    fn on_prepared(&self, info: &MediaInfo);
    fn on_completion(&self);
}

pub struct MediaPlayer {
    state:      PlayerState,
    info:       Option<MediaInfo>,
    position:   u64,    // milliseconds
    volume:     f32,
    looping:    bool,
    playback_rate: f32,
    listeners:  Vec<Box<dyn PlayerListener>>,
}

impl MediaPlayer {
    pub fn new() -> Self {
        MediaPlayer {
            state: PlayerState::Idle,
            info: None,
            position: 0,
            volume: 1.0,
            looping: false,
            playback_rate: 1.0,
            listeners: Vec::new(),
        }
    }

    pub fn add_listener(&mut self, l: Box<dyn PlayerListener>) {
        self.listeners.push(l);
    }

    fn transition(&mut self, new: PlayerState) {
        let old = self.state;
        self.state = new;
        for l in &self.listeners { l.on_state_changed(old, new); }
    }

    /// Set the data source URI (file:// or http://).
    pub fn set_data_source(&mut self, uri: impl Into<String>) -> Result<(), &'static str> {
        if !matches!(self.state, PlayerState::Idle | PlayerState::Stopped) {
            return Err("must be in Idle or Stopped state");
        }
        // Real impl: pass URI to GStreamer pipeline via gst_element_set_state.
        let uri = uri.into();
        self.info = Some(MediaInfo {
            uri,
            duration:  Duration::from_secs(0), // populated on prepare()
            width: 0, height: 0, has_video: false, has_audio: true,
            title: None, artist: None,
        });
        self.transition(PlayerState::Initialised);
        Ok(())
    }

    /// Prepare (buffer / decode first frames). Async in production.
    pub fn prepare(&mut self) -> Result<(), &'static str> {
        if self.state != PlayerState::Initialised { return Err("not initialised"); }
        // Real: gst_element_set_state(pipeline, GST_STATE_PAUSED) and wait.
        if let Some(info) = &mut self.info {
            info.duration     = Duration::from_secs(180);  // stub
            info.has_video    = info.uri.ends_with(".mp4") || info.uri.ends_with(".mkv");
            info.has_audio    = true;
            info.width        = if info.has_video { 1920 } else { 0 };
            info.height       = if info.has_video { 1080 } else { 0 };
            let info_clone = info.clone();
            for l in &self.listeners { l.on_prepared(&info_clone); }
        }
        self.transition(PlayerState::Prepared);
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), &'static str> {
        if !matches!(self.state, PlayerState::Prepared | PlayerState::Paused) {
            return Err("not prepared or paused");
        }
        // Real: gst_element_set_state(pipeline, GST_STATE_PLAYING)
        self.transition(PlayerState::Started);
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), &'static str> {
        if self.state != PlayerState::Started { return Err("not started"); }
        self.transition(PlayerState::Paused);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), &'static str> {
        if matches!(self.state, PlayerState::Idle | PlayerState::Stopped) { return Ok(()); }
        // Real: gst_element_set_state(pipeline, GST_STATE_READY)
        self.position = 0;
        self.transition(PlayerState::Stopped);
        Ok(())
    }

    pub fn release(&mut self) {
        let _ = self.stop();
        self.info = None;
        self.listeners.clear();
        self.state = PlayerState::Idle;
    }

    pub fn seek_to(&mut self, pos_ms: u64, _mode: SeekMode) -> Result<(), &'static str> {
        if !matches!(self.state, PlayerState::Started | PlayerState::Paused | PlayerState::Prepared) {
            return Err("cannot seek in current state");
        }
        // Real: gst_element_seek_simple(pipeline, GST_FORMAT_TIME, flags, pos_ns)
        let dur = self.info.as_ref().map(|i| i.duration.as_millis() as u64).unwrap_or(0);
        self.position = pos_ms.min(dur);
        for l in &self.listeners { l.on_position_changed(self.position); }
        Ok(())
    }

    pub fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(0.0, 1.0);
        // Real: set GstVolume element property.
    }

    pub fn set_looping(&mut self, v: bool) { self.looping = v; }

    pub fn set_playback_rate(&mut self, rate: f32) -> Result<(), &'static str> {
        if rate <= 0.0 { return Err("rate must be positive"); }
        self.playback_rate = rate;
        // Real: gst_element_seek with rate in the seek event.
        Ok(())
    }

    pub fn current_position(&self) -> u64  { self.position }
    pub fn duration(&self) -> Option<u64>  { self.info.as_ref().map(|i| i.duration.as_millis() as u64) }
    pub fn state(&self) -> PlayerState     { self.state }
    pub fn is_playing(&self) -> bool       { self.state == PlayerState::Started }
    pub fn volume(&self) -> f32            { self.volume }
    pub fn media_info(&self) -> Option<&MediaInfo> { self.info.as_ref() }
}

impl Default for MediaPlayer { fn default() -> Self { Self::new() } }

pub type SharedMediaPlayer = Arc<Mutex<MediaPlayer>>;

pub fn create_player() -> SharedMediaPlayer {
    Arc::new(Mutex::new(MediaPlayer::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_lifecycle() {
        let mut p = MediaPlayer::new();
        assert_eq!(p.state(), PlayerState::Idle);
        p.set_data_source("file:///sdcard/Music/test.mp3").unwrap();
        p.prepare().unwrap();
        p.start().unwrap();
        assert!(p.is_playing());
        p.pause().unwrap();
        assert_eq!(p.state(), PlayerState::Paused);
        p.stop().unwrap();
        assert_eq!(p.state(), PlayerState::Stopped);
    }

    #[test]
    fn seek_clamps_to_duration() {
        let mut p = MediaPlayer::new();
        p.set_data_source("file:///test.mp4").unwrap();
        p.prepare().unwrap();
        p.start().unwrap();
        p.seek_to(999_999_999, SeekMode::Closest).unwrap();
        let dur = p.duration().unwrap();
        assert!(p.current_position() <= dur);
    }

    #[test]
    fn negative_rate_rejected() {
        let mut p = MediaPlayer::new();
        assert!(p.set_playback_rate(-1.0).is_err());
    }
}
