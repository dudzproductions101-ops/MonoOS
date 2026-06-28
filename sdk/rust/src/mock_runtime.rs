//! mock_runtime.rs – Minimal host-side implementation of the MonoOS runtime
//! C ABI, compiled in only under `--features mock-runtime`.
//!
//! This lets `cargo test` exercise the safe wrapper layer end-to-end without
//! requiring `libmonoos_runtime.so` to be present (which only exists on a
//! real device / the emulator). It deliberately keeps just enough state to
//! make the wrappers' contracts observable in tests; it is not a substitute
//! for the real runtime's behavior (permission prompts, actual I/O, etc.).

#![cfg(feature = "mock-runtime")]

use crate::sys::*;
use core::ffi::{c_char, c_int, c_void};
use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::CString;

thread_local! {
    static GRANTED: RefCell<HashSet<MonoOS_Permission>> = RefCell::new(HashSet::new());
}

/// Test-only hook: pre-grant a permission so `monoos_check_permission` and
/// `monoos_request_permission` report it as already granted.
pub fn mock_grant(permission: MonoOS_Permission) {
    GRANTED.with(|g| g.borrow_mut().insert(permission));
}

/// Test-only hook: reset all granted permissions between tests.
pub fn mock_reset() {
    GRANTED.with(|g| g.borrow_mut().clear());
}

#[no_mangle]
pub extern "C" fn monoos_result_str(r: MonoOS_Result) -> *const c_char {
    let s = match r {
        MONOOS_OK => "OK",
        MONOOS_ERROR_INVALID_ARG => "INVALID_ARG",
        MONOOS_ERROR_PERMISSION_DENIED => "PERMISSION_DENIED",
        MONOOS_ERROR_NOT_FOUND => "NOT_FOUND",
        MONOOS_ERROR_ALREADY_EXISTS => "ALREADY_EXISTS",
        MONOOS_ERROR_NO_MEMORY => "NO_MEMORY",
        MONOOS_ERROR_IO => "IO",
        MONOOS_ERROR_TIMEOUT => "TIMEOUT",
        MONOOS_ERROR_NOT_SUPPORTED => "NOT_SUPPORTED",
        MONOOS_ERROR_NOT_INITIALISED => "NOT_INITIALISED",
        _ => "ERROR",
    };
    // Leak a small static-lifetime CString; acceptable for a test-only mock.
    Box::leak(CString::new(s).unwrap().into_boxed_c_str()).as_ptr()
}

#[no_mangle]
pub extern "C" fn monoos_context_create(package_name: *const c_char, _version_code: u32) -> *mut MonoOS_Context {
    if package_name.is_null() {
        return core::ptr::null_mut();
    }
    Box::into_raw(Box::new(MonoOS_Context { _private: [] }))
}

#[no_mangle]
pub extern "C" fn monoos_context_destroy(ctx: *mut MonoOS_Context) {
    if !ctx.is_null() {
        unsafe { drop(Box::from_raw(ctx)) };
    }
}

#[no_mangle]
pub extern "C" fn monoos_context_package_name(_ctx: *const MonoOS_Context) -> *const c_char {
    static NAME: &[u8] = b"com.monoos.mock\0";
    NAME.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn monoos_check_permission(permission: MonoOS_Permission) -> c_int {
    GRANTED.with(|g| {
        if g.borrow().contains(&permission) {
            MONOOS_OK
        } else {
            MONOOS_ERROR_PERMISSION_DENIED
        }
    })
}

#[no_mangle]
pub extern "C" fn monoos_request_permission(
    permission: MonoOS_Permission,
    callback: MonoOS_PermissionCallback,
    user_data: *mut c_void,
) {
    // Mock policy: auto-grant every requested permission (deterministic for tests).
    GRANTED.with(|g| g.borrow_mut().insert(permission));
    callback(permission, true, user_data);
}

#[no_mangle]
pub extern "C" fn monoos_request_permissions(
    permissions: *const MonoOS_Permission,
    count: usize,
    callback: MonoOS_PermissionCallback,
    user_data: *mut c_void,
) {
    if permissions.is_null() {
        return;
    }
    let slice = unsafe { core::slice::from_raw_parts(permissions, count) };
    for &p in slice {
        GRANTED.with(|g| g.borrow_mut().insert(p));
        callback(p, true, user_data);
    }
}

#[no_mangle]
pub extern "C" fn monoos_permission_state(permission: MonoOS_Permission) -> MonoOS_GrantState {
    GRANTED.with(|g| {
        if g.borrow().contains(&permission) {
            MonoOS_GrantState::Granted
        } else {
            MonoOS_GrantState::NotRequested
        }
    })
}

#[no_mangle]
pub extern "C" fn monoos_notif_create_channel(channel: *const MonoOS_NotifChannel) -> c_int {
    if channel.is_null() { MONOOS_ERROR_INVALID_ARG } else { MONOOS_OK }
}
#[no_mangle]
pub extern "C" fn monoos_notif_delete_channel(channel_id: *const c_char) -> c_int {
    if channel_id.is_null() { MONOOS_ERROR_INVALID_ARG } else { MONOOS_OK }
}
#[no_mangle]
pub extern "C" fn monoos_notif_post(notif: *const MonoOS_Notification) -> c_int {
    if notif.is_null() { MONOOS_ERROR_INVALID_ARG } else { MONOOS_OK }
}
#[no_mangle]
pub extern "C" fn monoos_notif_cancel(_notif_id: i32) -> c_int {
    MONOOS_OK
}
#[no_mangle]
pub extern "C" fn monoos_notif_cancel_all() {}

#[no_mangle]
pub extern "C" fn monoos_files_dir() -> *const c_char {
    static P: &[u8] = b"/data/data/com.monoos.mock/files\0";
    P.as_ptr() as *const c_char
}
#[no_mangle]
pub extern "C" fn monoos_cache_dir() -> *const c_char {
    static P: &[u8] = b"/data/data/com.monoos.mock/cache\0";
    P.as_ptr() as *const c_char
}
#[no_mangle]
pub extern "C" fn monoos_db_dir() -> *const c_char {
    static P: &[u8] = b"/data/data/com.monoos.mock/databases\0";
    P.as_ptr() as *const c_char
}
#[no_mangle]
pub extern "C" fn monoos_media_query(
    _media_type: MonoOS_MediaType,
    _callback: MonoOS_MediaQueryCallback,
    _user_data: *mut c_void,
) -> c_int {
    MONOOS_OK // no entries in the mock store
}
#[no_mangle]
pub extern "C" fn monoos_media_insert(
    path: *const c_char,
    _mime_type: *const c_char,
    out_uri: *mut MonoOS_ContentUri,
) -> c_int {
    if path.is_null() || out_uri.is_null() {
        return MONOOS_ERROR_INVALID_ARG;
    }
    MONOOS_OK
}
#[no_mangle]
pub extern "C" fn monoos_media_delete(_uri: *const MonoOS_ContentUri) -> c_int {
    MONOOS_OK
}

#[no_mangle]
pub extern "C" fn monoos_net_get_state(out: *mut MonoOS_NetworkState) -> c_int {
    if out.is_null() {
        return MONOOS_ERROR_INVALID_ARG;
    }
    unsafe {
        (*out).connected = true;
        (*out).net_type = MonoOS_NetworkType::Wifi;
        (*out).metered = false;
        (*out).roaming = false;
        (*out).signal_strength = -50;
        (*out).ssid = [0; 64];
    }
    MONOOS_OK
}
#[no_mangle]
pub extern "C" fn monoos_net_listen(_callback: MonoOS_NetworkCallback, _user_data: *mut c_void) -> c_int {
    MONOOS_OK
}
#[no_mangle]
pub extern "C" fn monoos_net_resolve(
    hostname: *const c_char,
    callback: MonoOS_DnsCallback,
    user_data: *mut c_void,
) -> c_int {
    if hostname.is_null() {
        return MONOOS_ERROR_INVALID_ARG;
    }
    let addr = CString::new("127.0.0.1").unwrap();
    let ptr = addr.as_ptr();
    callback(&ptr as *const *const c_char, 1, 0, user_data);
    MONOOS_OK
}

#[no_mangle]
pub extern "C" fn monoos_audio_open_stream(
    _sample_rate: u32,
    _channels: u32,
    _usage: MonoOS_AudioUsage,
    _volume: f32,
) -> MonoOS_AudioStreamHandle {
    1
}
#[no_mangle]
pub extern "C" fn monoos_audio_write(handle: MonoOS_AudioStreamHandle, _data: *const f32, frames: u32) -> c_int {
    if handle == MONOOS_AUDIO_INVALID_HANDLE { -1 } else { frames as c_int }
}
#[no_mangle]
pub extern "C" fn monoos_audio_set_volume(_handle: MonoOS_AudioStreamHandle, _volume: f32) -> c_int {
    MONOOS_OK
}
#[no_mangle]
pub extern "C" fn monoos_audio_close_stream(_handle: MonoOS_AudioStreamHandle) {}
#[no_mangle]
pub extern "C" fn monoos_audio_set_master_volume(_volume: f32) -> c_int {
    MONOOS_OK
}

#[no_mangle]
pub extern "C" fn monoos_ui_load_qml(qml_path: *const c_char) -> c_int {
    if qml_path.is_null() { MONOOS_ERROR_INVALID_ARG } else { MONOOS_OK }
}
#[no_mangle]
pub extern "C" fn monoos_ui_reload() -> c_int {
    MONOOS_OK
}
#[no_mangle]
pub extern "C" fn monoos_ui_set_status_bar_visible(_visible: bool) {}
#[no_mangle]
pub extern "C" fn monoos_ui_set_edge_to_edge(_enabled: bool) {}

// ── Media player mock ────────────────────────────────────────────────────────

struct MockPlayer {
    state: std::cell::Cell<MonoOS_PlayerState>,
    position_ms: std::cell::Cell<u64>,
    duration_ms: std::cell::Cell<u64>,
    looping: std::cell::Cell<bool>,
}

#[no_mangle]
pub extern "C" fn monoos_player_create() -> *mut MonoOS_Player {
    let mock = Box::new(MockPlayer {
        state: std::cell::Cell::new(MonoOS_PlayerState::Idle),
        position_ms: std::cell::Cell::new(0),
        duration_ms: std::cell::Cell::new(180_000), // pretend 3-minute track
        looping: std::cell::Cell::new(false),
    });
    Box::into_raw(mock) as *mut MonoOS_Player
}

#[no_mangle]
pub extern "C" fn monoos_player_destroy(player: *mut MonoOS_Player) {
    if !player.is_null() {
        unsafe { drop(Box::from_raw(player as *mut MockPlayer)) };
    }
}

#[no_mangle]
pub extern "C" fn monoos_player_set_uri(player: *mut MonoOS_Player, uri: *const c_char) -> c_int {
    if player.is_null() || uri.is_null() {
        return MONOOS_ERROR_INVALID_ARG;
    }
    let p = unsafe { &*(player as *const MockPlayer) };
    p.state.set(MonoOS_PlayerState::Idle);
    p.position_ms.set(0);
    MONOOS_OK
}

#[no_mangle]
pub extern "C" fn monoos_player_prepare(player: *mut MonoOS_Player) -> c_int {
    if player.is_null() {
        return MONOOS_ERROR_INVALID_ARG;
    }
    unsafe { &*(player as *const MockPlayer) }.state.set(MonoOS_PlayerState::Prepared);
    MONOOS_OK
}

#[no_mangle]
pub extern "C" fn monoos_player_start(player: *mut MonoOS_Player) -> c_int {
    if player.is_null() {
        return MONOOS_ERROR_INVALID_ARG;
    }
    unsafe { &*(player as *const MockPlayer) }.state.set(MonoOS_PlayerState::Started);
    MONOOS_OK
}

#[no_mangle]
pub extern "C" fn monoos_player_pause(player: *mut MonoOS_Player) -> c_int {
    if player.is_null() {
        return MONOOS_ERROR_INVALID_ARG;
    }
    unsafe { &*(player as *const MockPlayer) }.state.set(MonoOS_PlayerState::Paused);
    MONOOS_OK
}

#[no_mangle]
pub extern "C" fn monoos_player_stop(player: *mut MonoOS_Player) -> c_int {
    if player.is_null() {
        return MONOOS_ERROR_INVALID_ARG;
    }
    let p = unsafe { &*(player as *const MockPlayer) };
    p.state.set(MonoOS_PlayerState::Stopped);
    p.position_ms.set(0);
    MONOOS_OK
}

#[no_mangle]
pub extern "C" fn monoos_player_seek(player: *mut MonoOS_Player, pos_ms: u64) -> c_int {
    if player.is_null() {
        return MONOOS_ERROR_INVALID_ARG;
    }
    unsafe { &*(player as *const MockPlayer) }.position_ms.set(pos_ms);
    MONOOS_OK
}

#[no_mangle]
pub extern "C" fn monoos_player_set_volume(player: *mut MonoOS_Player, _volume: f32) -> c_int {
    if player.is_null() { MONOOS_ERROR_INVALID_ARG } else { MONOOS_OK }
}

#[no_mangle]
pub extern "C" fn monoos_player_set_rate(player: *mut MonoOS_Player, _rate: f32) -> c_int {
    if player.is_null() { MONOOS_ERROR_INVALID_ARG } else { MONOOS_OK }
}

#[no_mangle]
pub extern "C" fn monoos_player_set_looping(player: *mut MonoOS_Player, looping: bool) {
    if !player.is_null() {
        unsafe { &*(player as *const MockPlayer) }.looping.set(looping);
    }
}

#[no_mangle]
pub extern "C" fn monoos_player_position(player: *const MonoOS_Player) -> u64 {
    if player.is_null() {
        return 0;
    }
    unsafe { &*(player as *const MockPlayer) }.position_ms.get()
}

#[no_mangle]
pub extern "C" fn monoos_player_duration(player: *const MonoOS_Player) -> u64 {
    if player.is_null() {
        return 0;
    }
    unsafe { &*(player as *const MockPlayer) }.duration_ms.get()
}

#[no_mangle]
pub extern "C" fn monoos_player_state(player: *const MonoOS_Player) -> MonoOS_PlayerState {
    if player.is_null() {
        return MonoOS_PlayerState::Error;
    }
    unsafe { &*(player as *const MockPlayer) }.state.get()
}

#[no_mangle]
pub extern "C" fn monoos_player_set_listener(
    player: *mut MonoOS_Player,
    listener: *const MonoOS_PlayerListener,
    user_data: *mut c_void,
) {
    if player.is_null() || listener.is_null() {
        return;
    }
    // Mock: immediately fire on_state with the player's current state so
    // tests can observe that a listener was wired up correctly.
    let p = unsafe { &*(player as *const MockPlayer) };
    let l = unsafe { &*listener };
    if let Some(cb) = l.on_state {
        cb(player, p.state.get(), user_data);
    }
}
