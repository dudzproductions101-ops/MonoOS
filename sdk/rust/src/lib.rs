//! monoos-sdk – Safe Rust bindings for the MonoOS application SDK.
//!
//! This crate mirrors the C ABI declared in `sdk/api/*.h` one module per
//! header, plus a `sys` module exposing the raw `extern "C"` declarations
//! for interop with other native code.
//!
//! # Example
//! ```no_run
//! use monoos_sdk::{context::Context, permissions::Permission};
//!
//! let ctx = Context::create("com.example.app", 1).expect("context");
//! monoos_sdk::permissions::request_permission(Permission::Camera, |granted| {
//!     if granted {
//!         println!("camera access granted");
//!     }
//! });
//! ```

pub mod audio;
pub mod context;
pub mod media;
pub mod network;
pub mod notifications;
pub mod permissions;
pub mod result;
pub mod storage;
pub mod sys;
pub mod ui;

#[cfg(feature = "mock-runtime")]
pub mod mock_runtime;

pub use context::Context;
pub use result::{MonoOsError, MonoOsResult};
