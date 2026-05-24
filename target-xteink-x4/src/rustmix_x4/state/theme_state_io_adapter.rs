//! Theme State I/O Adapter.
//!
//! This module is deliberately pure and hardware-free. It defines the narrow
//! boundary between Rustmix reader theme/layout state and whichever storage
//! implementation owns the physical SD/FAT behavior.
//!
//! Owned here:
//! - `state/<BOOKID>.THM` path convention
//! - fixed theme record encode/decode
//! - theme-state read/write adapter contract
//! - validation marker
//!
//! Not owned here:
//! - SD-card access
//! - SPI arbitration
//! - FAT/filesystem implementation
//! - display rendering
//! - live theme application

#![allow(dead_code)]

use super::progress_state_io_adapter::{BOOK_ID_8_3_LEN, BookId8};

/// Validation marker emitted by validation / boot marker plumbing.
pub const THEME_STATE_IO_ADAPTER_MARKER: &str = "x4-theme-state-io-adapter-ok";

pub const THEME_STATE_DIR: &str = "state";
pub const THEME_STATE_EXTENSION: &str = "THM";
pub const THEME_RECORD_MAGIC: [u8; 4] = *b"VTHM";
pub const THEME_RECORD_VERSION: u8 = 1;

/// magic + version + preset + font_scale + contrast + flags + book_id
/// + margin + line_spacing + reserved + updated + checksum.
pub const THEME_RECORD_LEN: usize = 4 + 1 + 1 + 1 + 1 + 2 + 8 + 2 + 2 + 2 + 8 + 4;
pub const THEME_STATE_PATH_MAX_LEN: usize = "state/".len() + BOOK_ID_8_3_LEN + ".THM".len();

/// Per-book theme preset. These are persisted values, not live UI behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ThemePreset {
    /// Use global/default reader theme.
    System = 0,
    /// Black text on light e-paper background.
    Light = 1,
    /// Inverted/dark-style palette for future devices or custom rendering.
    Dark = 2,
    /// Accessibility-oriented high-contrast mode.
    HighContrast = 3,
    /// Caller-owned custom theme/layout values.
    Custom = 255,
}

impl ThemePreset {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::System),
            1 => Some(Self::Light),
            2 => Some(Self::Dark),
            3 => Some(Self::HighContrast),
            255 => Some(Self::Custom),
            _ => None,
        }
    }
}

/// Theme/layout state persisted per book.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThemeState {
    pub book_id: BookId8,
    pub preset: ThemePreset,
    pub font_scale_percent: u8,
    pub contrast_percent: u8,
    pub flags: u16,
    pub margin_px: u16,
    pub line_spacing_percent: u16,
    pub updated_epoch_seconds: u64,
}

impl ThemeState {
    pub const fn new(
        book_id: BookId8,
        preset: ThemePreset,
        font_scale_percent: u8,
        contrast_percent: u8,
        margin_px: u16,
        line_spacing_percent: u16,
        updated_epoch_seconds: u64,
    ) -> Self {
        Self {
            book_id,
            preset,
            font_scale_percent,
            contrast_percent,
            flags: 0,
            margin_px,
            line_spacing_percent,
            updated_epoch_seconds,
        }
    }

    pub const fn default_for_book(book_id: BookId8) -> Self {
        Self {
            book_id,
            preset: ThemePreset::System,
            font_scale_percent: 100,
            contrast_percent: 100,
            flags: 0,
            margin_px: 8,
            line_spacing_percent: 120,
            updated_epoch_seconds: 0,
        }
    }
}

/// Storage-facing byte I/O boundary for theme records.
///
/// A future concrete storage implementation should implement this trait using the
/// existing X4 storage runtime. This trait intentionally knows nothing about SD/FAT/SPI.
pub trait ThemeStateIo {
    type Error;

    /// Reads a theme record from `path` into `out`.
    ///
    /// Return `Ok(None)` when the file does not exist.
    /// Return `Ok(Some(len))` when bytes were read.
    fn read_theme_record(
        &mut self,
        path: &str,
        out: &mut [u8],
    ) -> Result<Option<usize>, Self::Error>;

    /// Writes `bytes` to `path`, replacing any existing record.
    fn write_theme_record(&mut self, path: &str, bytes: &[u8]) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeStateCodecError {
    BufferTooSmall,
    InvalidLength,
    InvalidMagic,
    UnsupportedVersion(u8),
    InvalidPreset(u8),
    InvalidChecksum,
    InvalidBookId,
    PathBufferTooSmall,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ThemeStateAdapterError<E> {
    Io(E),
    Codec(ThemeStateCodecError),
}

impl<E> From<ThemeStateCodecError> for ThemeStateAdapterError<E> {
    fn from(value: ThemeStateCodecError) -> Self {
        Self::Codec(value)
    }
}

/// Thin theme-state adapter over a caller-provided I/O implementation.
pub struct ThemeStateIoAdapter<IO> {
    io: IO,
}

impl<IO> ThemeStateIoAdapter<IO> {
    pub const fn new(io: IO) -> Self {
        Self { io }
    }

    pub fn into_inner(self) -> IO {
        self.io
    }

    pub fn io_mut(&mut self) -> &mut IO {
        &mut self.io
    }
}

impl<IO> ThemeStateIoAdapter<IO>
where
    IO: ThemeStateIo,
{
    pub fn read_theme(
        &mut self,
        book_id: BookId8,
    ) -> Result<Option<ThemeState>, ThemeStateAdapterError<IO::Error>> {
        let mut path = [0u8; THEME_STATE_PATH_MAX_LEN];
        let path_len = write_theme_state_path(book_id, &mut path)?;
        let path = core::str::from_utf8(&path[..path_len])
            .map_err(|_| ThemeStateCodecError::InvalidBookId)?;

        let mut record = [0u8; THEME_RECORD_LEN];
        let Some(len) = self
            .io
            .read_theme_record(path, &mut record)
            .map_err(ThemeStateAdapterError::Io)?
        else {
            return Ok(None);
        };

        if len != THEME_RECORD_LEN {
            return Err(ThemeStateCodecError::InvalidLength.into());
        }

        let state = decode_theme_state(&record)?;
        if state.book_id != book_id {
            return Err(ThemeStateCodecError::InvalidBookId.into());
        }

        Ok(Some(state))
    }

    pub fn write_theme(
        &mut self,
        state: ThemeState,
    ) -> Result<(), ThemeStateAdapterError<IO::Error>> {
        let mut path = [0u8; THEME_STATE_PATH_MAX_LEN];
        let path_len = write_theme_state_path(state.book_id, &mut path)?;
        let path = core::str::from_utf8(&path[..path_len])
            .map_err(|_| ThemeStateCodecError::InvalidBookId)?;

        let mut record = [0u8; THEME_RECORD_LEN];
        encode_theme_state(state, &mut record)?;

        self.io
            .write_theme_record(path, &record)
            .map_err(ThemeStateAdapterError::Io)
    }
}

pub fn theme_state_io_adapter_marker() -> &'static str {
    THEME_STATE_IO_ADAPTER_MARKER
}

pub fn write_theme_state_path(
    book_id: BookId8,
    out: &mut [u8],
) -> Result<usize, ThemeStateCodecError> {
    if out.len() < THEME_STATE_PATH_MAX_LEN {
        return Err(ThemeStateCodecError::PathBufferTooSmall);
    }

    let mut offset = 0;
    copy_bytes(THEME_STATE_DIR.as_bytes(), out, &mut offset);
    out[offset] = b'/';
    offset += 1;
    copy_bytes(book_id.as_bytes(), out, &mut offset);
    out[offset] = b'.';
    offset += 1;
    copy_bytes(THEME_STATE_EXTENSION.as_bytes(), out, &mut offset);

    Ok(offset)
}

pub fn encode_theme_state(
    state: ThemeState,
    out: &mut [u8],
) -> Result<usize, ThemeStateCodecError> {
    if out.len() < THEME_RECORD_LEN {
        return Err(ThemeStateCodecError::BufferTooSmall);
    }

    let record = &mut out[..THEME_RECORD_LEN];
    record.fill(0);

    record[0..4].copy_from_slice(&THEME_RECORD_MAGIC);
    record[4] = THEME_RECORD_VERSION;
    record[5] = state.preset as u8;
    record[6] = state.font_scale_percent;
    record[7] = state.contrast_percent;
    record[8..10].copy_from_slice(&state.flags.to_le_bytes());
    record[10..18].copy_from_slice(state.book_id.as_bytes());
    record[18..20].copy_from_slice(&state.margin_px.to_le_bytes());
    record[20..22].copy_from_slice(&state.line_spacing_percent.to_le_bytes());
    record[22..24].copy_from_slice(&0u16.to_le_bytes());
    record[24..32].copy_from_slice(&state.updated_epoch_seconds.to_le_bytes());

    let checksum = additive_checksum(&record[..32]);
    record[32..36].copy_from_slice(&checksum.to_le_bytes());

    Ok(THEME_RECORD_LEN)
}

pub fn decode_theme_state(input: &[u8]) -> Result<ThemeState, ThemeStateCodecError> {
    if input.len() != THEME_RECORD_LEN {
        return Err(ThemeStateCodecError::InvalidLength);
    }

    if input[0..4] != THEME_RECORD_MAGIC[..] {
        return Err(ThemeStateCodecError::InvalidMagic);
    }

    let version = input[4];
    if version != THEME_RECORD_VERSION {
        return Err(ThemeStateCodecError::UnsupportedVersion(version));
    }

    let preset_byte = input[5];
    let preset = ThemePreset::from_u8(preset_byte)
        .ok_or(ThemeStateCodecError::InvalidPreset(preset_byte))?;

    let mut book_id = [0u8; BOOK_ID_8_3_LEN];
    book_id.copy_from_slice(&input[10..18]);
    if !book_id.iter().all(|b| is_safe_book_id_byte(*b)) {
        return Err(ThemeStateCodecError::InvalidBookId);
    }

    let expected = u32::from_le_bytes([input[32], input[33], input[34], input[35]]);
    let actual = additive_checksum(&input[..32]);
    if expected != actual {
        return Err(ThemeStateCodecError::InvalidChecksum);
    }

    let flags = u16::from_le_bytes([input[8], input[9]]);
    let margin_px = u16::from_le_bytes([input[18], input[19]]);
    let line_spacing_percent = u16::from_le_bytes([input[20], input[21]]);
    let updated_epoch_seconds = u64::from_le_bytes([
        input[24], input[25], input[26], input[27], input[28], input[29], input[30], input[31],
    ]);

    Ok(ThemeState {
        book_id: BookId8::from_bytes_unchecked(book_id),
        preset,
        font_scale_percent: input[6],
        contrast_percent: input[7],
        flags,
        margin_px,
        line_spacing_percent,
        updated_epoch_seconds,
    })
}

fn additive_checksum(bytes: &[u8]) -> u32 {
    let mut value = 0u32;
    for byte in bytes {
        value = value.wrapping_add(*byte as u32);
        value = value.rotate_left(3) ^ 0x5A5A_A5A5;
    }
    value
}

fn copy_bytes(src: &[u8], dst: &mut [u8], offset: &mut usize) {
    let mut idx = 0;
    while idx < src.len() {
        dst[*offset] = src[idx];
        *offset += 1;
        idx += 1;
    }
}

fn is_safe_book_id_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MemoryThemeIo {
        path: [u8; THEME_STATE_PATH_MAX_LEN],
        path_len: usize,
        bytes: [u8; THEME_RECORD_LEN],
        has_record: bool,
    }

    impl Default for MemoryThemeIo {
        fn default() -> Self {
            Self {
                path: [0u8; THEME_STATE_PATH_MAX_LEN],
                path_len: 0,
                bytes: [0u8; THEME_RECORD_LEN],
                has_record: false,
            }
        }
    }

    impl MemoryThemeIo {
        fn path_str(&self) -> &str {
            core::str::from_utf8(&self.path[..self.path_len]).unwrap()
        }
    }

    impl ThemeStateIo for MemoryThemeIo {
        type Error = ();

        fn read_theme_record(
            &mut self,
            path: &str,
            out: &mut [u8],
        ) -> Result<Option<usize>, Self::Error> {
            assert_eq!(path, self.path_str());
            if !self.has_record {
                return Ok(None);
            }
            out[..THEME_RECORD_LEN].copy_from_slice(&self.bytes);
            Ok(Some(THEME_RECORD_LEN))
        }

        fn write_theme_record(&mut self, path: &str, bytes: &[u8]) -> Result<(), Self::Error> {
            let path_bytes = path.as_bytes();
            assert!(path_bytes.len() <= THEME_STATE_PATH_MAX_LEN);
            self.path.fill(0);
            self.path[..path_bytes.len()].copy_from_slice(path_bytes);
            self.path_len = path_bytes.len();
            self.bytes.copy_from_slice(bytes);
            self.has_record = true;
            Ok(())
        }
    }

    #[test]
    fn path_is_8_3_safe() {
        let mut path = [0u8; THEME_STATE_PATH_MAX_LEN];
        let book_id = BookId8::from_candidate("ALICE001");
        let len = write_theme_state_path(book_id, &mut path).unwrap();
        assert_eq!(
            core::str::from_utf8(&path[..len]).unwrap(),
            "state/ALICE001.THM"
        );
    }

    #[test]
    fn encode_decode_round_trip() {
        let book_id = BookId8::from_candidate("ALICE001");
        let mut state = ThemeState::new(book_id, ThemePreset::HighContrast, 115, 125, 10, 130, 99);
        state.flags = 0x0001;

        let mut record = [0u8; THEME_RECORD_LEN];
        let len = encode_theme_state(state, &mut record).unwrap();
        assert_eq!(len, THEME_RECORD_LEN);
        assert_eq!(decode_theme_state(&record).unwrap(), state);
    }

    #[test]
    fn adapter_delegates_read_write_to_io_boundary() {
        let io = MemoryThemeIo::default();
        let mut adapter = ThemeStateIoAdapter::new(io);
        let book_id = BookId8::from_candidate("ALICE001");
        let state = ThemeState::new(book_id, ThemePreset::Light, 100, 100, 8, 120, 1234);

        adapter.write_theme(state).unwrap();
        let restored = adapter.read_theme(book_id).unwrap().unwrap();

        assert_eq!(restored, state);
    }

    #[test]
    fn corrupted_checksum_is_rejected() {
        let book_id = BookId8::from_candidate("ALICE001");
        let state = ThemeState::default_for_book(book_id);
        let mut record = [0u8; THEME_RECORD_LEN];
        encode_theme_state(state, &mut record).unwrap();
        record[18] ^= 0xFF;

        assert_eq!(
            decode_theme_state(&record),
            Err(ThemeStateCodecError::InvalidChecksum)
        );
    }
}
