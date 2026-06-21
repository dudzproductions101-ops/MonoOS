# MonoOS SDK – API Reference

**SDK Version:** 1.0.0

---

## Module Index

| Module | Header | Description |
|--------|--------|-------------|
| Core | `monoos.h` | Context lifecycle, result codes |
| Permissions | `monoos_permissions.h` | Runtime permission model |
| Storage | `monoos_storage.h` | Scoped storage and media store |
| Notifications | `monoos_notifications.h` | Notification channels and posts |
| Audio | `monoos_audio.h` | PCM stream playback |
| Media | `monoos_media.h` | High-level media player |
| Network | `monoos_network.h` | Connectivity state and DNS |

---

## Result Codes

| Constant | Value | Meaning |
|----------|-------|---------|
| `MONOOS_OK` | 0 | Success |
| `MONOOS_ERROR` | -1 | Generic error |
| `MONOOS_ERROR_INVALID_ARG` | -2 | Null or out-of-range argument |
| `MONOOS_ERROR_PERMISSION_DENIED` | -3 | Permission not granted |
| `MONOOS_ERROR_NOT_FOUND` | -4 | Resource not found |
| `MONOOS_ERROR_ALREADY_EXISTS` | -5 | Resource already exists |
| `MONOOS_ERROR_NO_MEMORY` | -6 | Memory allocation failed |
| `MONOOS_ERROR_IO` | -7 | I/O error |
| `MONOOS_ERROR_TIMEOUT` | -8 | Operation timed out |
| `MONOOS_ERROR_NOT_SUPPORTED` | -9 | Feature not available on device |
| `MONOOS_ERROR_NOT_INITIALISED` | -10 | Component not yet initialised |

---

## Core API

### `monoos_context_create`

```c
MonoOS_Context *monoos_context_create(const char *package_name, uint32_t version_code);
```

Creates the application context.  Must be the first MonoOS API call.  
Returns `NULL` on failure (e.g., package not registered with the system).

### `monoos_context_destroy`

```c
void monoos_context_destroy(MonoOS_Context *ctx);
```

Releases all resources held by the context.  Safe to call with `NULL`.

---

## Permissions API

### `monoos_check_permission`

```c
int monoos_check_permission(MonoOS_Permission permission);
```

Returns `MONOOS_OK` if the permission is currently granted, otherwise  
`MONOOS_ERROR_PERMISSION_DENIED`.

### `monoos_request_permission`

```c
void monoos_request_permission(MonoOS_Permission permission,
                               MonoOS_PermissionCallback callback,
                               void *user_data);
```

Shows the system permission dialog if needed.  The `callback` is  
guaranteed to be called exactly once.

---

## Storage API

### `monoos_files_dir`

```c
const char *monoos_files_dir(void);
```

Returns the absolute path to the app's private files directory, e.g.:  
`/data/data/com.example.app/files`

### `monoos_media_query`

```c
int monoos_media_query(MonoOS_MediaType type,
                       MonoOS_MediaQueryCallback callback,
                       void *user_data);
```

Enumerates shared media.  Requires `MONOOS_PERM_STORAGE`.  
The callback may be invoked on a background thread.

---

## Audio API

### `monoos_audio_open_stream`

```c
MonoOS_AudioStreamHandle monoos_audio_open_stream(uint32_t sample_rate,
                                                  uint32_t channels,
                                                  MonoOS_AudioUsage usage,
                                                  float volume);
```

Opens a PCM output stream.  Standard configuration: 48000 Hz, 2 ch, FLOAT32.

---

## Media API

### Complete example

```c
#include <monoos/monoos.h>

void on_state(MonoOS_Player *p, MonoOS_PlayerState s, void *u) {
    if (s == MONOOS_PLAYER_PREPARED) monoos_player_start(p);
}

int main(void) {
    MonoOS_Context *ctx = monoos_context_create("com.example.app", 1);
    MonoOS_Player  *p   = monoos_player_create();

    MonoOS_PlayerListener l = { .on_state = on_state };
    monoos_player_set_listener(p, &l, NULL);
    monoos_player_set_uri(p, "file:///sdcard/Music/song.mp3");
    monoos_player_prepare(p);

    // ... event loop ...

    monoos_player_destroy(p);
    monoos_context_destroy(ctx);
    return 0;
}
```

---

## Network API

### `monoos_net_get_state`

```c
int monoos_net_get_state(MonoOS_NetworkState *out);
```

Populates `*out` with current network state.  
Returns `MONOOS_OK` or `MONOOS_ERROR_PERMISSION_DENIED`.
