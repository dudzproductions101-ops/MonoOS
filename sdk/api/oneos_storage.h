/**
 * oneos_storage.h – OneOS Scoped Storage API
 *
 * Provides access to an app's private data directory and, with the
 * STORAGE permission, to shared media collections.
 */

#pragma once
#ifndef ONEOS_STORAGE_H
#define ONEOS_STORAGE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Content URI ─────────────────────────────────────────────────────────── */
/** Opaque content URI identifying a file in the media store. */
typedef struct { char uri[256]; } OneoS_ContentUri;

/* ── Media types ─────────────────────────────────────────────────────────── */
typedef enum {
    ONEOS_MEDIA_IMAGE    = 0,
    ONEOS_MEDIA_VIDEO    = 1,
    ONEOS_MEDIA_AUDIO    = 2,
    ONEOS_MEDIA_DOCUMENT = 3,
    ONEOS_MEDIA_OTHER    = 4,
} OneoS_MediaType;

/* ── Media entry ─────────────────────────────────────────────────────────── */
typedef struct {
    OneoS_ContentUri uri;
    char             display_name[256];
    uint64_t         size_bytes;
    OneoS_MediaType  media_type;
    char             mime_type[64];
    uint64_t         date_added;       /**< Unix seconds. */
    uint64_t         date_modified;
    uint32_t         width;            /**< 0 for non-image/video. */
    uint32_t         height;
    uint64_t         duration_ms;      /**< 0 for non-audio/video. */
} OneoS_MediaEntry;

/* ── API: app-private storage ────────────────────────────────────────────── */

/**
 * Get the absolute path to the app's private files directory.
 *
 * The returned string is valid for the lifetime of the context.
 * The directory is created automatically on first access.
 */
const char *oneos_files_dir(void);

/** Get the app's private cache directory path. */
const char *oneos_cache_dir(void);

/** Get the app's private database directory path. */
const char *oneos_db_dir(void);

/* ── API: shared media store (requires ONEOS_PERM_STORAGE) ──────────────── */

typedef void (*OneoS_MediaQueryCallback)(const OneoS_MediaEntry *entry,
                                          void                   *user_data);

/**
 * Query the shared media store.
 *
 * @param type       Which media type to enumerate.
 * @param callback   Called once per matching entry.  May be called on a
 *                   background thread; do not access UI from it.
 * @param user_data  Forwarded to callback.
 * @return ONEOS_OK on success, error code otherwise.
 */
int oneos_media_query(OneoS_MediaType          type,
                       OneoS_MediaQueryCallback callback,
                       void                    *user_data);

/**
 * Insert a file into the shared media store.
 *
 * @param path         Absolute path to the file to insert.
 * @param mime_type    MIME type string (e.g. "image/jpeg").
 * @param out_uri      Populated with the assigned content URI on success.
 * @return ONEOS_OK on success.
 */
int oneos_media_insert(const char       *path,
                        const char       *mime_type,
                        OneoS_ContentUri *out_uri);

/**
 * Delete a media entry from the shared store.
 * Requires the app to own the file or hold MANAGE_MEDIA.
 */
int oneos_media_delete(const OneoS_ContentUri *uri);

#ifdef __cplusplus
}
#endif
#endif /* ONEOS_STORAGE_H */
