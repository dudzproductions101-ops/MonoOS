/**
 * monoos_notifications.h – MonoOS Notification API
 *
 * Apps must create at least one notification channel before posting
 * notifications (requires MONOOS_PERM_NOTIFICATIONS).
 */

#pragma once
#ifndef MONOOS_NOTIFICATIONS_H
#define MONOOS_NOTIFICATIONS_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Priority ────────────────────────────────────────────────────────────── */
typedef enum {
    MONOOS_NOTIF_PRIORITY_MIN     = 0,
    MONOOS_NOTIF_PRIORITY_LOW     = 1,
    MONOOS_NOTIF_PRIORITY_DEFAULT = 2,
    MONOOS_NOTIF_PRIORITY_HIGH    = 3,
    MONOOS_NOTIF_PRIORITY_MAX     = 4,
} MonoOS_NotifPriority;

/* ── Channel ─────────────────────────────────────────────────────────────── */
typedef struct {
    char                 id[64];
    char                 name[128];
    char                 description[256];
    MonoOS_NotifPriority  importance;
    bool                 vibrate;
    bool                 show_badge;
} MonoOS_NotifChannel;

/**
 * Create or update a notification channel.
 * This is a no-op if a channel with the same id already exists and
 * the user has not locked its settings.
 *
 * @return MONOOS_OK on success.
 */
int monoos_notif_create_channel(const MonoOS_NotifChannel *channel);

/** Delete a notification channel and all its active notifications. */
int monoos_notif_delete_channel(const char *channel_id);

/* ── Notification builder ────────────────────────────────────────────────── */
typedef struct {
    int32_t              id;             /**< Unique id within this app.   */
    char                 channel_id[64]; /**< Must match a created channel. */
    char                 title[256];
    char                 body[512];
    char                 ticker[256];    /**< Accessibility text.           */
    MonoOS_NotifPriority  priority;
    bool                 auto_cancel;    /**< Dismiss on tap.               */
    bool                 ongoing;        /**< Cannot be swiped away.        */
    uint32_t             badge_count;    /**< App-icon badge number.        */
} MonoOS_Notification;

/**
 * Post or update a notification.
 *
 * @param notif  Fully-populated notification descriptor.
 * @return MONOOS_OK, MONOOS_ERROR_PERMISSION_DENIED, or MONOOS_ERROR_NOT_FOUND
 *         (if the channel does not exist).
 */
int monoos_notif_post(const MonoOS_Notification *notif);

/** Cancel a notification previously posted by this app. */
int monoos_notif_cancel(int32_t notif_id);

/** Cancel all notifications posted by this app. */
void monoos_notif_cancel_all(void);

#ifdef __cplusplus
}
#endif
#endif /* MONOOS_NOTIFICATIONS_H */
