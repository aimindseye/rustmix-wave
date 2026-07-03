//! Rustmix Remote integration scaffold.
//!
//! This module is intentionally self-contained for r1. It provides the RRBP
//! parser and RemoteEvent definitions. BLE stack wiring and main-loop routing
//! should be added behind the Rustmix-Wave BLE remote feature gate.

pub mod rrbp;

pub use rrbp::{RemoteEvent, RrbpError, RrbpParser, RRBP_PACKET_LEN, RRBP_PROTOCOL_VERSION};
