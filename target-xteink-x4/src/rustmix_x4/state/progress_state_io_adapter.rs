//! Progress State I/O Adapter.
//!
//! This module is deliberately pure and hardware-free. It defines the narrow
//! boundary between Rustmix reader state and whichever storage implementation
//! owns the physical SD/FAT behavior.
//!
//! Owned here:
//! - `state/<BOOKID>.PRG` path convention
//! - fixed progress record encode/decode
//! - progress-state read/write adapter contract
//! - validation marker
//!
//! Not owned here:
//! - SD-card access
//! - SPI arbitration
//! - FAT/filesystem implementation
//! - book discovery
//! - reader pagination

#![allow(dead_code)]

use core::fmt;

/// Validation marker emitted by validation / boot marker plumbing.
pub const PROGRESS_STATE_IO_ADAPTER_MARKER: &str = "x4-progress-state-io-adapter-ok";

pub const PROGRESS_STATE_DIR: &str = "state";
pub const PROGRESS_STATE_EXTENSION: &str = "PRG";
pub const BOOK_ID_8_3_LEN: usize = 8;
pub const PROGRESS_RECORD_MAGIC: [u8; 4] = *b"VPRG";
pub const PROGRESS_RECORD_VERSION: u8 = 1;

/// magic + version + unit + flags + book_id + page_index + logical_offset + updated + checksum.
pub const PROGRESS_RECORD_LEN: usize = 4 + 1 + 1 + 2 + 8 + 4 + 4 + 8 + 4;
pub const PROGRESS_STATE_PATH_MAX_LEN: usize = "state/".len() + BOOK_ID_8_3_LEN + ".PRG".len();

/// Unit used by a persisted reading location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ProgressLocationUnit {
    /// Reader-visible page index. This is usually the safest initial unit for X4.
    PageIndex = 0,
    /// Byte offset within a normalized text/cache stream.
    ByteOffset = 1,
    /// Reader slice/chunk index. Useful when pagination is generated in strips/slices.
    SliceIndex = 2,
}

impl ProgressLocationUnit {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::PageIndex),
            1 => Some(Self::ByteOffset),
            2 => Some(Self::SliceIndex),
            _ => None,
        }
    }
}

/// 8.3-safe per-book ID.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BookId8 {
    bytes: [u8; BOOK_ID_8_3_LEN],
}

impl BookId8 {
    pub const fn from_bytes_unchecked(bytes: [u8; BOOK_ID_8_3_LEN]) -> Self {
        Self { bytes }
    }

    /// Creates an 8.3-safe ID from a candidate string.
    ///
    /// If the candidate is already exactly 8 safe ASCII characters, it is uppercased
    /// and used directly. Otherwise, an uppercase 8-character FNV-1a hex ID is derived.
    pub fn from_candidate(candidate: &str) -> Self {
        let bytes = candidate.as_bytes();
        if bytes.len() == BOOK_ID_8_3_LEN && bytes.iter().all(|b| is_safe_book_id_byte(*b)) {
            let mut out = [0u8; BOOK_ID_8_3_LEN];
            let mut idx = 0;
            while idx < BOOK_ID_8_3_LEN {
                out[idx] = ascii_upper(bytes[idx]);
                idx += 1;
            }
            Self { bytes: out }
        } else {
            Self::from_hash(candidate.as_bytes())
        }
    }

    pub fn from_hash(input: &[u8]) -> Self {
        let hash = fnv1a32(input);
        let mut out = [0u8; BOOK_ID_8_3_LEN];
        write_hex_u32(hash, &mut out);
        Self { bytes: out }
    }

    pub const fn as_bytes(&self) -> &[u8; BOOK_ID_8_3_LEN] {
        &self.bytes
    }
}

impl fmt::Debug for BookId8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BookId8(")?;
        for byte in self.bytes {
            f.write_str(char_from_ascii(byte))?;
        }
        f.write_str(")")
    }
}

/// Progress state persisted per book.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProgressState {
    pub book_id: BookId8,
    pub unit: ProgressLocationUnit,
    pub page_index: u32,
    pub logical_offset: u32,
    pub updated_epoch_seconds: u64,
}

impl ProgressState {
    pub const fn new(
        book_id: BookId8,
        unit: ProgressLocationUnit,
        page_index: u32,
        logical_offset: u32,
        updated_epoch_seconds: u64,
    ) -> Self {
        Self {
            book_id,
            unit,
            page_index,
            logical_offset,
            updated_epoch_seconds,
        }
    }
}

/// Storage-facing byte I/O boundary for progress records.
///
/// A future concrete storage implementation should implement this trait using the
/// existing X4 storage runtime. This trait intentionally knows nothing about SD/FAT/SPI.
pub trait ProgressStateIo {
    type Error;

    /// Reads a progress record from `path` into `out`.
    ///
    /// Return `Ok(None)` when the file does not exist.
    /// Return `Ok(Some(len))` when bytes were read.
    fn read_progress_record(
        &mut self,
        path: &str,
        out: &mut [u8],
    ) -> Result<Option<usize>, Self::Error>;

    /// Writes `bytes` to `path`, replacing any existing record.
    fn write_progress_record(&mut self, path: &str, bytes: &[u8]) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressStateCodecError {
    BufferTooSmall,
    InvalidLength,
    InvalidMagic,
    UnsupportedVersion(u8),
    InvalidUnit(u8),
    InvalidChecksum,
    InvalidBookId,
    PathBufferTooSmall,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ProgressStateAdapterError<E> {
    Io(E),
    Codec(ProgressStateCodecError),
}

impl<E> From<ProgressStateCodecError> for ProgressStateAdapterError<E> {
    fn from(value: ProgressStateCodecError) -> Self {
        Self::Codec(value)
    }
}

/// Thin progress-state adapter over a caller-provided I/O implementation.
pub struct ProgressStateIoAdapter<IO> {
    io: IO,
}

impl<IO> ProgressStateIoAdapter<IO> {
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

impl<IO> ProgressStateIoAdapter<IO>
where
    IO: ProgressStateIo,
{
    pub fn read_progress(
        &mut self,
        book_id: BookId8,
    ) -> Result<Option<ProgressState>, ProgressStateAdapterError<IO::Error>> {
        let mut path = [0u8; PROGRESS_STATE_PATH_MAX_LEN];
        let path_len = write_progress_state_path(book_id, &mut path)?;
        let path = core::str::from_utf8(&path[..path_len])
            .map_err(|_| ProgressStateCodecError::InvalidBookId)?;

        let mut record = [0u8; PROGRESS_RECORD_LEN];
        let Some(len) = self
            .io
            .read_progress_record(path, &mut record)
            .map_err(ProgressStateAdapterError::Io)?
        else {
            return Ok(None);
        };

        if len != PROGRESS_RECORD_LEN {
            return Err(ProgressStateCodecError::InvalidLength.into());
        }

        let state = decode_progress_state(&record)?;
        if state.book_id != book_id {
            return Err(ProgressStateCodecError::InvalidBookId.into());
        }

        Ok(Some(state))
    }

    pub fn write_progress(
        &mut self,
        state: ProgressState,
    ) -> Result<(), ProgressStateAdapterError<IO::Error>> {
        let mut path = [0u8; PROGRESS_STATE_PATH_MAX_LEN];
        let path_len = write_progress_state_path(state.book_id, &mut path)?;
        let path = core::str::from_utf8(&path[..path_len])
            .map_err(|_| ProgressStateCodecError::InvalidBookId)?;

        let mut record = [0u8; PROGRESS_RECORD_LEN];
        encode_progress_state(state, &mut record)?;

        self.io
            .write_progress_record(path, &record)
            .map_err(ProgressStateAdapterError::Io)
    }
}

pub fn progress_state_io_adapter_marker() -> &'static str {
    PROGRESS_STATE_IO_ADAPTER_MARKER
}

pub fn write_progress_state_path(
    book_id: BookId8,
    out: &mut [u8],
) -> Result<usize, ProgressStateCodecError> {
    if out.len() < PROGRESS_STATE_PATH_MAX_LEN {
        return Err(ProgressStateCodecError::PathBufferTooSmall);
    }

    let mut offset = 0;
    copy_bytes(PROGRESS_STATE_DIR.as_bytes(), out, &mut offset);
    out[offset] = b'/';
    offset += 1;
    copy_bytes(book_id.as_bytes(), out, &mut offset);
    out[offset] = b'.';
    offset += 1;
    copy_bytes(PROGRESS_STATE_EXTENSION.as_bytes(), out, &mut offset);

    Ok(offset)
}

pub fn encode_progress_state(
    state: ProgressState,
    out: &mut [u8],
) -> Result<usize, ProgressStateCodecError> {
    if out.len() < PROGRESS_RECORD_LEN {
        return Err(ProgressStateCodecError::BufferTooSmall);
    }

    let record = &mut out[..PROGRESS_RECORD_LEN];
    record.fill(0);

    record[0..4].copy_from_slice(&PROGRESS_RECORD_MAGIC);
    record[4] = PROGRESS_RECORD_VERSION;
    record[5] = state.unit as u8;
    record[6..8].copy_from_slice(&0u16.to_le_bytes());
    record[8..16].copy_from_slice(state.book_id.as_bytes());
    record[16..20].copy_from_slice(&state.page_index.to_le_bytes());
    record[20..24].copy_from_slice(&state.logical_offset.to_le_bytes());
    record[24..32].copy_from_slice(&state.updated_epoch_seconds.to_le_bytes());

    let checksum = additive_checksum(&record[..32]);
    record[32..36].copy_from_slice(&checksum.to_le_bytes());

    Ok(PROGRESS_RECORD_LEN)
}

pub fn decode_progress_state(input: &[u8]) -> Result<ProgressState, ProgressStateCodecError> {
    if input.len() != PROGRESS_RECORD_LEN {
        return Err(ProgressStateCodecError::InvalidLength);
    }

    if input[0..4] != PROGRESS_RECORD_MAGIC[..] {
        return Err(ProgressStateCodecError::InvalidMagic);
    }

    let version = input[4];
    if version != PROGRESS_RECORD_VERSION {
        return Err(ProgressStateCodecError::UnsupportedVersion(version));
    }

    let unit_byte = input[5];
    let unit = ProgressLocationUnit::from_u8(unit_byte)
        .ok_or(ProgressStateCodecError::InvalidUnit(unit_byte))?;

    let mut book_id = [0u8; BOOK_ID_8_3_LEN];
    book_id.copy_from_slice(&input[8..16]);
    if !book_id.iter().all(|b| is_safe_book_id_byte(*b)) {
        return Err(ProgressStateCodecError::InvalidBookId);
    }

    let expected = u32::from_le_bytes([input[32], input[33], input[34], input[35]]);
    let actual = additive_checksum(&input[..32]);
    if expected != actual {
        return Err(ProgressStateCodecError::InvalidChecksum);
    }

    let page_index = u32::from_le_bytes([input[16], input[17], input[18], input[19]]);
    let logical_offset = u32::from_le_bytes([input[20], input[21], input[22], input[23]]);
    let updated_epoch_seconds = u64::from_le_bytes([
        input[24], input[25], input[26], input[27], input[28], input[29], input[30], input[31],
    ]);

    Ok(ProgressState {
        book_id: BookId8::from_bytes_unchecked(book_id),
        unit,
        page_index,
        logical_offset,
        updated_epoch_seconds,
    })
}

fn additive_checksum(bytes: &[u8]) -> u32 {
    let mut value = 0u32;
    for byte in bytes {
        value = value.wrapping_add(*byte as u32);
        value = value.rotate_left(3) ^ 0xA5A5_5A5A;
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

fn ascii_upper(byte: u8) -> u8 {
    if byte.is_ascii_lowercase() {
        byte - 32
    } else {
        byte
    }
}

fn char_from_ascii(byte: u8) -> &'static str {
    match byte {
        b'0' => "0",
        b'1' => "1",
        b'2' => "2",
        b'3' => "3",
        b'4' => "4",
        b'5' => "5",
        b'6' => "6",
        b'7' => "7",
        b'8' => "8",
        b'9' => "9",
        b'A' => "A",
        b'B' => "B",
        b'C' => "C",
        b'D' => "D",
        b'E' => "E",
        b'F' => "F",
        b'G' => "G",
        b'H' => "H",
        b'I' => "I",
        b'J' => "J",
        b'K' => "K",
        b'L' => "L",
        b'M' => "M",
        b'N' => "N",
        b'O' => "O",
        b'P' => "P",
        b'Q' => "Q",
        b'R' => "R",
        b'S' => "S",
        b'T' => "T",
        b'U' => "U",
        b'V' => "V",
        b'W' => "W",
        b'X' => "X",
        b'Y' => "Y",
        b'Z' => "Z",
        b'_' => "_",
        b'-' => "-",
        _ => "?",
    }
}

fn fnv1a32(input: &[u8]) -> u32 {
    let mut hash = 0x811C_9DC5u32;
    for byte in input {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn write_hex_u32(value: u32, out: &mut [u8; BOOK_ID_8_3_LEN]) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut idx = 0;
    while idx < BOOK_ID_8_3_LEN {
        let shift = 28 - (idx * 4);
        out[idx] = HEX[((value >> shift) & 0xF) as usize];
        idx += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemoryProgressIo {
        path: heaplessless::String<32>,
        bytes: [u8; PROGRESS_RECORD_LEN],
        has_record: bool,
    }

    impl ProgressStateIo for MemoryProgressIo {
        type Error = ();

        fn read_progress_record(
            &mut self,
            path: &str,
            out: &mut [u8],
        ) -> Result<Option<usize>, Self::Error> {
            assert_eq!(path, self.path.as_str());
            if !self.has_record {
                return Ok(None);
            }
            out[..PROGRESS_RECORD_LEN].copy_from_slice(&self.bytes);
            Ok(Some(PROGRESS_RECORD_LEN))
        }

        fn write_progress_record(&mut self, path: &str, bytes: &[u8]) -> Result<(), Self::Error> {
            self.path.clear();
            self.path.push_str(path).unwrap();
            self.bytes.copy_from_slice(bytes);
            self.has_record = true;
            Ok(())
        }
    }

    #[test]
    fn book_id_uses_safe_candidate_when_possible() {
        let book_id = BookId8::from_candidate("abc123xy");
        assert_eq!(book_id.as_bytes(), b"ABC123XY");
    }

    #[test]
    fn book_id_hashes_long_candidate() {
        let book_id = BookId8::from_candidate("Alice's Adventures in Wonderland.epub");
        assert_eq!(book_id.as_bytes().len(), 8);
        assert!(book_id.as_bytes().iter().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn path_is_8_3_safe() {
        let mut path = [0u8; PROGRESS_STATE_PATH_MAX_LEN];
        let book_id = BookId8::from_candidate("ALICE001");
        let len = write_progress_state_path(book_id, &mut path).unwrap();
        assert_eq!(
            core::str::from_utf8(&path[..len]).unwrap(),
            "state/ALICE001.PRG"
        );
    }

    #[test]
    fn encode_decode_round_trip() {
        let book_id = BookId8::from_candidate("ALICE001");
        let state = ProgressState::new(
            book_id,
            ProgressLocationUnit::PageIndex,
            42,
            2048,
            1_765_000_000,
        );

        let mut record = [0u8; PROGRESS_RECORD_LEN];
        let len = encode_progress_state(state, &mut record).unwrap();
        assert_eq!(len, PROGRESS_RECORD_LEN);
        assert_eq!(decode_progress_state(&record).unwrap(), state);
    }

    #[test]
    fn adapter_delegates_read_write_to_io_boundary() {
        let io = MemoryProgressIo::default();
        let mut adapter = ProgressStateIoAdapter::new(io);
        let book_id = BookId8::from_candidate("ALICE001");
        let state = ProgressState::new(book_id, ProgressLocationUnit::SliceIndex, 7, 128, 99);

        adapter.write_progress(state).unwrap();
        let restored = adapter.read_progress(book_id).unwrap().unwrap();

        assert_eq!(restored, state);
    }

    #[test]
    fn corrupted_checksum_is_rejected() {
        let book_id = BookId8::from_candidate("ALICE001");
        let state = ProgressState::new(book_id, ProgressLocationUnit::PageIndex, 1, 2, 3);
        let mut record = [0u8; PROGRESS_RECORD_LEN];
        encode_progress_state(state, &mut record).unwrap();
        record[16] ^= 0xFF;

        assert_eq!(
            decode_progress_state(&record),
            Err(ProgressStateCodecError::InvalidChecksum)
        );
    }
}

#[cfg(test)]
mod heaplessless {
    #[derive(Clone)]
    pub struct String<const N: usize> {
        buf: [u8; N],
        len: usize,
    }

    impl<const N: usize> Default for String<N> {
        fn default() -> Self {
            Self {
                buf: [0u8; N],
                len: 0,
            }
        }
    }

    impl<const N: usize> String<N> {
        pub fn clear(&mut self) {
            self.len = 0;
        }

        pub fn push_str(&mut self, value: &str) -> Result<(), ()> {
            let bytes = value.as_bytes();
            if bytes.len() > N {
                return Err(());
            }
            self.buf[..bytes.len()].copy_from_slice(bytes);
            self.len = bytes.len();
            Ok(())
        }

        pub fn as_str(&self) -> &str {
            core::str::from_utf8(&self.buf[..self.len]).unwrap()
        }
    }
}
