/**
 * monoos_media.h – MonoOS Media Playback API
 *
 * High-level wrapper around the GStreamer-based media player engine.
 */

#pragma once
#ifndef MONOOS_MEDIA_H
#define MONOOS_MEDIA_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Player handle ───────────────────────────────────────────────────────── */
typedef struct MonoOS_Player MonoOS_Player;

/* ── Player state ────────────────────────────────────────────────────────── */
typedef enum {
    MONOOS_PLAYER_IDLE       = 0,
    MONOOS_PLAYER_PREPARED   = 1,
    MONOOS_PLAYER_STARTED    = 2,
    MONOOS_PLAYER_PAUSED     = 3,
    MONOOS_PLAYER_STOPPED    = 4,
    MONOOS_PLAYER_COMPLETE   = 5,
    MONOOS_PLAYER_ERROR      = 6,
} MonoOS_PlayerState;

/* ── Listener callbacks ──────────────────────────────────────────────────── */
typedef struct {
    /** Invoked when the player transitions to a new state. */
    void (*on_state)(MonoOS_Player *player, MonoOS_PlayerState state, void *user);
    /** Invoked periodically with the current playback position in ms. */
    void (*on_position)(MonoOS_Player *player, uint64_t pos_ms, void *user);
    /** Invoked on an unrecoverable error. */
    void (*on_error)(MonoOS_Player *player, int code, const char *msg, void *user);
    /** Invoked when playback reaches the end of the media. */
    void (*on_complete)(MonoOS_Player *player, void *user);
} MonoOS_PlayerListener;

/* ── API ─────────────────────────────────────────────────────────────────── */

/** Create a new media player instance. */
MonoOS_Player *monoos_player_create(void);

/** Destroy a player instance and release all resources. */
void monoos_player_destroy(MonoOS_Player *player);

/**
 * Set the data source URI.
 * Supported schemes: file://, http://, https://, content://
 * Must be called when the player is in IDLE or STOPPED state.
 */
int monoos_player_set_uri(MonoOS_Player *player, const char *uri);

/**
 * Prepare the player (buffering, codec init).
 * Call before start().  On completion, the listener's on_state callback
 * is called with MONOOS_PLAYER_PREPARED.
 */
int monoos_player_prepare(MonoOS_Player *player);

/** Start or resume playback. */
int monoos_player_start(MonoOS_Player *player);

/** Pause playback. */
int monoos_player_pause(MonoOS_Player *player);

/** Stop playback and reset position to 0. */
int monoos_player_stop(MonoOS_Player *player);

/**
 * Seek to a position.
 * @param pos_ms  Target position in milliseconds.
 */
int monoos_player_seek(MonoOS_Player *player, uint64_t pos_ms);

/** Set playback volume (0.0 = silent, 1.0 = full). */
int monoos_player_set_volume(MonoOS_Player *player, float volume);

/** Set playback rate (1.0 = normal, 2.0 = double speed). */
int monoos_player_set_rate(MonoOS_Player *player, float rate);

/** Enable looping (restarts automatically on completion). */
void monoos_player_set_looping(MonoOS_Player *player, bool loop);

/** Get current position in milliseconds. */
uint64_t monoos_player_position(const MonoOS_Player *player);

/** Get media duration in milliseconds (0 if unknown). */
uint64_t monoos_player_duration(const MonoOS_Player *player);

/** Get current player state. */
MonoOS_PlayerState monoos_player_state(const MonoOS_Player *player);

/**
 * Register event listener callbacks.
 * @param listener   Struct of callbacks; any field may be NULL.
 * @param user_data  Forwarded unchanged to every callback.
 */
void monoos_player_set_listener(MonoOS_Player               *player,
                                const MonoOS_PlayerListener *listener,
                                void                       *user_data);

#ifdef __cplusplus
}
#endif
#endif /* MONOOS_MEDIA_H */
