//! main.rs – MonoOS Basic App Template
//!
//! Entry point for a minimal MonoOS application.
//! Replace this file with your own application logic.

// The MonoOS runtime calls monoos_app_main() on the UI thread after the
// application context has been created and the window is ready.

/// Application entry point called by the MonoOS runtime.
///
/// `ctx` is a raw pointer to the C-level `MonoOS_Context` created by the
/// runtime.  In a real app you would wrap it in a safe Rust struct.
#[no_mangle]
pub extern "C" fn monoos_app_main(ctx: *mut core::ffi::c_void) {
    // Safety: ctx is valid for the lifetime of the application.
    let _ = ctx;

    // Load the QML UI.  The path is relative to the package root.
    if let Err(e) = monoos_sdk::ui::load_qml("res/qml/Main.qml") {
        eprintln!("[basic_app] failed to load UI: {e}");
    }

    // The event loop is managed by the runtime; this function returns
    // immediately and the runtime continues on the UI thread.
}

/// Called by the runtime when the app is brought to the foreground.
#[no_mangle]
pub extern "C" fn monoos_app_resume() {}

/// Called by the runtime when the app moves to the background.
#[no_mangle]
pub extern "C" fn monoos_app_pause() {}

/// Called by the runtime just before the app process is killed.
#[no_mangle]
pub extern "C" fn monoos_app_destroy() {}
