/**
 * oneos_permissions.h – OneOS Runtime Permission API
 *
 * Apps must declare permissions in their manifest (META-INF/manifest.toml)
 * and then request them at runtime using this API.  The system presents a
 * UI prompt; the user's decision is cached per app.
 */

#pragma once
#ifndef ONEOS_PERMISSIONS_H
#define ONEOS_PERMISSIONS_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Permission identifiers ──────────────────────────────────────────────── */
typedef uint32_t OneoS_Permission;

#define ONEOS_PERM_CAMERA       0x00000001U  /**< Access camera hardware.       */
#define ONEOS_PERM_MICROPHONE   0x00000002U  /**< Record audio.                 */
#define ONEOS_PERM_LOCATION     0x00000004U  /**< Access precise GPS location.  */
#define ONEOS_PERM_CONTACTS     0x00000008U  /**< Read/write contacts.          */
#define ONEOS_PERM_STORAGE      0x00000010U  /**< Read/write shared storage.    */
#define ONEOS_PERM_PHONE        0x00000020U  /**< Make calls, access call log.  */
#define ONEOS_PERM_BLUETOOTH    0x00000040U  /**< Scan and connect Bluetooth.   */
#define ONEOS_PERM_NFC          0x00000080U  /**< Read/write NFC tags.          */
#define ONEOS_PERM_SENSORS      0x00000100U  /**< Access motion/env sensors.    */
#define ONEOS_PERM_NETWORK      0x00000200U  /**< Send/receive network traffic. */
#define ONEOS_PERM_NOTIFICATIONS 0x00000400U /**< Post notifications.           */

/* ── Grant state ─────────────────────────────────────────────────────────── */
typedef enum {
    ONEOS_GRANT_NOT_REQUESTED   = 0,
    ONEOS_GRANT_GRANTED         = 1,
    ONEOS_GRANT_DENIED          = 2,
    ONEOS_GRANT_PERM_DENIED     = 3, /**< "Don't ask again" was checked. */
} OneoS_GrantState;

/* ── Callback for async permission results ───────────────────────────────── */
/**
 * @param permission  The permission that was decided.
 * @param granted     true if the user granted the permission.
 * @param user_data   Caller-supplied pointer passed to oneos_request_permission.
 */
typedef void (*OneoS_PermissionCallback)(OneoS_Permission permission,
                                          bool             granted,
                                          void            *user_data);

/* ── API ─────────────────────────────────────────────────────────────────── */

/**
 * Check whether the calling app currently holds a permission.
 *
 * @return ONEOS_OK if granted, ONEOS_ERROR_PERMISSION_DENIED otherwise.
 */
int oneos_check_permission(OneoS_Permission permission);

/**
 * Request a runtime permission.  If the user has already decided, the
 * callback is invoked synchronously (on the calling thread).  Otherwise
 * a system dialog is displayed and the callback is invoked on the main
 * UI thread when the user responds.
 *
 * @param permission  Permission to request.
 * @param callback    Invoked with the result (must not be NULL).
 * @param user_data   Forwarded unchanged to the callback.
 */
void oneos_request_permission(OneoS_Permission         permission,
                               OneoS_PermissionCallback callback,
                               void                    *user_data);

/**
 * Request multiple permissions in a single dialog.
 *
 * @param permissions  Array of permission bits.
 * @param count        Length of the array.
 * @param callback     Called once for each permission in the array.
 * @param user_data    Forwarded unchanged to each callback invocation.
 */
void oneos_request_permissions(const OneoS_Permission  *permissions,
                                size_t                   count,
                                OneoS_PermissionCallback callback,
                                void                    *user_data);

/**
 * Query the current grant state of a permission without triggering a dialog.
 */
OneoS_GrantState oneos_permission_state(OneoS_Permission permission);

#ifdef __cplusplus
}
#endif
#endif /* ONEOS_PERMISSIONS_H */
