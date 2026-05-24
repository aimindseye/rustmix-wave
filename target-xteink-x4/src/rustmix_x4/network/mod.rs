#![allow(dead_code)]

//! Rustmix-owned Wi-Fi runtime for Xteink X4.
//!
//! Active Wi-Fi setup, scan, transfer, and network-time code lives here.
//! `target-xteink-x4/src/rustmix_x4` remains an imported compatibility/runtime reference and
//! must not receive new Wi-Fi features.

pub mod biscuit_wifi;
pub mod network_time;
pub mod time_status;
pub mod upload;
pub mod wifi_scan;

pub const RUSTMIX_WIFI_RUNTIME_MARKER: &str = "rustmix-wifi-runtime-owned-ok";

pub struct RustmixWifiRuntimeOwnership;

impl RustmixWifiRuntimeOwnership {
    pub const fn marker() -> &'static str {
        RUSTMIX_WIFI_RUNTIME_MARKER
    }
}
