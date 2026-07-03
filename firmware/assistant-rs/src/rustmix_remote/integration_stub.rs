#![allow(dead_code)]

//! Integration notes for Rustmix-Wave.
//!
//! This file is a reference stub only. Do not wire it into production until the
//! BLE stack location and existing main-loop event route are confirmed.

use super::rrbp::{RemoteEvent, RrbpParser};

pub struct RustmixRemoteBridge {
    parser: RrbpParser,
}

impl RustmixRemoteBridge {
    pub fn new() -> Self {
        Self { parser: RrbpParser::new() }
    }

    /// Called from the BLE command characteristic write callback.
    ///
    /// Production code should enqueue the returned event into the existing
    /// firmware event queue instead of directly changing reader state.
    pub fn on_ble_command_write(&mut self, bytes: &[u8]) -> Option<RemoteEvent> {
        match self.parser.parse(bytes) {
            Ok(event) => event,
            Err(_) => None,
        }
    }
}
