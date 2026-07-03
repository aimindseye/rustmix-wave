//! RRBP parser-to-queue bridge.

use super::{RemoteEventQueue, RrbpError, RrbpParser};

#[derive(Debug)]
pub enum RemoteBridgeWriteOutcome {
    Enqueued,
    DuplicateOrUnsupported,
    InvalidPacket(RrbpError),
}

pub struct RustmixRemoteBridge {
    parser: RrbpParser,
    queue: RemoteEventQueue,
}

impl RustmixRemoteBridge {
    #[must_use]
    pub fn new(queue: RemoteEventQueue) -> Self {
        Self {
            parser: RrbpParser::new(),
            queue,
        }
    }

    /// Called from the BLE Command characteristic write callback.
    ///
    /// This method only parses and enqueues; it never mutates reader/UI state.
    pub fn on_command_write(&mut self, bytes: &[u8]) -> RemoteBridgeWriteOutcome {
        match self.parser.parse(bytes) {
            Ok(Some(event)) => {
                self.queue.push(event);
                RemoteBridgeWriteOutcome::Enqueued
            }
            Ok(None) => RemoteBridgeWriteOutcome::DuplicateOrUnsupported,
            Err(error) => RemoteBridgeWriteOutcome::InvalidPacket(error),
        }
    }

    pub fn reset_sequence_tracking(&mut self) {
        self.parser.reset_sequence_tracking();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rustmix_remote::RemoteEvent;

    #[test]
    fn bridge_enqueues_page_turn() {
        let queue = RemoteEventQueue::default();
        let mut bridge = RustmixRemoteBridge::new(queue.clone());
        assert!(matches!(
            bridge.on_command_write(&[0x01, 0x05, 0x01, 0, 0, 0]),
            RemoteBridgeWriteOutcome::Enqueued
        ));
        assert_eq!(queue.pop(), Some(RemoteEvent::PageNext));
    }
}
