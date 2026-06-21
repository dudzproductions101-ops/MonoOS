# OneOS SDK – API Reference

**SDK Version:** 1.0.0

---

## Module Index

| Module | Header | Description |
|--------|--------|-------------|
| Core | `oneos.h` | Context lifecycle, result codes |
| Permissions | `oneos_permissions.h` | Runtime permission model |
| Storage | `oneos_storage.h` | Scoped storage and media store |
| Notifications | `oneos_notifications.h` | Notification channels and posts |
| Audio | `oneos_audio.h` | PCM stream playback |
| Media | `oneos_media.h` | High-level media player |
| Network | `oneos_network.h` | Connectivity state and DNS |

---

## Result Codes

| Constant | Value | Meaning |
|----------|-------|---------|
| `ONEOS_OK` | 0 | Success |
| `ONEOS_ERROR` | -1 | Generic error |
| `ONEOS_ERROR_INVALID_ARG` | -2 | Null or out-of-range argument |
| `ONEOS_ERROR_PERMISSION_DENIED` | -3 | Permission not granted |
| `ONEOS_ERROR_NOT_FOUND` | -4 | Resource not found |
| `ONEOS_ERROR_ALREADY_EXISTS` | -5 | Resource already exists |
| `ONEOS_ERROR_NO_MEMORY` | -6 | Memory allocation failed |
| `ONEOS_ERROR_IO` | -7 | I/O error |
| `ONEOS_ERROR_TIMEOUT` | -8 | Operation timed out |
| `ONEOS_ERROR_NOT_SUPPORTED` | -9 | Feature not available on device |
| `ONEOS_ERROR_NOT_INITIALISED` | -10 | Component not yet initialised |

---

## Core API

### `oneos_context_create`

```c
OneOS_Context *oneos_context_create(const char *package_name, uint32_t version_code);
```

Creates the application context.  Must be the first OneOS API call.  
Returns `NULL` on failure (e.g., package not registered with the system).

### `oneos_context_destroy`

```c
void oneos_context_destroy(OneOS_Context *ctx);
```

Releases all resources held by the context.  Safe to call with `NULL`.

---

## Permissions API

### `oneos_check_permission`

```c
int oneos_check_permission(OneoS_Permission permission);
```

Returns `ONEOS_OK` if the permission is currently granted, otherwise  
`ONEOS_ERROR_PERMISSION_DENIED`.

### `oneos_request_permission`

```c
void oneos_request_permission(OneoS_Permission permission,
                               OneoS_PermissionCallback callback,
                               void *user_data);
```

Shows the system permission dialog if needed.  The `callback` is  
guaranteed to be called exactly once.

---

## Storage API

### `oneos_files_dir`

```c
const char *oneos_files_dir(void);
```

Returns the absolute path to the app's private files directory, e.g.:  
`/data/data/com.example.app/files`

### `oneos_media_query`

```c
int oneos_media_query(OneoS_MediaType type,
                       OneoS_MediaQueryCallback callback,
                       void *user_data);
```

Enumerates shared media.  Requires `ONEOS_PERM_STORAGE`.  
The callback may be invoked on a background thread.

---

## Audio API

### `oneos_audio_open_stream`

```c
OneoS_AudioStreamHandle oneos_audio_open_stream(uint32_t sample_rate,
                                                  uint32_t channels,
                                                  OneoS_AudioUsage usage,
                                                  float volume);
```

Opens a PCM output stream.  Standard configuration: 48000 Hz, 2 ch, FLOAT32.

---

## Media API

### Complete example

```c
#include <oneos/oneos.h>

void on_state(OneoS_Player *p, OneoS_PlayerState s, void *u) {
    if (s == ONEOS_PLAYER_PREPARED) oneos_player_start(p);
}

int main(void) {
    OneOS_Context *ctx = oneos_context_create("com.example.app", 1);
    OneoS_Player  *p   = oneos_player_create();

    OneoS_PlayerListener l = { .on_state = on_state };
    oneos_player_set_listener(p, &l, NULL);
    oneos_player_set_uri(p, "file:///sdcard/Music/song.mp3");
    oneos_player_prepare(p);

    // ... event loop ...

    oneos_player_destroy(p);
    oneos_context_destroy(ctx);
    return 0;
}
```

---

## Network API

### `oneos_net_get_state`

```c
int oneos_net_get_state(OneoS_NetworkState *out);
```

Populates `*out` with current network state.  
Returns `ONEOS_OK` or `ONEOS_ERROR_PERMISSION_DENIED`.
