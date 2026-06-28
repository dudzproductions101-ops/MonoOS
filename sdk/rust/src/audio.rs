//! audio.rs – Safe wrapper around `monoos_audio.h`.

use crate::result::{check, MonoOsResult};
use crate::sys;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioUsage {
    Media,
    Notification,
    Ringtone,
    VoiceCall,
    Alarm,
    Game,
}

impl From<AudioUsage> for sys::MonoOS_AudioUsage {
    fn from(u: AudioUsage) -> Self {
        match u {
            AudioUsage::Media => sys::MonoOS_AudioUsage::Media,
            AudioUsage::Notification => sys::MonoOS_AudioUsage::Notification,
            AudioUsage::Ringtone => sys::MonoOS_AudioUsage::Ringtone,
            AudioUsage::VoiceCall => sys::MonoOS_AudioUsage::VoiceCall,
            AudioUsage::Alarm => sys::MonoOS_AudioUsage::Alarm,
            AudioUsage::Game => sys::MonoOS_AudioUsage::Game,
        }
    }
}

/// An open PCM output stream. Closed automatically on drop.
pub struct AudioStream {
    handle: sys::MonoOS_AudioStreamHandle,
}

impl AudioStream {
    /// Open a PCM output stream.
    ///
    /// `volume` is clamped to `[0.0, 1.0]`.
    pub fn open(sample_rate: u32, channels: u32, usage: AudioUsage, volume: f32) -> Option<Self> {
        let handle = unsafe {
            sys::monoos_audio_open_stream(sample_rate, channels, usage.into(), volume.clamp(0.0, 1.0))
        };
        if handle == sys::MONOOS_AUDIO_INVALID_HANDLE {
            None
        } else {
            Some(AudioStream { handle })
        }
    }

    /// Write interleaved float PCM samples (`[-1.0, 1.0]`). `frames` must
    /// already be interleaved per the stream's channel count (e.g. for a
    /// stereo stream, `frames.len()` is `2 * frame_count`). Returns the
    /// number of frames actually accepted.
    pub fn write(&mut self, frames: &[f32]) -> MonoOsResult<u32> {
        let n_frames = frames.len() as u32;
        let ret = unsafe { sys::monoos_audio_write(self.handle, frames.as_ptr(), n_frames) };
        if ret < 0 {
            Err(crate::result::MonoOsError::from_code(ret))
        } else {
            Ok(ret as u32)
        }
    }

    /// Set this stream's volume (`[0.0, 1.0]`).
    pub fn set_volume(&mut self, volume: f32) -> MonoOsResult<()> {
        check(unsafe { sys::monoos_audio_set_volume(self.handle, volume.clamp(0.0, 1.0)) })
    }
}

impl Drop for AudioStream {
    fn drop(&mut self) {
        unsafe { sys::monoos_audio_close_stream(self.handle) };
    }
}

/// Set the system master volume. Requires the media-control permission.
pub fn set_master_volume(volume: f32) -> MonoOsResult<()> {
    check(unsafe { sys::monoos_audio_set_master_volume(volume.clamp(0.0, 1.0)) })
}
