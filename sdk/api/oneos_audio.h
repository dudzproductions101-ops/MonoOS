/**
 * oneos_audio.h – OneOS Audio Playback & Recording API
 */

#pragma once
#ifndef ONEOS_AUDIO_H
#define ONEOS_AUDIO_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Audio usage ─────────────────────────────────────────────────────────── */
typedef enum {
    ONEOS_AUDIO_USAGE_MEDIA        = 0,
    ONEOS_AUDIO_USAGE_NOTIFICATION = 1,
    ONEOS_AUDIO_USAGE_RINGTONE     = 2,
    ONEOS_AUDIO_USAGE_VOICE_CALL   = 3,
    ONEOS_AUDIO_USAGE_ALARM        = 4,
    ONEOS_AUDIO_USAGE_GAME         = 5,
} OneoS_AudioUsage;

/* ── Stream handle ───────────────────────────────────────────────────────── */
typedef uint32_t OneoS_AudioStreamHandle;
#define ONEOS_AUDIO_INVALID_HANDLE  0U

/**
 * Open a PCM output stream.
 *
 * @param sample_rate  Samples per second (e.g. 48000).
 * @param channels     1 = mono, 2 = stereo.
 * @param usage        Routing / ducking policy.
 * @param volume       Initial volume 0.0–1.0.
 * @return Stream handle, or ONEOS_AUDIO_INVALID_HANDLE on failure.
 */
OneoS_AudioStreamHandle oneos_audio_open_stream(uint32_t        sample_rate,
                                                  uint32_t        channels,
                                                  OneoS_AudioUsage usage,
                                                  float            volume);

/**
 * Write interleaved 32-bit float PCM samples to the stream.
 *
 * @param handle  Stream returned by oneos_audio_open_stream.
 * @param data    Interleaved float samples in [-1, +1].
 * @param frames  Number of sample frames (not bytes, not total samples).
 * @return Number of frames accepted, or negative error code.
 */
int oneos_audio_write(OneoS_AudioStreamHandle handle,
                       const float            *data,
                       uint32_t                frames);

/** Set the volume of an open stream (0.0 = silent, 1.0 = full). */
int oneos_audio_set_volume(OneoS_AudioStreamHandle handle, float volume);

/** Close an open audio stream. */
void oneos_audio_close_stream(OneoS_AudioStreamHandle handle);

/** Set the system master volume (0.0–1.0). Requires ONEOS_PERM_MEDIA_CONTROL. */
int oneos_audio_set_master_volume(float volume);

#ifdef __cplusplus
}
#endif
#endif /* ONEOS_AUDIO_H */
