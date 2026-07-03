//! Rustmix Remote support.
//!
//! The host-testable RRBP parser and queue are always compiled. The ESP-IDF BLE
//! GATT server is compiled only with `--features rustmix-remote-ble`.

pub mod bridge;
pub mod queue;
pub mod rrbp;

#[cfg(all(target_os = "espidf", feature = "rustmix-remote-ble"))]
pub mod ble_gatt;

pub use bridge::{RemoteBridgeWriteOutcome, RustmixRemoteBridge};
pub use queue::{RemoteEventQueue, REMOTE_EVENT_QUEUE_CAPACITY};
pub use rrbp::{RemoteEvent, RrbpError, RrbpParser, RRBP_PACKET_LEN, RRBP_PROTOCOL_VERSION};
