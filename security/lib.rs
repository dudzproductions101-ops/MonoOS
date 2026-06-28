//! monoos_security – MonoOS privacy engine.
//!
//! Brings together the independent privacy-protection modules into one
//! crate. Each module is self-contained (no cross-module dependencies);
//! they are unified here so the rest of the OS (settings UI, package
//! manager, framework) has a single crate to depend on.

#[path = "camera_monitor/camera_monitor.rs"]
pub mod camera_monitor;

#[path = "microphone_monitor/microphone_monitor.rs"]
pub mod microphone_monitor;

#[path = "network_monitor/network_monitor.rs"]
pub mod network_monitor;

#[path = "privacy_dashboard/privacy_dashboard.rs"]
pub mod privacy_dashboard;

#[path = "telemetry_guard/telemetry_guard.rs"]
pub mod telemetry_guard;

#[path = "tracker_blocker/tracker_blocker.rs"]
pub mod tracker_blocker;

pub use monoos_crypto as crypto;
