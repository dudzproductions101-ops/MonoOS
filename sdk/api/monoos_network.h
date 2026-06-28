/**
 * monoos_network.h – MonoOS Network Connectivity API
 *
 * Provides connectivity state queries and DNS resolution.
 * Requires MONOOS_PERM_NETWORK.
 */

#pragma once
#ifndef MONOOS_NETWORK_H
#define MONOOS_NETWORK_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Network type ────────────────────────────────────────────────────────── */
typedef enum {
    MONOOS_NET_TYPE_NONE      = 0,
    MONOOS_NET_TYPE_WIFI      = 1,
    MONOOS_NET_TYPE_CELLULAR  = 2,
    MONOOS_NET_TYPE_ETHERNET  = 3,
    MONOOS_NET_TYPE_VPN       = 4,
    MONOOS_NET_TYPE_BLUETOOTH = 5,
} MonoOS_NetworkType;

/* ── Connectivity state ──────────────────────────────────────────────────── */
typedef struct {
    bool              connected;
    MonoOS_NetworkType type;
    bool              metered;        /**< True if data has a cost (cellular). */
    bool              roaming;
    int32_t           signal_strength; /**< dBm (negative) or 0 if unknown. */
    char              ssid[64];        /**< Wi-Fi SSID, empty otherwise. */
} MonoOS_NetworkState;

/* ── Connectivity change callback ────────────────────────────────────────── */
typedef void (*MonoOS_NetworkCallback)(const MonoOS_NetworkState *state,
                                       void                     *user_data);

/* ── API ─────────────────────────────────────────────────────────────────── */

/**
 * Synchronously query the current network state.
 *
 * @param out  Populated on success.
 * @return MONOOS_OK or MONOOS_ERROR_PERMISSION_DENIED.
 */
int monoos_net_get_state(MonoOS_NetworkState *out);

/**
 * Register a callback for connectivity changes.
 * The callback is invoked on the main thread.
 *
 * @param callback   Called on each state change.  NULL to unregister.
 * @param user_data  Forwarded to callback.
 * @return MONOOS_OK, or an error code.
 */
int monoos_net_listen(MonoOS_NetworkCallback callback, void *user_data);

/**
 * Non-blocking DNS resolution via the MonoOS DNS-over-HTTPS resolver.
 *
 * @param hostname    Hostname to resolve.
 * @param callback    Called with results: array of IP strings, or NULL + err.
 * @param user_data   Forwarded to callback.
 */
typedef void (*MonoOS_DnsCallback)(const char **addrs, int count,
                                   int err, void *user_data);

int monoos_net_resolve(const char       *hostname,
                       MonoOS_DnsCallback callback,
                       void             *user_data);

#ifdef __cplusplus
}
#endif
#endif /* MONOOS_NETWORK_H */
