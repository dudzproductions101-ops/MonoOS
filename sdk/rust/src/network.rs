//! network.rs – Safe wrapper around `monoos_network.h`.

use crate::result::{check, MonoOsResult};
use crate::sys;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkType {
    None,
    Wifi,
    Cellular,
    Ethernet,
    Vpn,
    Bluetooth,
}

impl From<sys::MonoOS_NetworkType> for NetworkType {
    fn from(t: sys::MonoOS_NetworkType) -> Self {
        match t {
            sys::MonoOS_NetworkType::None => NetworkType::None,
            sys::MonoOS_NetworkType::Wifi => NetworkType::Wifi,
            sys::MonoOS_NetworkType::Cellular => NetworkType::Cellular,
            sys::MonoOS_NetworkType::Ethernet => NetworkType::Ethernet,
            sys::MonoOS_NetworkType::Vpn => NetworkType::Vpn,
            sys::MonoOS_NetworkType::Bluetooth => NetworkType::Bluetooth,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkState {
    pub connected: bool,
    pub net_type: NetworkType,
    pub metered: bool,
    pub roaming: bool,
    pub signal_strength: i32,
    pub ssid: String,
}

impl From<&sys::MonoOS_NetworkState> for NetworkState {
    fn from(s: &sys::MonoOS_NetworkState) -> Self {
        let bytes: &[u8] = unsafe { std::slice::from_raw_parts(s.ssid.as_ptr() as *const u8, s.ssid.len()) };
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        NetworkState {
            connected: s.connected,
            net_type: s.net_type.into(),
            metered: s.metered,
            roaming: s.roaming,
            signal_strength: s.signal_strength,
            ssid: String::from_utf8_lossy(&bytes[..len]).into_owned(),
        }
    }
}

/// Synchronously query the current connectivity state. Requires
/// [`crate::permissions::Permission::Network`].
pub fn get_state() -> MonoOsResult<NetworkState> {
    let mut raw = sys::MonoOS_NetworkState {
        connected: false,
        net_type: sys::MonoOS_NetworkType::None,
        metered: false,
        roaming: false,
        signal_strength: 0,
        ssid: [0; 64],
    };
    check(unsafe { sys::monoos_net_get_state(&mut raw) })?;
    Ok((&raw).into())
}

/// Register a callback invoked on every connectivity change. The closure
/// lives for the remainder of the process (matching the C API, which has
/// no "unregister with a handle" mechanism — pass through `monoos_net_listen`
/// with a null/no-op callback to stop receiving updates, per monoos_network.h).
pub fn listen<F>(callback: F) -> MonoOsResult<()>
where
    F: Fn(&NetworkState) + 'static,
{
    let boxed: Box<dyn Fn(&NetworkState)> = Box::new(callback);
    let ctx = Box::into_raw(Box::new(boxed));
    check(unsafe { sys::monoos_net_listen(listen_trampoline, ctx as *mut c_void) })
}

extern "C" fn listen_trampoline(state: *const sys::MonoOS_NetworkState, user_data: *mut c_void) {
    if state.is_null() || user_data.is_null() {
        return;
    }
    let boxed: &Box<dyn Fn(&NetworkState)> = unsafe { &*(user_data as *const Box<dyn Fn(&NetworkState)>) };
    let owned: NetworkState = unsafe { &*state }.into();
    (boxed)(&owned);
}

/// Resolve a hostname via the MonoOS privacy-preserving DNS-over-HTTPS
/// resolver. `on_result` receives `Ok(addrs)` or `Err(code)` on failure.
pub fn resolve<F>(hostname: &str, on_result: F) -> MonoOsResult<()>
where
    F: FnOnce(Result<Vec<String>, i32>) + 'static,
{
    let c_host = CString::new(hostname).map_err(|_| crate::result::MonoOsError::InvalidArg)?;
    let boxed: Box<dyn FnOnce(Result<Vec<String>, i32>)> = Box::new(on_result);
    let ctx = Box::into_raw(Box::new(boxed));
    unsafe {
        sys::monoos_net_resolve(c_host.as_ptr(), resolve_trampoline, ctx as *mut c_void);
    }
    Ok(())
}

extern "C" fn resolve_trampoline(
    addrs: *const *const c_char,
    count: std::os::raw::c_int,
    err: std::os::raw::c_int,
    user_data: *mut c_void,
) {
    if user_data.is_null() {
        return;
    }
    let boxed: Box<Box<dyn FnOnce(Result<Vec<String>, i32>)>> =
        unsafe { Box::from_raw(user_data as *mut Box<dyn FnOnce(Result<Vec<String>, i32>)>) };

    if err != 0 || addrs.is_null() {
        (boxed)(Err(err));
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts(addrs, count.max(0) as usize) };
    let strings: Vec<String> = slice
        .iter()
        .filter(|p| !p.is_null())
        .map(|&p| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
        .collect();
    (boxed)(Ok(strings));
}
