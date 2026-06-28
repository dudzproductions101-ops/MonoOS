//! ui.rs – Safe wrapper around `monoos_ui.h`.

use crate::result::{check, MonoOsResult};
use crate::sys;
use std::ffi::CString;

/// Load and display a QML document as the app's root UI surface.
///
/// `qml_path` is relative to the package root, e.g. `"res/qml/Main.qml"`.
pub fn load_qml(qml_path: &str) -> MonoOsResult<()> {
    let c_path = CString::new(qml_path).map_err(|_| crate::result::MonoOsError::InvalidArg)?;
    check(unsafe { sys::monoos_ui_load_qml(c_path.as_ptr()) })
}

/// Reload the currently displayed QML document. Primarily useful during
/// development for fast iteration.
pub fn reload() -> MonoOsResult<()> {
    check(unsafe { sys::monoos_ui_reload() })
}

/// Show or hide the system status bar over this app's window.
pub fn set_status_bar_visible(visible: bool) {
    unsafe { sys::monoos_ui_set_status_bar_visible(visible) }
}

/// Request the app's window be drawn edge-to-edge, behind system bars.
pub fn set_edge_to_edge(enabled: bool) {
    unsafe { sys::monoos_ui_set_edge_to_edge(enabled) }
}
