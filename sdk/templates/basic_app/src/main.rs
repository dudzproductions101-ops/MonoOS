//! main.rs – OneOS Basic App Template
//!
//! Entry point for a minimal OneOS application.
//! Replace this file with your own application logic.

// The OneOS runtime calls oneos_app_main() on the UI thread after the
// application context has been created and the window is ready.

/// Application entry point called by the OneOS runtime.
///
/// `ctx` is a raw pointer to the C-level `OneOS_Context` created by the
/// runtime.  In a real app you would wrap it in a safe Rust struct.
#[no_mangle]
pub extern "C" fn oneos_app_main(ctx: *mut core::ffi::c_void) {
    // Safety: ctx is valid for the lifetime of the application.
    let _ = ctx;

    // Load the QML UI.  The path is relative to the package root.
    oneos_sdk::ui::load_qml("res/qml/Main.qml");

    // The event loop is managed by the runtime; this function returns
    // immediately and the runtime continues on the UI thread.
}

/// Called by the runtime when the app is brought to the foreground.
#[no_mangle]
pub extern "C" fn oneos_app_resume() {}

/// Called by the runtime when the app moves to the background.
#[no_mangle]
pub extern "C" fn oneos_app_pause() {}

/// Called by the runtime just before the app process is killed.
#[no_mangle]
pub extern "C" fn oneos_app_destroy() {}
