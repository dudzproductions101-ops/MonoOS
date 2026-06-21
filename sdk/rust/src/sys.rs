//! sys.rs – Raw FFI declarations mirroring `sdk/api/*.h` byte-for-byte.
//!
//! Nothing in this module is safe to call directly from application code;
//! use the safe wrappers in the sibling modules instead. These symbols are
//! provided at runtime by `libmonoos_runtime.so` on-device.

#![allow(non_camel_case_types, non_snake_case)]

use core::ffi::{c_char, c_int, c_void};

// ── Result codes (monoos.h) ──────────────────────────────────────────────────
pub type MonoOS_Result = i32;

pub const MONOOS_OK: MonoOS_Result = 0;
pub const MONOOS_ERROR: MonoOS_Result = -1;
pub const MONOOS_ERROR_INVALID_ARG: MonoOS_Result = -2;
pub const MONOOS_ERROR_PERMISSION_DENIED: MonoOS_Result = -3;
pub const MONOOS_ERROR_NOT_FOUND: MonoOS_Result = -4;
pub const MONOOS_ERROR_ALREADY_EXISTS: MonoOS_Result = -5;
pub const MONOOS_ERROR_NO_MEMORY: MonoOS_Result = -6;
pub const MONOOS_ERROR_IO: MonoOS_Result = -7;
pub const MONOOS_ERROR_TIMEOUT: MonoOS_Result = -8;
pub const MONOOS_ERROR_NOT_SUPPORTED: MonoOS_Result = -9;
pub const MONOOS_ERROR_NOT_INITIALISED: MonoOS_Result = -10;

#[repr(C)]
pub struct MonoOS_Context {
    pub(crate) _private: [u8; 0],
}

// ── Permissions (monoos_permissions.h) ───────────────────────────────────────
pub type MonoOS_Permission = u32;

pub const MONOOS_PERM_CAMERA: MonoOS_Permission = 0x0000_0001;
pub const MONOOS_PERM_MICROPHONE: MonoOS_Permission = 0x0000_0002;
pub const MONOOS_PERM_LOCATION: MonoOS_Permission = 0x0000_0004;
pub const MONOOS_PERM_CONTACTS: MonoOS_Permission = 0x0000_0008;
pub const MONOOS_PERM_STORAGE: MonoOS_Permission = 0x0000_0010;
pub const MONOOS_PERM_PHONE: MonoOS_Permission = 0x0000_0020;
pub const MONOOS_PERM_BLUETOOTH: MonoOS_Permission = 0x0000_0040;
pub const MONOOS_PERM_NFC: MonoOS_Permission = 0x0000_0080;
pub const MONOOS_PERM_SENSORS: MonoOS_Permission = 0x0000_0100;
pub const MONOOS_PERM_NETWORK: MonoOS_Permission = 0x0000_0200;
pub const MONOOS_PERM_NOTIFICATIONS: MonoOS_Permission = 0x0000_0400;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonoOS_GrantState {
    NotRequested = 0,
    Granted = 1,
    Denied = 2,
    PermDenied = 3,
}

pub type MonoOS_PermissionCallback =
    extern "C" fn(permission: MonoOS_Permission, granted: bool, user_data: *mut c_void);

// ── Notifications (monoos_notifications.h) ───────────────────────────────────
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonoOS_NotifPriority {
    Min = 0,
    Low = 1,
    Default = 2,
    High = 3,
    Max = 4,
}

#[repr(C)]
pub struct MonoOS_NotifChannel {
    pub id: [c_char; 64],
    pub name: [c_char; 128],
    pub description: [c_char; 256],
    pub importance: MonoOS_NotifPriority,
    pub vibrate: bool,
    pub show_badge: bool,
}

#[repr(C)]
pub struct MonoOS_Notification {
    pub id: i32,
    pub channel_id: [c_char; 64],
    pub title: [c_char; 256],
    pub body: [c_char; 512],
    pub ticker: [c_char; 256],
    pub priority: MonoOS_NotifPriority,
    pub auto_cancel: bool,
    pub ongoing: bool,
    pub badge_count: u32,
}

// ── Storage (monoos_storage.h) ───────────────────────────────────────────────
#[repr(C)]
pub struct MonoOS_ContentUri {
    pub uri: [c_char; 256],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonoOS_MediaType {
    Image = 0,
    Video = 1,
    Audio = 2,
    Document = 3,
    Other = 4,
}

#[repr(C)]
pub struct MonoOS_MediaEntry {
    pub uri: MonoOS_ContentUri,
    pub display_name: [c_char; 256],
    pub size_bytes: u64,
    pub media_type: MonoOS_MediaType,
    pub mime_type: [c_char; 64],
    pub date_added: u64,
    pub date_modified: u64,
    pub width: u32,
    pub height: u32,
    pub duration_ms: u64,
}

pub type MonoOS_MediaQueryCallback =
    extern "C" fn(entry: *const MonoOS_MediaEntry, user_data: *mut c_void);

// ── Network (monoos_network.h) ───────────────────────────────────────────────
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonoOS_NetworkType {
    None = 0,
    Wifi = 1,
    Cellular = 2,
    Ethernet = 3,
    Vpn = 4,
    Bluetooth = 5,
}

#[repr(C)]
pub struct MonoOS_NetworkState {
    pub connected: bool,
    pub net_type: MonoOS_NetworkType,
    pub metered: bool,
    pub roaming: bool,
    pub signal_strength: i32,
    pub ssid: [c_char; 64],
}

pub type MonoOS_NetworkCallback =
    extern "C" fn(state: *const MonoOS_NetworkState, user_data: *mut c_void);
pub type MonoOS_DnsCallback =
    extern "C" fn(addrs: *const *const c_char, count: c_int, err: c_int, user_data: *mut c_void);

// ── Audio (monoos_audio.h) ───────────────────────────────────────────────────
pub type MonoOS_AudioStreamHandle = u32;
pub const MONOOS_AUDIO_INVALID_HANDLE: MonoOS_AudioStreamHandle = 0;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonoOS_AudioUsage {
    Media = 0,
    Notification = 1,
    Ringtone = 2,
    VoiceCall = 3,
    Alarm = 4,
    Game = 5,
}

// ── Media playback (monoos_media.h) ──────────────────────────────────────────
#[repr(C)]
pub struct MonoOS_Player {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonoOS_PlayerState {
    Idle = 0,
    Prepared = 1,
    Started = 2,
    Paused = 3,
    Stopped = 4,
    Complete = 5,
    Error = 6,
}

#[repr(C)]
pub struct MonoOS_PlayerListener {
    pub on_state: Option<extern "C" fn(player: *mut MonoOS_Player, state: MonoOS_PlayerState, user: *mut c_void)>,
    pub on_position: Option<extern "C" fn(player: *mut MonoOS_Player, pos_ms: u64, user: *mut c_void)>,
    pub on_error:
        Option<extern "C" fn(player: *mut MonoOS_Player, code: c_int, msg: *const c_char, user: *mut c_void)>,
    pub on_complete: Option<extern "C" fn(player: *mut MonoOS_Player, user: *mut c_void)>,
}

// ── Extern symbols ───────────────────────────────────────────────────────────
#[cfg_attr(not(feature = "mock-runtime"), link(name = "monoos_runtime"))]
extern "C" {
    pub fn monoos_result_str(r: MonoOS_Result) -> *const c_char;

    pub fn monoos_context_create(package_name: *const c_char, version_code: u32) -> *mut MonoOS_Context;
    pub fn monoos_context_destroy(ctx: *mut MonoOS_Context);
    pub fn monoos_context_package_name(ctx: *const MonoOS_Context) -> *const c_char;

    pub fn monoos_check_permission(permission: MonoOS_Permission) -> c_int;
    pub fn monoos_request_permission(
        permission: MonoOS_Permission,
        callback: MonoOS_PermissionCallback,
        user_data: *mut c_void,
    );
    pub fn monoos_request_permissions(
        permissions: *const MonoOS_Permission,
        count: usize,
        callback: MonoOS_PermissionCallback,
        user_data: *mut c_void,
    );
    pub fn monoos_permission_state(permission: MonoOS_Permission) -> MonoOS_GrantState;

    pub fn monoos_notif_create_channel(channel: *const MonoOS_NotifChannel) -> c_int;
    pub fn monoos_notif_delete_channel(channel_id: *const c_char) -> c_int;
    pub fn monoos_notif_post(notif: *const MonoOS_Notification) -> c_int;
    pub fn monoos_notif_cancel(notif_id: i32) -> c_int;
    pub fn monoos_notif_cancel_all();

    pub fn monoos_files_dir() -> *const c_char;
    pub fn monoos_cache_dir() -> *const c_char;
    pub fn monoos_db_dir() -> *const c_char;
    pub fn monoos_media_query(
        media_type: MonoOS_MediaType,
        callback: MonoOS_MediaQueryCallback,
        user_data: *mut c_void,
    ) -> c_int;
    pub fn monoos_media_insert(
        path: *const c_char,
        mime_type: *const c_char,
        out_uri: *mut MonoOS_ContentUri,
    ) -> c_int;
    pub fn monoos_media_delete(uri: *const MonoOS_ContentUri) -> c_int;

    pub fn monoos_net_get_state(out: *mut MonoOS_NetworkState) -> c_int;
    pub fn monoos_net_listen(callback: MonoOS_NetworkCallback, user_data: *mut c_void) -> c_int;
    pub fn monoos_net_resolve(
        hostname: *const c_char,
        callback: MonoOS_DnsCallback,
        user_data: *mut c_void,
    ) -> c_int;

    pub fn monoos_audio_open_stream(
        sample_rate: u32,
        channels: u32,
        usage: MonoOS_AudioUsage,
        volume: f32,
    ) -> MonoOS_AudioStreamHandle;
    pub fn monoos_audio_write(handle: MonoOS_AudioStreamHandle, data: *const f32, frames: u32) -> c_int;
    pub fn monoos_audio_set_volume(handle: MonoOS_AudioStreamHandle, volume: f32) -> c_int;
    pub fn monoos_audio_close_stream(handle: MonoOS_AudioStreamHandle);
    pub fn monoos_audio_set_master_volume(volume: f32) -> c_int;

    // ── UI (runtime QML loader, called by app templates) ──────────────────
    pub fn monoos_ui_load_qml(qml_path: *const c_char) -> c_int;
    pub fn monoos_ui_reload() -> c_int;
    pub fn monoos_ui_set_status_bar_visible(visible: bool);
    pub fn monoos_ui_set_edge_to_edge(enabled: bool);

    // ── Media playback ──────────────────────────────────────────────────────
    pub fn monoos_player_create() -> *mut MonoOS_Player;
    pub fn monoos_player_destroy(player: *mut MonoOS_Player);
    pub fn monoos_player_set_uri(player: *mut MonoOS_Player, uri: *const c_char) -> c_int;
    pub fn monoos_player_prepare(player: *mut MonoOS_Player) -> c_int;
    pub fn monoos_player_start(player: *mut MonoOS_Player) -> c_int;
    pub fn monoos_player_pause(player: *mut MonoOS_Player) -> c_int;
    pub fn monoos_player_stop(player: *mut MonoOS_Player) -> c_int;
    pub fn monoos_player_seek(player: *mut MonoOS_Player, pos_ms: u64) -> c_int;
    pub fn monoos_player_set_volume(player: *mut MonoOS_Player, volume: f32) -> c_int;
    pub fn monoos_player_set_rate(player: *mut MonoOS_Player, rate: f32) -> c_int;
    pub fn monoos_player_set_looping(player: *mut MonoOS_Player, looping: bool);
    pub fn monoos_player_position(player: *const MonoOS_Player) -> u64;
    pub fn monoos_player_duration(player: *const MonoOS_Player) -> u64;
    pub fn monoos_player_state(player: *const MonoOS_Player) -> MonoOS_PlayerState;
    pub fn monoos_player_set_listener(
        player: *mut MonoOS_Player,
        listener: *const MonoOS_PlayerListener,
        user_data: *mut c_void,
    );
}
