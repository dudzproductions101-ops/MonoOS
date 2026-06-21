//! permissions.rs – Safe wrapper around `monoos_permissions.h`.

use crate::result::{check, MonoOsResult};
use crate::sys;
use std::os::raw::c_void;

/// A single runtime permission. Mirrors the `MONOOS_PERM_*` bitmask values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Permission {
    Camera = sys::MONOOS_PERM_CAMERA,
    Microphone = sys::MONOOS_PERM_MICROPHONE,
    Location = sys::MONOOS_PERM_LOCATION,
    Contacts = sys::MONOOS_PERM_CONTACTS,
    Storage = sys::MONOOS_PERM_STORAGE,
    Phone = sys::MONOOS_PERM_PHONE,
    Bluetooth = sys::MONOOS_PERM_BLUETOOTH,
    Nfc = sys::MONOOS_PERM_NFC,
    Sensors = sys::MONOOS_PERM_SENSORS,
    Network = sys::MONOOS_PERM_NETWORK,
    Notifications = sys::MONOOS_PERM_NOTIFICATIONS,
}

impl Permission {
    fn bits(self) -> sys::MonoOS_Permission {
        self as sys::MonoOS_Permission
    }
}

/// Whether the user has granted, denied, or not yet decided on a permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantState {
    NotRequested,
    Granted,
    Denied,
    /// User checked "Don't ask again" — re-requesting will not show a dialog.
    PermanentlyDenied,
}

impl From<sys::MonoOS_GrantState> for GrantState {
    fn from(s: sys::MonoOS_GrantState) -> Self {
        match s {
            sys::MonoOS_GrantState::NotRequested => GrantState::NotRequested,
            sys::MonoOS_GrantState::Granted => GrantState::Granted,
            sys::MonoOS_GrantState::Denied => GrantState::Denied,
            sys::MonoOS_GrantState::PermDenied => GrantState::PermanentlyDenied,
        }
    }
}

/// Returns `Ok(())` if the permission is currently granted.
pub fn check_permission(permission: Permission) -> MonoOsResult<()> {
    check(unsafe { sys::monoos_check_permission(permission.bits()) })
}

/// Query the current grant state without triggering a system dialog.
pub fn permission_state(permission: Permission) -> GrantState {
    unsafe { sys::monoos_permission_state(permission.bits()) }.into()
}

/// Request a single runtime permission. `on_result` is called once with the
/// outcome — synchronously if already decided, or after the user responds
/// to the system dialog otherwise.
///
/// # Notes
/// The closure is boxed and leaked into a raw pointer for the duration of
/// the C callback round-trip; it is freed inside the trampoline regardless
/// of which path the runtime takes.
pub fn request_permission<F>(permission: Permission, on_result: F)
where
    F: FnOnce(bool) + 'static,
{
    let boxed: Box<dyn FnOnce(bool)> = Box::new(on_result);
    let ctx = Box::into_raw(Box::new(boxed));
    unsafe {
        sys::monoos_request_permission(permission.bits(), trampoline, ctx as *mut c_void);
    }
}

/// Request multiple permissions at once. `on_each` is invoked once per
/// permission in `permissions`, in the order the runtime reports them.
pub fn request_permissions<F>(permissions: &[Permission], on_each: F)
where
    F: Fn(Permission, bool) + 'static,
{
    let bits: Vec<sys::MonoOS_Permission> = permissions.iter().map(|p| p.bits()).collect();
    let remaining = bits.len();
    let ctx = Box::into_raw(Box::new(MultiCtx {
        callback: Box::new(on_each),
        remaining,
    }));
    unsafe {
        sys::monoos_request_permissions(
            bits.as_ptr(),
            bits.len(),
            trampoline_multi,
            ctx as *mut c_void,
        );
    }
}

struct MultiCtx {
    callback: Box<dyn Fn(Permission, bool)>,
    remaining: usize,
}

extern "C" fn trampoline(_permission: sys::MonoOS_Permission, granted: bool, user_data: *mut c_void) {
    if user_data.is_null() {
        return;
    }
    let boxed: Box<Box<dyn FnOnce(bool)>> = unsafe { Box::from_raw(user_data as *mut Box<dyn FnOnce(bool)>) };
    (boxed)(granted);
}

extern "C" fn trampoline_multi(permission: sys::MonoOS_Permission, granted: bool, user_data: *mut c_void) {
    if user_data.is_null() {
        return;
    }
    // SAFETY: the runtime invokes this once per permission in the batch,
    // always with the same `user_data` pointer, and (per monoos_permissions.h)
    // always on the calling/main thread sequentially — so a plain mutable
    // borrow followed by freeing on the last call is sound. We only take
    // ownership (and free) once `remaining` reaches zero.
    let ctx: &mut MultiCtx = unsafe { &mut *(user_data as *mut MultiCtx) };
    if let Some(p) = bits_to_permission(permission) {
        (ctx.callback)(p, granted);
    }
    ctx.remaining = ctx.remaining.saturating_sub(1);
    if ctx.remaining == 0 {
        unsafe { drop(Box::from_raw(user_data as *mut MultiCtx)) };
    }
}

fn bits_to_permission(bits: sys::MonoOS_Permission) -> Option<Permission> {
    Some(match bits {
        sys::MONOOS_PERM_CAMERA => Permission::Camera,
        sys::MONOOS_PERM_MICROPHONE => Permission::Microphone,
        sys::MONOOS_PERM_LOCATION => Permission::Location,
        sys::MONOOS_PERM_CONTACTS => Permission::Contacts,
        sys::MONOOS_PERM_STORAGE => Permission::Storage,
        sys::MONOOS_PERM_PHONE => Permission::Phone,
        sys::MONOOS_PERM_BLUETOOTH => Permission::Bluetooth,
        sys::MONOOS_PERM_NFC => Permission::Nfc,
        sys::MONOOS_PERM_SENSORS => Permission::Sensors,
        sys::MONOOS_PERM_NETWORK => Permission::Network,
        sys::MONOOS_PERM_NOTIFICATIONS => Permission::Notifications,
        _ => return None,
    })
}
