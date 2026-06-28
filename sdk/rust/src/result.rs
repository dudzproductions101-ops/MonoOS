//! result.rs – Safe `Result<T, MonoOsError>` wrapper around raw `MonoOS_Result`
//! codes returned by the C ABI.

use crate::sys;
use std::fmt;

/// An error returned by a MonoOS SDK call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonoOsError {
    Generic,
    InvalidArg,
    PermissionDenied,
    NotFound,
    AlreadyExists,
    NoMemory,
    Io,
    Timeout,
    NotSupported,
    NotInitialised,
    /// A raw code the wrapper didn't recognise (forward-compat).
    Unknown(i32),
}

impl MonoOsError {
    pub(crate) fn from_code(code: sys::MonoOS_Result) -> Self {
        match code {
            sys::MONOOS_ERROR => MonoOsError::Generic,
            sys::MONOOS_ERROR_INVALID_ARG => MonoOsError::InvalidArg,
            sys::MONOOS_ERROR_PERMISSION_DENIED => MonoOsError::PermissionDenied,
            sys::MONOOS_ERROR_NOT_FOUND => MonoOsError::NotFound,
            sys::MONOOS_ERROR_ALREADY_EXISTS => MonoOsError::AlreadyExists,
            sys::MONOOS_ERROR_NO_MEMORY => MonoOsError::NoMemory,
            sys::MONOOS_ERROR_IO => MonoOsError::Io,
            sys::MONOOS_ERROR_TIMEOUT => MonoOsError::Timeout,
            sys::MONOOS_ERROR_NOT_SUPPORTED => MonoOsError::NotSupported,
            sys::MONOOS_ERROR_NOT_INITIALISED => MonoOsError::NotInitialised,
            other => MonoOsError::Unknown(other),
        }
    }
}

impl fmt::Display for MonoOsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MonoOsError::Generic => "generic MonoOS error",
            MonoOsError::InvalidArg => "invalid argument",
            MonoOsError::PermissionDenied => "permission denied",
            MonoOsError::NotFound => "not found",
            MonoOsError::AlreadyExists => "already exists",
            MonoOsError::NoMemory => "out of memory",
            MonoOsError::Io => "I/O error",
            MonoOsError::Timeout => "timed out",
            MonoOsError::NotSupported => "not supported",
            MonoOsError::NotInitialised => "not initialised",
            MonoOsError::Unknown(c) => return write!(f, "unknown MonoOS error ({c})"),
        };
        f.write_str(s)
    }
}

impl std::error::Error for MonoOsError {}

pub type MonoOsResult<T> = Result<T, MonoOsError>;

/// Convert a raw `MonoOS_Result` into `Ok(())` or `Err(MonoOsError)`.
pub(crate) fn check(code: sys::MonoOS_Result) -> MonoOsResult<()> {
    if code == sys::MONOOS_OK {
        Ok(())
    } else {
        Err(MonoOsError::from_code(code))
    }
}
