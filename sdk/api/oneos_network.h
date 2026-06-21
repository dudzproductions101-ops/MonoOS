/**
 * oneos_network.h – OneOS Network Connectivity API
 *
 * Provides connectivity state queries and DNS resolution.
 * Requires ONEOS_PERM_NETWORK.
 */

#pragma once
#ifndef ONEOS_NETWORK_H
#define ONEOS_NETWORK_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Network type ────────────────────────────────────────────────────────── */
typedef enum {
    ONEOS_NET_TYPE_NONE      = 0,
    ONEOS_NET_TYPE_WIFI      = 1,
    ONEOS_NET_TYPE_CELLULAR  = 2,
    ONEOS_NET_TYPE_ETHERNET  = 3,
    ONEOS_NET_TYPE_VPN       = 4,
    ONEOS_NET_TYPE_BLUETOOTH = 5,
} OneoS_NetworkType;

/* ── Connectivity state ──────────────────────────────────────────────────── */
typedef struct {
    bool              connected;
    OneoS_NetworkType type;
    bool              metered;        /**< True if data has a cost (cellular). */
    bool              roaming;
    int32_t           signal_strength; /**< dBm (negative) or 0 if unknown. */
    char              ssid[64];        /**< Wi-Fi SSID, empty otherwise. */
} OneoS_NetworkState;

/* ── Connectivity change callback ────────────────────────────────────────── */
typedef void (*OneoS_NetworkCallback)(const OneoS_NetworkState *state,
                                       void                     *user_data);

/* ── API ─────────────────────────────────────────────────────────────────── */

/**
 * Synchronously query the current network state.
 *
 * @param out  Populated on success.
 * @return ONEOS_OK or ONEOS_ERROR_PERMISSION_DENIED.
 */
int oneos_net_get_state(OneoS_NetworkState *out);

/**
 * Register a callback for connectivity changes.
 * The callback is invoked on the main thread.
 *
 * @param callback   Called on each state change.  NULL to unregister.
 * @param user_data  Forwarded to callback.
 * @return ONEOS_OK, or an error code.
 */
int oneos_net_listen(OneoS_NetworkCallback callback, void *user_data);

/**
 * Non-blocking DNS resolution via the OneOS DNS-over-HTTPS resolver.
 *
 * @param hostname    Hostname to resolve.
 * @param callback    Called with results: array of IP strings, or NULL + err.
 * @param user_data   Forwarded to callback.
 */
typedef void (*OneoS_DnsCallback)(const char **addrs, int count,
                                   int err, void *user_data);

int oneos_net_resolve(const char       *hostname,
                       OneoS_DnsCallback callback,
                       void             *user_data);

#ifdef __cplusplus
}
#endif
#endif /* ONEOS_NETWORK_H */
