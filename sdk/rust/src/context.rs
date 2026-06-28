//! context.rs – Safe wrapper around `MonoOS_Context`.

use crate::sys;
use std::ffi::{CStr, CString};

/// An application context, created once at app startup.
///
/// Wraps the heap-allocated `MonoOS_Context*` returned by the runtime and
/// destroys it automatically on drop.
pub struct Context {
    raw: *mut sys::MonoOS_Context,
}

// The underlying C context is only ever touched from the thread that owns
// it in practice (the app's main/UI thread), but the pointer itself has no
// thread-affinity requirement documented in monoos.h, so we allow moving it
// across threads while still requiring exclusive access for mutation.
unsafe impl Send for Context {}

impl Context {
    /// Create a new application context.
    ///
    /// `package_name` should be a reverse-DNS identifier (e.g.
    /// `"com.example.app"`), matching the app's manifest.
    pub fn create(package_name: &str, version_code: u32) -> Option<Self> {
        let c_name = CString::new(package_name).ok()?;
        let raw = unsafe { sys::monoos_context_create(c_name.as_ptr(), version_code) };
        if raw.is_null() {
            None
        } else {
            Some(Context { raw })
        }
    }

    /// The package name this context was created with.
    pub fn package_name(&self) -> String {
        unsafe {
            let ptr = sys::monoos_context_package_name(self.raw);
            if ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }

    /// Raw pointer access for FFI interop (e.g. passing to other native
    /// libraries that expect `MonoOS_Context*`). Prefer the safe API above.
    pub fn as_raw(&self) -> *mut sys::MonoOS_Context {
        self.raw
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { sys::monoos_context_destroy(self.raw) };
    }
}
