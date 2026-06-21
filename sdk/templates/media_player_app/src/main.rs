//! main.rs – Media Player App Template
//!
//! Demonstrates use of the OneOS Media Player API.

use oneos_sdk::media::{Player, PlayerState};
use oneos_sdk::storage::{MediaStore, MediaType};

#[no_mangle]
pub extern "C" fn oneos_app_main(_ctx: *mut core::ffi::c_void) {
    oneos_sdk::ui::load_qml("res/qml/PlayerUI.qml");
}

/// Called from QML when the user taps a track.
#[no_mangle]
pub extern "C" fn play_track(uri_cstr: *const core::ffi::c_char) {
    let uri = unsafe { std::ffi::CStr::from_ptr(uri_cstr) }
        .to_str().unwrap_or_default();

    // Real app would reuse a Player instance; simplified here.
    let mut p = Player::new();
    if p.set_uri(uri).is_ok() && p.prepare().is_ok() {
        let _ = p.start();
    }
}
