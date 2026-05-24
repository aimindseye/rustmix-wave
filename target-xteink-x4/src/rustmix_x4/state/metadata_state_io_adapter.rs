//! Metadata State I/O Adapter.
//!
//! This module is deliberately pure and hardware-free. It defines the narrow
//! boundary between Rustmix per-book metadata state and whichever storage
//! implementation owns the physical SD/FAT behavior.
//!
//! Owned here:
//! - `state/<BOOKID>.MTA` path convention
//! - fixed metadata record encode/decode
//! - display-name cache field for future file-browser/reader title use
//! - metadata-state read/write adapter contract
//! - validation marker
//!
//! Not owned here:
//! - SD-card access
//! - SPI arbitration
//! - FAT/filesystem implementation
//! - file discovery
//! - live library UI behavior
//! - reader rendering

#![allow(dead_code)]

use super::progress_state_io_adapter::{BOOK_ID_8_3_LEN, BookId8};

/// Validation marker emitted by validation / boot marker plumbing.
pub const METADATA_STATE_IO_ADAPTER_MARKER: &str = "x4-metadata-state-io-adapter-ok";

pub const METADATA_STATE_DIR: &str = "state";
pub const METADATA_STATE_EXTENSION: &str = "MTA";
pub const METADATA_RECORD_MAGIC: [u8; 4] = *b"VMTA";
pub const METADATA_RECORD_VERSION: u8 = 1;
pub const METADATA_DISPLAY_NAME_MAX_LEN: usize = 64;

/// magic + version + kind + flags + book_id + file_size + content_fingerprint
/// + display_name_len + reserved + display_name + updated + checksum.
pub const METADATA_RECORD_LEN: usize =
    4 + 1 + 1 + 2 + 8 + 8 + 4 + 1 + 3 + METADATA_DISPLAY_NAME_MAX_LEN + 8 + 4;
pub const METADATA_STATE_PATH_MAX_LEN: usize = "state/".len() + BOOK_ID_8_3_LEN + ".MTA".len();

/// Persisted reader file kind. This is metadata only; it does not select live
/// parser behavior in this implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MetadataFileKind {
    Unknown = 0,
    Text = 1,
    Epub = 2,
    Xtc = 3,
    Rustmix = 4,
}

impl MetadataFileKind {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Unknown),
            1 => Some(Self::Text),
            2 => Some(Self::Epub),
            3 => Some(Self::Xtc),
            4 => Some(Self::Rustmix),
            _ => None,
        }
    }
}

/// Small fixed per-book metadata cache.
///
/// `display_name` is a UTF-8 prefix, not an owned String, so the type remains
/// usable in the no_std X4 target without allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetadataState {
    pub book_id: BookId8,
    pub file_kind: MetadataFileKind,
    pub flags: u16,
    pub file_size_bytes: u64,
    pub content_fingerprint: u32,
    pub display_name_len: u8,
    pub display_name: [u8; METADATA_DISPLAY_NAME_MAX_LEN],
    pub updated_epoch_seconds: u64,
}

impl MetadataState {
    pub fn new(
        book_id: BookId8,
        file_kind: MetadataFileKind,
        file_size_bytes: u64,
        content_fingerprint: u32,
        display_name: &str,
        updated_epoch_seconds: u64,
    ) -> Self {
        let mut state = Self::empty(
            book_id,
            file_kind,
            file_size_bytes,
            content_fingerprint,
            updated_epoch_seconds,
        );
        state.set_display_name_truncated(display_name);
        state
    }

    pub const fn empty(
        book_id: BookId8,
        file_kind: MetadataFileKind,
        file_size_bytes: u64,
        content_fingerprint: u32,
        updated_epoch_seconds: u64,
    ) -> Self {
        Self {
            book_id,
            file_kind,
            flags: 0,
            file_size_bytes,
            content_fingerprint,
            display_name_len: 0,
            display_name: [0u8; METADATA_DISPLAY_NAME_MAX_LEN],
            updated_epoch_seconds,
        }
    }

    pub fn set_display_name_truncated(&mut self, value: &str) {
        self.display_name = [0u8; METADATA_DISPLAY_NAME_MAX_LEN];

        let bytes = value.as_bytes();
        let mut len = if bytes.len() > METADATA_DISPLAY_NAME_MAX_LEN {
            METADATA_DISPLAY_NAME_MAX_LEN
        } else {
            bytes.len()
        };

        while !value.is_char_boundary(len) {
            len -= 1;
        }

        self.display_name[..len].copy_from_slice(&bytes[..len]);
        self.display_name_len = len as u8;
    }

    pub fn display_name_str(&self) -> Result<&str, MetadataStateCodecError> {
        let len = self.display_name_len as usize;
        if len > METADATA_DISPLAY_NAME_MAX_LEN {
            return Err(MetadataStateCodecError::InvalidDisplayName);
        }
        core::str::from_utf8(&self.display_name[..len])
            .map_err(|_| MetadataStateCodecError::InvalidDisplayName)
    }
}

/// Storage-facing byte I/O boundary for metadata records.
///
/// A future concrete storage implementation should implement this trait using
/// the existing X4 storage runtime. This trait intentionally knows nothing
/// about SD/FAT/SPI.
pub trait MetadataStateIo {
    type Error;

    /// Reads a metadata record from `path` into `out`.
    ///
    /// Return `Ok(None)` when the file does not exist.
    /// Return `Ok(Some(len))` when bytes were read.
    fn read_metadata_record(
        &mut self,
        path: &str,
        out: &mut [u8],
    ) -> Result<Option<usize>, Self::Error>;

    /// Writes `bytes` to `path`, replacing any existing record.
    fn write_metadata_record(&mut self, path: &str, bytes: &[u8]) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataStateCodecError {
    BufferTooSmall,
    InvalidLength,
    InvalidMagic,
    UnsupportedVersion(u8),
    InvalidFileKind(u8),
    InvalidChecksum,
    InvalidBookId,
    InvalidDisplayName,
    PathBufferTooSmall,
}

#[derive(Debug, Eq, PartialEq)]
pub enum MetadataStateAdapterError<E> {
    Io(E),
    Codec(MetadataStateCodecError),
}

impl<E> From<MetadataStateCodecError> for MetadataStateAdapterError<E> {
    fn from(value: MetadataStateCodecError) -> Self {
        Self::Codec(value)
    }
}

/// Thin metadata-state adapter over a caller-provided I/O implementation.
pub struct MetadataStateIoAdapter<IO> {
    io: IO,
}

impl<IO> MetadataStateIoAdapter<IO> {
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

impl<IO> MetadataStateIoAdapter<IO>
where
    IO: MetadataStateIo,
{
    pub fn read_metadata(
        &mut self,
        book_id: BookId8,
    ) -> Result<Option<MetadataState>, MetadataStateAdapterError<IO::Error>> {
        let mut path = [0u8; METADATA_STATE_PATH_MAX_LEN];
        let path_len = write_metadata_state_path(book_id, &mut path)?;
        let path = core::str::from_utf8(&path[..path_len])
            .map_err(|_| MetadataStateCodecError::InvalidBookId)?;

        let mut record = [0u8; METADATA_RECORD_LEN];
        let Some(len) = self
            .io
            .read_metadata_record(path, &mut record)
            .map_err(MetadataStateAdapterError::Io)?
        else {
            return Ok(None);
        };

        if len != METADATA_RECORD_LEN {
            return Err(MetadataStateCodecError::InvalidLength.into());
        }

        let state = decode_metadata_state(&record)?;
        if state.book_id != book_id {
            return Err(MetadataStateCodecError::InvalidBookId.into());
        }

        Ok(Some(state))
    }

    pub fn write_metadata(
        &mut self,
        state: MetadataState,
    ) -> Result<(), MetadataStateAdapterError<IO::Error>> {
        let mut path = [0u8; METADATA_STATE_PATH_MAX_LEN];
        let path_len = write_metadata_state_path(state.book_id, &mut path)?;
        let path = core::str::from_utf8(&path[..path_len])
            .map_err(|_| MetadataStateCodecError::InvalidBookId)?;

        let mut record = [0u8; METADATA_RECORD_LEN];
        encode_metadata_state(state, &mut record)?;

        self.io
            .write_metadata_record(path, &record)
            .map_err(MetadataStateAdapterError::Io)
    }
}

pub fn metadata_state_io_adapter_marker() -> &'static str {
    METADATA_STATE_IO_ADAPTER_MARKER
}

pub fn write_metadata_state_path(
    book_id: BookId8,
    out: &mut [u8],
) -> Result<usize, MetadataStateCodecError> {
    if out.len() < METADATA_STATE_PATH_MAX_LEN {
        return Err(MetadataStateCodecError::PathBufferTooSmall);
    }

    let mut offset = 0;
    copy_bytes(METADATA_STATE_DIR.as_bytes(), out, &mut offset);
    out[offset] = b'/';
    offset += 1;
    copy_bytes(book_id.as_bytes(), out, &mut offset);
    out[offset] = b'.';
    offset += 1;
    copy_bytes(METADATA_STATE_EXTENSION.as_bytes(), out, &mut offset);

    Ok(offset)
}

pub fn encode_metadata_state(
    state: MetadataState,
    out: &mut [u8],
) -> Result<usize, MetadataStateCodecError> {
    if out.len() < METADATA_RECORD_LEN {
        return Err(MetadataStateCodecError::BufferTooSmall);
    }

    let display_len = state.display_name_len as usize;
    if display_len > METADATA_DISPLAY_NAME_MAX_LEN {
        return Err(MetadataStateCodecError::InvalidDisplayName);
    }
    if core::str::from_utf8(&state.display_name[..display_len]).is_err() {
        return Err(MetadataStateCodecError::InvalidDisplayName);
    }

    let record = &mut out[..METADATA_RECORD_LEN];
    record.fill(0);

    record[0..4].copy_from_slice(&METADATA_RECORD_MAGIC);
    record[4] = METADATA_RECORD_VERSION;
    record[5] = state.file_kind as u8;
    record[6..8].copy_from_slice(&state.flags.to_le_bytes());
    record[8..16].copy_from_slice(state.book_id.as_bytes());
    record[16..24].copy_from_slice(&state.file_size_bytes.to_le_bytes());
    record[24..28].copy_from_slice(&state.content_fingerprint.to_le_bytes());
    record[28] = state.display_name_len;
    record[29..32].copy_from_slice(&[0u8; 3]);
    record[32..96].copy_from_slice(&state.display_name);
    record[96..104].copy_from_slice(&state.updated_epoch_seconds.to_le_bytes());

    let checksum = additive_checksum(&record[..104]);
    record[104..108].copy_from_slice(&checksum.to_le_bytes());

    Ok(METADATA_RECORD_LEN)
}

pub fn decode_metadata_state(input: &[u8]) -> Result<MetadataState, MetadataStateCodecError> {
    if input.len() != METADATA_RECORD_LEN {
        return Err(MetadataStateCodecError::InvalidLength);
    }

    if input[0..4] != METADATA_RECORD_MAGIC[..] {
        return Err(MetadataStateCodecError::InvalidMagic);
    }

    let version = input[4];
    if version != METADATA_RECORD_VERSION {
        return Err(MetadataStateCodecError::UnsupportedVersion(version));
    }

    let kind_byte = input[5];
    let file_kind = MetadataFileKind::from_u8(kind_byte)
        .ok_or(MetadataStateCodecError::InvalidFileKind(kind_byte))?;

    let mut book_id = [0u8; BOOK_ID_8_3_LEN];
    book_id.copy_from_slice(&input[8..16]);
    if !book_id.iter().all(|b| is_safe_book_id_byte(*b)) {
        return Err(MetadataStateCodecError::InvalidBookId);
    }

    let display_name_len = input[28];
    let display_len = display_name_len as usize;
    if display_len > METADATA_DISPLAY_NAME_MAX_LEN {
        return Err(MetadataStateCodecError::InvalidDisplayName);
    }

    let mut display_name = [0u8; METADATA_DISPLAY_NAME_MAX_LEN];
    display_name.copy_from_slice(&input[32..96]);
    if core::str::from_utf8(&display_name[..display_len]).is_err() {
        return Err(MetadataStateCodecError::InvalidDisplayName);
    }

    let expected = u32::from_le_bytes([input[104], input[105], input[106], input[107]]);
    let actual = additive_checksum(&input[..104]);
    if expected != actual {
        return Err(MetadataStateCodecError::InvalidChecksum);
    }

    let flags = u16::from_le_bytes([input[6], input[7]]);
    let file_size_bytes = u64::from_le_bytes([
        input[16], input[17], input[18], input[19], input[20], input[21], input[22], input[23],
    ]);
    let content_fingerprint = u32::from_le_bytes([input[24], input[25], input[26], input[27]]);
    let updated_epoch_seconds = u64::from_le_bytes([
        input[96], input[97], input[98], input[99], input[100], input[101], input[102], input[103],
    ]);

    Ok(MetadataState {
        book_id: BookId8::from_bytes_unchecked(book_id),
        file_kind,
        flags,
        file_size_bytes,
        content_fingerprint,
        display_name_len,
        display_name,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MemoryMetadataIo {
        path: heaplessless::String<40>,
        bytes: [u8; METADATA_RECORD_LEN],
        has_record: bool,
    }

    impl Default for MemoryMetadataIo {
        fn default() -> Self {
            Self {
                path: heaplessless::String::default(),
                bytes: [0u8; METADATA_RECORD_LEN],
                has_record: false,
            }
        }
    }

    impl MetadataStateIo for MemoryMetadataIo {
        type Error = ();

        fn read_metadata_record(
            &mut self,
            path: &str,
            out: &mut [u8],
        ) -> Result<Option<usize>, Self::Error> {
            assert_eq!(path, self.path.as_str());
            if !self.has_record {
                return Ok(None);
            }
            out[..METADATA_RECORD_LEN].copy_from_slice(&self.bytes);
            Ok(Some(METADATA_RECORD_LEN))
        }

        fn write_metadata_record(&mut self, path: &str, bytes: &[u8]) -> Result<(), Self::Error> {
            self.path.clear();
            self.path.push_str(path).unwrap();
            self.bytes.copy_from_slice(bytes);
            self.has_record = true;
            Ok(())
        }
    }

    #[test]
    fn path_is_8_3_safe() {
        let mut path = [0u8; METADATA_STATE_PATH_MAX_LEN];
        let book_id = BookId8::from_candidate("ALICE001");
        let len = write_metadata_state_path(book_id, &mut path).unwrap();
        assert_eq!(
            core::str::from_utf8(&path[..len]).unwrap(),
            "state/ALICE001.MTA"
        );
    }

    #[test]
    fn display_name_round_trips() {
        let book_id = BookId8::from_candidate("ALICE001");
        let state = MetadataState::new(
            book_id,
            MetadataFileKind::Epub,
            123_456,
            0xAABB_CCDD,
            "Alice's Adventures in Wonderland",
            1_765_000_000,
        );

        let mut record = [0u8; METADATA_RECORD_LEN];
        let len = encode_metadata_state(state, &mut record).unwrap();
        assert_eq!(len, METADATA_RECORD_LEN);

        let restored = decode_metadata_state(&record).unwrap();
        assert_eq!(restored, state);
        assert_eq!(
            restored.display_name_str().unwrap(),
            "Alice's Adventures in Wonderland"
        );
    }

    #[test]
    fn long_utf8_display_name_truncates_on_char_boundary() {
        let book_id = BookId8::from_candidate("ALICE001");
        let mut state = MetadataState::empty(book_id, MetadataFileKind::Epub, 1, 2, 3);
        state.set_display_name_truncated(
            "Alice's Adventures in Wonderland — Through the Looking-Glass — illustrated edition",
        );

        assert!((state.display_name_len as usize) <= METADATA_DISPLAY_NAME_MAX_LEN);
        assert!(state.display_name_str().is_ok());
    }

    #[test]
    fn adapter_delegates_read_write_to_io_boundary() {
        let io = MemoryMetadataIo::default();
        let mut adapter = MetadataStateIoAdapter::new(io);
        let book_id = BookId8::from_candidate("ALICE001");
        let state = MetadataState::new(
            book_id,
            MetadataFileKind::Epub,
            99,
            0x0102_0304,
            "Alice's Adventures in Wonderland",
            123,
        );

        adapter.write_metadata(state).unwrap();
        let restored = adapter.read_metadata(book_id).unwrap().unwrap();

        assert_eq!(restored, state);
    }

    #[test]
    fn corrupted_checksum_is_rejected() {
        let book_id = BookId8::from_candidate("ALICE001");
        let state = MetadataState::new(book_id, MetadataFileKind::Text, 1, 2, "Alice", 3);
        let mut record = [0u8; METADATA_RECORD_LEN];
        encode_metadata_state(state, &mut record).unwrap();
        record[16] ^= 0xFF;

        assert_eq!(
            decode_metadata_state(&record),
            Err(MetadataStateCodecError::InvalidChecksum)
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
