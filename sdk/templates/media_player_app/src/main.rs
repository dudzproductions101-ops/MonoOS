//! main.rs – Media Player App Template
//!
//! Demonstrates use of the MonoOS Media Player and Media Store APIs.

use monoos_sdk::media::{Player, PlayerListener, PlayerState};
use monoos_sdk::storage::{MediaStore, MediaType};
use std::sync::Mutex;

/// The app keeps a single shared `Player` instance alive for its lifetime,
/// rather than creating a new one per track (which would drop/recreate the
/// underlying native player on every tap).
static PLAYER: Mutex<Option<Player>> = Mutex::new(None);

#[no_mangle]
pub extern "C" fn monoos_app_main(_ctx: *mut core::ffi::c_void) {
    if let Err(e) = monoos_sdk::ui::load_qml("res/qml/PlayerUI.qml") {
        eprintln!("[media_player] failed to load UI: {e}");
        return;
    }

    let mut player = Player::new();
    player.set_listener(PlayerListener {
        on_state: Some(Box::new(|state| {
            if state == PlayerState::Complete {
                eprintln!("[media_player] playback complete");
            }
        })),
        on_error: Some(Box::new(|code, msg| {
            eprintln!("[media_player] playback error {code}: {msg}");
        })),
        ..Default::default()
    });

    *PLAYER.lock().unwrap() = Some(player);

    // Populate the QML track list from the shared audio media store.
    // Requires the user to have granted the Storage permission; in a real
    // app this would be requested up front via monoos_sdk::permissions.
    let store = MediaStore::new();
    match store.query(MediaType::Audio) {
        Ok(tracks) => eprintln!("[media_player] found {} track(s)", tracks.len()),
        Err(e) => eprintln!("[media_player] could not query media store: {e}"),
    }
}

/// Called from QML when the user taps a track.
#[no_mangle]
pub extern "C" fn play_track(uri_cstr: *const core::ffi::c_char) {
    if uri_cstr.is_null() {
        return;
    }
    let uri = match unsafe { std::ffi::CStr::from_ptr(uri_cstr) }.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut guard = PLAYER.lock().unwrap();
    let player = guard.get_or_insert_with(Player::new);

    if let Err(e) = player.set_uri(uri) {
        eprintln!("[media_player] set_uri failed: {e}");
        return;
    }
    if let Err(e) = player.prepare() {
        eprintln!("[media_player] prepare failed: {e}");
        return;
    }
    if let Err(e) = player.start() {
        eprintln!("[media_player] start failed: {e}");
    }
}

/// Called from QML when the user taps pause.
#[no_mangle]
pub extern "C" fn pause_track() {
    if let Some(player) = PLAYER.lock().unwrap().as_mut() {
        let _ = player.pause();
    }
}
