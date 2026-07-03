//! Rustmix Remote BLE Protocol v1 parser.
//!
//! RRBP packets are intentionally tiny so a BLE write callback can parse them
//! quickly and enqueue a `RemoteEvent` without touching reader/UI state.

pub const RRBP_PROTOCOL_VERSION: u8 = 0x01;
pub const RRBP_PACKET_LEN: usize = 6;

pub const FLAG_LONG_PRESS: u8 = 0x01;
pub const FLAG_REPEAT: u8 = 0x02;
pub const FLAG_ROTARY: u8 = 0x04;
pub const FLAG_HIGH_PRIORITY: u8 = 0x08;
pub const FLAG_REQUIRE_ACK: u8 = 0x10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteEvent {
    PageNext,
    PagePrevious,
    Select,
    Back,
    Menu,
    Sleep,
    Wake,
    ScrollUp,
    ScrollDown,
    NextChapterOrFile,
    PreviousChapterOrFile,
    ToggleBookmark,
    Refresh,
}

impl RemoteEvent {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::PageNext => "page-next",
            Self::PagePrevious => "page-previous",
            Self::Select => "select",
            Self::Back => "back",
            Self::Menu => "menu",
            Self::Sleep => "sleep",
            Self::Wake => "wake",
            Self::ScrollUp => "scroll-up",
            Self::ScrollDown => "scroll-down",
            Self::NextChapterOrFile => "next-chapter-or-file",
            Self::PreviousChapterOrFile => "previous-chapter-or-file",
            Self::ToggleBookmark => "toggle-bookmark",
            Self::Refresh => "refresh",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RrbpError {
    PacketTooShort { got: usize },
    PacketTooLong { got: usize },
    UnsupportedVersion { got: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RrbpPacket {
    pub version: u8,
    pub sequence: u8,
    pub command: u8,
    pub flags: u8,
    pub parameter: u8,
    pub reserved: u8,
}

impl RrbpPacket {
    pub fn decode(bytes: &[u8]) -> Result<Self, RrbpError> {
        if bytes.len() < RRBP_PACKET_LEN {
            return Err(RrbpError::PacketTooShort { got: bytes.len() });
        }
        if bytes.len() > RRBP_PACKET_LEN {
            return Err(RrbpError::PacketTooLong { got: bytes.len() });
        }
        if bytes[0] != RRBP_PROTOCOL_VERSION {
            return Err(RrbpError::UnsupportedVersion { got: bytes[0] });
        }
        Ok(Self {
            version: bytes[0],
            sequence: bytes[1],
            command: bytes[2],
            flags: bytes[3],
            parameter: bytes[4],
            reserved: bytes[5],
        })
    }

    #[must_use]
    pub const fn event(self) -> Option<RemoteEvent> {
        match self.command {
            0x01 => Some(RemoteEvent::PageNext),
            0x02 => Some(RemoteEvent::PagePrevious),
            0x03 => Some(RemoteEvent::Select),
            0x04 => Some(RemoteEvent::Back),
            0x05 => Some(RemoteEvent::Menu),
            0x06 => Some(RemoteEvent::Sleep),
            0x07 => Some(RemoteEvent::Wake),
            0x08 => Some(RemoteEvent::ScrollUp),
            0x09 => Some(RemoteEvent::ScrollDown),
            0x0A => Some(RemoteEvent::NextChapterOrFile),
            0x0B => Some(RemoteEvent::PreviousChapterOrFile),
            0x0C => Some(RemoteEvent::ToggleBookmark),
            0x0D => Some(RemoteEvent::Refresh),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct RrbpParser {
    last_sequence: Option<u8>,
}

impl RrbpParser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_sequence: None,
        }
    }

    /// Parse a 6-byte RRBP v1 packet.
    ///
    /// Returns:
    /// - `Ok(Some(event))` for a new supported command.
    /// - `Ok(None)` for duplicate sequence numbers or unsupported commands.
    /// - `Err(_)` for malformed packets.
    pub fn parse(&mut self, bytes: &[u8]) -> Result<Option<RemoteEvent>, RrbpError> {
        let packet = RrbpPacket::decode(bytes)?;
        if self.last_sequence == Some(packet.sequence) {
            return Ok(None);
        }
        self.last_sequence = Some(packet.sequence);
        Ok(packet.event())
    }

    pub fn reset_sequence_tracking(&mut self) {
        self.last_sequence = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_page_next() {
        let mut parser = RrbpParser::new();
        let event = parser.parse(&[0x01, 0x00, 0x01, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(event, Some(RemoteEvent::PageNext));
    }

    #[test]
    fn parses_page_previous() {
        let mut parser = RrbpParser::new();
        let event = parser.parse(&[0x01, 0x01, 0x02, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(event, Some(RemoteEvent::PagePrevious));
    }

    #[test]
    fn drops_duplicate_sequence() {
        let mut parser = RrbpParser::new();
        assert_eq!(
            parser.parse(&[0x01, 0x07, 0x01, 0x00, 0x00, 0x00]).unwrap(),
            Some(RemoteEvent::PageNext)
        );
        assert_eq!(
            parser.parse(&[0x01, 0x07, 0x02, 0x00, 0x00, 0x00]).unwrap(),
            None
        );
    }

    #[test]
    fn ignores_unknown_command() {
        let mut parser = RrbpParser::new();
        assert_eq!(
            parser.parse(&[0x01, 0x02, 0xFF, 0x00, 0x00, 0x00]).unwrap(),
            None
        );
    }

    #[test]
    fn rejects_bad_version() {
        let mut parser = RrbpParser::new();
        assert_eq!(
            parser.parse(&[0x02, 0x00, 0x01, 0x00, 0x00, 0x00]),
            Err(RrbpError::UnsupportedVersion { got: 0x02 })
        );
    }

    #[test]
    fn rejects_short_packet() {
        let mut parser = RrbpParser::new();
        assert_eq!(
            parser.parse(&[0x01, 0x00, 0x01]),
            Err(RrbpError::PacketTooShort { got: 3 })
        );
    }

    #[test]
    fn rejects_long_packet() {
        let mut parser = RrbpParser::new();
        assert_eq!(
            parser.parse(&[0x01, 0x00, 0x01, 0, 0, 0, 0]),
            Err(RrbpError::PacketTooLong { got: 7 })
        );
    }
}
