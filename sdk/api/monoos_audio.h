/**
 * monoos_audio.h – MonoOS Audio Playback & Recording API
 */

#pragma once
#ifndef MONOOS_AUDIO_H
#define MONOOS_AUDIO_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Audio usage ─────────────────────────────────────────────────────────── */
typedef enum {
    MONOOS_AUDIO_USAGE_MEDIA        = 0,
    MONOOS_AUDIO_USAGE_NOTIFICATION = 1,
    MONOOS_AUDIO_USAGE_RINGTONE     = 2,
    MONOOS_AUDIO_USAGE_VOICE_CALL   = 3,
    MONOOS_AUDIO_USAGE_ALARM        = 4,
    MONOOS_AUDIO_USAGE_GAME         = 5,
} MonoOS_AudioUsage;

/* ── Stream handle ───────────────────────────────────────────────────────── */
typedef uint32_t MonoOS_AudioStreamHandle;
#define MONOOS_AUDIO_INVALID_HANDLE  0U

/**
 * Open a PCM output stream.
 *
 * @param sample_rate  Samples per second (e.g. 48000).
 * @param channels     1 = mono, 2 = stereo.
 * @param usage        Routing / ducking policy.
 * @param volume       Initial volume 0.0–1.0.
 * @return Stream handle, or MONOOS_AUDIO_INVALID_HANDLE on failure.
 */
MonoOS_AudioStreamHandle monoos_audio_open_stream(uint32_t        sample_rate,
                                                  uint32_t        channels,
                                                  MonoOS_AudioUsage usage,
                                                  float            volume);

/**
 * Write interleaved 32-bit float PCM samples to the stream.
 *
 * @param handle  Stream returned by monoos_audio_open_stream.
 * @param data    Interleaved float samples in [-1, +1].
 * @param frames  Number of sample frames (not bytes, not total samples).
 * @return Number of frames accepted, or negative error code.
 */
int monoos_audio_write(MonoOS_AudioStreamHandle handle,
                       const float            *data,
                       uint32_t                frames);

/** Set the volume of an open stream (0.0 = silent, 1.0 = full). */
int monoos_audio_set_volume(MonoOS_AudioStreamHandle handle, float volume);

/** Close an open audio stream. */
void monoos_audio_close_stream(MonoOS_AudioStreamHandle handle);

/** Set the system master volume (0.0–1.0). Requires MONOOS_PERM_MEDIA_CONTROL. */
int monoos_audio_set_master_volume(float volume);

#ifdef __cplusplus
}
#endif
#endif /* MONOOS_AUDIO_H */
