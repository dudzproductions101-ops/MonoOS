/**
 * oneos_media.h – OneOS Media Playback API
 *
 * High-level wrapper around the GStreamer-based media player engine.
 */

#pragma once
#ifndef ONEOS_MEDIA_H
#define ONEOS_MEDIA_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Player handle ───────────────────────────────────────────────────────── */
typedef struct OneoS_Player OneoS_Player;

/* ── Player state ────────────────────────────────────────────────────────── */
typedef enum {
    ONEOS_PLAYER_IDLE       = 0,
    ONEOS_PLAYER_PREPARED   = 1,
    ONEOS_PLAYER_STARTED    = 2,
    ONEOS_PLAYER_PAUSED     = 3,
    ONEOS_PLAYER_STOPPED    = 4,
    ONEOS_PLAYER_COMPLETE   = 5,
    ONEOS_PLAYER_ERROR      = 6,
} OneoS_PlayerState;

/* ── Listener callbacks ──────────────────────────────────────────────────── */
typedef struct {
    /** Invoked when the player transitions to a new state. */
    void (*on_state)(OneoS_Player *player, OneoS_PlayerState state, void *user);
    /** Invoked periodically with the current playback position in ms. */
    void (*on_position)(OneoS_Player *player, uint64_t pos_ms, void *user);
    /** Invoked on an unrecoverable error. */
    void (*on_error)(OneoS_Player *player, int code, const char *msg, void *user);
    /** Invoked when playback reaches the end of the media. */
    void (*on_complete)(OneoS_Player *player, void *user);
} OneoS_PlayerListener;

/* ── API ─────────────────────────────────────────────────────────────────── */

/** Create a new media player instance. */
OneoS_Player *oneos_player_create(void);

/** Destroy a player instance and release all resources. */
void oneos_player_destroy(OneoS_Player *player);

/**
 * Set the data source URI.
 * Supported schemes: file://, http://, https://, content://
 * Must be called when the player is in IDLE or STOPPED state.
 */
int oneos_player_set_uri(OneoS_Player *player, const char *uri);

/**
 * Prepare the player (buffering, codec init).
 * Call before start().  On completion, the listener's on_state callback
 * is called with ONEOS_PLAYER_PREPARED.
 */
int oneos_player_prepare(OneoS_Player *player);

/** Start or resume playback. */
int oneos_player_start(OneoS_Player *player);

/** Pause playback. */
int oneos_player_pause(OneoS_Player *player);

/** Stop playback and reset position to 0. */
int oneos_player_stop(OneoS_Player *player);

/**
 * Seek to a position.
 * @param pos_ms  Target position in milliseconds.
 */
int oneos_player_seek(OneoS_Player *player, uint64_t pos_ms);

/** Set playback volume (0.0 = silent, 1.0 = full). */
int oneos_player_set_volume(OneoS_Player *player, float volume);

/** Set playback rate (1.0 = normal, 2.0 = double speed). */
int oneos_player_set_rate(OneoS_Player *player, float rate);

/** Enable looping (restarts automatically on completion). */
void oneos_player_set_looping(OneoS_Player *player, bool loop);

/** Get current position in milliseconds. */
uint64_t oneos_player_position(const OneoS_Player *player);

/** Get media duration in milliseconds (0 if unknown). */
uint64_t oneos_player_duration(const OneoS_Player *player);

/** Get current player state. */
OneoS_PlayerState oneos_player_state(const OneoS_Player *player);

/**
 * Register event listener callbacks.
 * @param listener   Struct of callbacks; any field may be NULL.
 * @param user_data  Forwarded unchanged to every callback.
 */
void oneos_player_set_listener(OneoS_Player               *player,
                                const OneoS_PlayerListener *listener,
                                void                       *user_data);

#ifdef __cplusplus
}
#endif
#endif /* ONEOS_MEDIA_H */
