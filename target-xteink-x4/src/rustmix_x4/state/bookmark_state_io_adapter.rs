//! Bookmark State I/O Adapter.
//!
//! This module is deliberately pure and hardware-free. It defines the narrow
//! boundary between Rustmix bookmark state and whichever storage implementation
//! owns the physical SD/FAT behavior.
//!
//! Owned here:
//! - `state/<BOOKID>.BKM` path convention
//! - `state/BMIDX.TXT` index path convention
//! - fixed bookmark file encode/decode
//! - bookmark-state read/write adapter contract
//! - validation marker
//!
//! Not owned here:
//! - SD-card access
//! - SPI arbitration
//! - FAT/filesystem implementation
//! - file discovery
//! - live bookmark UI behavior
//! - reader rendering
//! - highlight/note semantics beyond a reserved entry kind

#![allow(dead_code)]

use super::progress_state_io_adapter::{BOOK_ID_8_3_LEN, BookId8};

/// Validation marker emitted by validation / boot marker plumbing.
pub const BOOKMARK_STATE_IO_ADAPTER_MARKER: &str = "x4-bookmark-state-io-adapter-ok";

pub const BOOKMARK_STATE_DIR: &str = "state";
pub const BOOKMARK_STATE_EXTENSION: &str = "BKM";
pub const BOOKMARK_INDEX_FILE_NAME: &str = "BMIDX.TXT";
pub const BOOKMARK_RECORD_MAGIC: [u8; 4] = *b"VBKM";
pub const BOOKMARK_RECORD_VERSION: u8 = 1;
pub const BOOKMARK_MAX_ENTRIES: usize = 32;
pub const BOOKMARK_LABEL_MAX_LEN: usize = 32;

/// magic + version + flags + count + book_id + updated + checksum.
pub const BOOKMARK_HEADER_LEN: usize = 4 + 1 + 1 + 2 + 8 + 8 + 4;
/// slot + kind + unit + flags + page_index + logical_offset + created + label_len + reserved + label.
pub const BOOKMARK_ENTRY_RECORD_LEN: usize =
    1 + 1 + 1 + 1 + 4 + 4 + 8 + 1 + 3 + BOOKMARK_LABEL_MAX_LEN;
pub const BOOKMARK_FILE_LEN: usize =
    BOOKMARK_HEADER_LEN + (BOOKMARK_MAX_ENTRIES * BOOKMARK_ENTRY_RECORD_LEN);
pub const BOOKMARK_STATE_PATH_MAX_LEN: usize = "state/".len() + BOOK_ID_8_3_LEN + ".BKM".len();
pub const BOOKMARK_INDEX_PATH_LEN: usize = "state/".len() + "BMIDX.TXT".len();

/// Persisted bookmark entry kind.
///
/// Only `Bookmark` is meant for live reader behavior in this implementation. The other
/// values reserve space so the on-disk shape can evolve without changing the
/// file size when highlights/notes are moved later.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BookmarkEntryKind {
    Empty = 0,
    Bookmark = 1,
    HighlightAnchor = 2,
    NoteAnchor = 3,
}

impl BookmarkEntryKind {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Empty),
            1 => Some(Self::Bookmark),
            2 => Some(Self::HighlightAnchor),
            3 => Some(Self::NoteAnchor),
            _ => None,
        }
    }
}

/// Unit used by a persisted bookmark location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BookmarkLocationUnit {
    PageIndex = 0,
    ByteOffset = 1,
    SliceIndex = 2,
}

impl BookmarkLocationUnit {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::PageIndex),
            1 => Some(Self::ByteOffset),
            2 => Some(Self::SliceIndex),
            _ => None,
        }
    }
}

/// Fixed bookmark entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookmarkEntry {
    pub slot: u8,
    pub kind: BookmarkEntryKind,
    pub unit: BookmarkLocationUnit,
    pub flags: u8,
    pub page_index: u32,
    pub logical_offset: u32,
    pub created_epoch_seconds: u64,
    pub label_len: u8,
    pub label: [u8; BOOKMARK_LABEL_MAX_LEN],
}

impl BookmarkEntry {
    pub const fn empty() -> Self {
        Self {
            slot: 0,
            kind: BookmarkEntryKind::Empty,
            unit: BookmarkLocationUnit::PageIndex,
            flags: 0,
            page_index: 0,
            logical_offset: 0,
            created_epoch_seconds: 0,
            label_len: 0,
            label: [0u8; BOOKMARK_LABEL_MAX_LEN],
        }
    }

    pub fn new(
        slot: u8,
        unit: BookmarkLocationUnit,
        page_index: u32,
        logical_offset: u32,
        label: &str,
        created_epoch_seconds: u64,
    ) -> Self {
        let mut entry = Self {
            slot,
            kind: BookmarkEntryKind::Bookmark,
            unit,
            flags: 0,
            page_index,
            logical_offset,
            created_epoch_seconds,
            label_len: 0,
            label: [0u8; BOOKMARK_LABEL_MAX_LEN],
        };
        entry.set_label_truncated(label);
        entry
    }

    pub fn set_label_truncated(&mut self, value: &str) {
        self.label = [0u8; BOOKMARK_LABEL_MAX_LEN];

        let bytes = value.as_bytes();
        let mut len = if bytes.len() > BOOKMARK_LABEL_MAX_LEN {
            BOOKMARK_LABEL_MAX_LEN
        } else {
            bytes.len()
        };

        while !value.is_char_boundary(len) {
            len -= 1;
        }

        self.label[..len].copy_from_slice(&bytes[..len]);
        self.label_len = len as u8;
    }

    pub fn label_str(&self) -> Result<&str, BookmarkStateCodecError> {
        let len = self.label_len as usize;
        if len > BOOKMARK_LABEL_MAX_LEN {
            return Err(BookmarkStateCodecError::InvalidLabel);
        }
        core::str::from_utf8(&self.label[..len]).map_err(|_| BookmarkStateCodecError::InvalidLabel)
    }
}

/// Fixed-size bookmark state for one book.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookmarkState {
    pub book_id: BookId8,
    pub flags: u8,
    pub count: u16,
    pub entries: [BookmarkEntry; BOOKMARK_MAX_ENTRIES],
    pub updated_epoch_seconds: u64,
}

impl BookmarkState {
    pub const fn empty(book_id: BookId8, updated_epoch_seconds: u64) -> Self {
        Self {
            book_id,
            flags: 0,
            count: 0,
            entries: [BookmarkEntry::empty(); BOOKMARK_MAX_ENTRIES],
            updated_epoch_seconds,
        }
    }

    pub fn push(&mut self, entry: BookmarkEntry) -> Result<(), BookmarkStateCodecError> {
        let idx = self.count as usize;
        if idx >= BOOKMARK_MAX_ENTRIES {
            return Err(BookmarkStateCodecError::TooManyEntries);
        }
        if entry.kind == BookmarkEntryKind::Empty {
            return Err(BookmarkStateCodecError::InvalidEntryKind(0));
        }
        self.entries[idx] = entry;
        self.count += 1;
        Ok(())
    }

    pub fn entry_count(&self) -> usize {
        self.count as usize
    }
}

/// Storage-facing byte I/O boundary for bookmark records and the bookmark index.
///
/// A future concrete storage implementation should implement this trait using
/// the existing X4 storage runtime. This trait intentionally knows nothing
/// about SD/FAT/SPI.
pub trait BookmarkStateIo {
    type Error;

    /// Reads a bookmark record from `path` into `out`.
    ///
    /// Return `Ok(None)` when the file does not exist.
    /// Return `Ok(Some(len))` when bytes were read.
    fn read_bookmark_record(
        &mut self,
        path: &str,
        out: &mut [u8],
    ) -> Result<Option<usize>, Self::Error>;

    /// Writes `bytes` to `path`, replacing any existing bookmark record.
    fn write_bookmark_record(&mut self, path: &str, bytes: &[u8]) -> Result<(), Self::Error>;

    /// Reads the bookmark index file from `path` into `out`.
    fn read_bookmark_index(
        &mut self,
        path: &str,
        out: &mut [u8],
    ) -> Result<Option<usize>, Self::Error>;

    /// Writes raw bookmark index bytes to `path`.
    fn write_bookmark_index(&mut self, path: &str, bytes: &[u8]) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookmarkStateCodecError {
    BufferTooSmall,
    InvalidLength,
    InvalidMagic,
    UnsupportedVersion(u8),
    InvalidEntryKind(u8),
    InvalidLocationUnit(u8),
    InvalidChecksum,
    InvalidBookId,
    InvalidCount,
    InvalidLabel,
    PathBufferTooSmall,
    IndexBufferTooSmall,
    TooManyEntries,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BookmarkStateAdapterError<E> {
    Io(E),
    Codec(BookmarkStateCodecError),
}

impl<E> From<BookmarkStateCodecError> for BookmarkStateAdapterError<E> {
    fn from(value: BookmarkStateCodecError) -> Self {
        Self::Codec(value)
    }
}

/// Thin bookmark-state adapter over a caller-provided I/O implementation.
pub struct BookmarkStateIoAdapter<IO> {
    io: IO,
}

impl<IO> BookmarkStateIoAdapter<IO> {
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

impl<IO> BookmarkStateIoAdapter<IO>
where
    IO: BookmarkStateIo,
{
    pub fn read_bookmarks(
        &mut self,
        book_id: BookId8,
    ) -> Result<Option<BookmarkState>, BookmarkStateAdapterError<IO::Error>> {
        let mut path = [0u8; BOOKMARK_STATE_PATH_MAX_LEN];
        let path_len = write_bookmark_state_path(book_id, &mut path)?;
        let path = core::str::from_utf8(&path[..path_len])
            .map_err(|_| BookmarkStateCodecError::InvalidBookId)?;

        let mut record = [0u8; BOOKMARK_FILE_LEN];
        let Some(len) = self
            .io
            .read_bookmark_record(path, &mut record)
            .map_err(BookmarkStateAdapterError::Io)?
        else {
            return Ok(None);
        };

        if len != BOOKMARK_FILE_LEN {
            return Err(BookmarkStateCodecError::InvalidLength.into());
        }

        let state = decode_bookmark_state(&record)?;
        if state.book_id != book_id {
            return Err(BookmarkStateCodecError::InvalidBookId.into());
        }

        Ok(Some(state))
    }

    pub fn write_bookmarks(
        &mut self,
        state: BookmarkState,
    ) -> Result<(), BookmarkStateAdapterError<IO::Error>> {
        let mut path = [0u8; BOOKMARK_STATE_PATH_MAX_LEN];
        let path_len = write_bookmark_state_path(state.book_id, &mut path)?;
        let path = core::str::from_utf8(&path[..path_len])
            .map_err(|_| BookmarkStateCodecError::InvalidBookId)?;

        let mut record = [0u8; BOOKMARK_FILE_LEN];
        encode_bookmark_state(state, &mut record)?;

        self.io
            .write_bookmark_record(path, &record)
            .map_err(BookmarkStateAdapterError::Io)
    }

    pub fn read_bookmark_index_raw(
        &mut self,
        out: &mut [u8],
    ) -> Result<Option<usize>, BookmarkStateAdapterError<IO::Error>> {
        let mut path = [0u8; BOOKMARK_INDEX_PATH_LEN];
        let path_len = write_bookmark_index_path(&mut path)?;
        let path = core::str::from_utf8(&path[..path_len])
            .map_err(|_| BookmarkStateCodecError::InvalidBookId)?;

        self.io
            .read_bookmark_index(path, out)
            .map_err(BookmarkStateAdapterError::Io)
    }

    pub fn write_bookmark_index_raw(
        &mut self,
        bytes: &[u8],
    ) -> Result<(), BookmarkStateAdapterError<IO::Error>> {
        let mut path = [0u8; BOOKMARK_INDEX_PATH_LEN];
        let path_len = write_bookmark_index_path(&mut path)?;
        let path = core::str::from_utf8(&path[..path_len])
            .map_err(|_| BookmarkStateCodecError::InvalidBookId)?;

        self.io
            .write_bookmark_index(path, bytes)
            .map_err(BookmarkStateAdapterError::Io)
    }
}

pub fn bookmark_state_io_adapter_marker() -> &'static str {
    BOOKMARK_STATE_IO_ADAPTER_MARKER
}

pub fn write_bookmark_state_path(
    book_id: BookId8,
    out: &mut [u8],
) -> Result<usize, BookmarkStateCodecError> {
    if out.len() < BOOKMARK_STATE_PATH_MAX_LEN {
        return Err(BookmarkStateCodecError::PathBufferTooSmall);
    }

    let mut offset = 0;
    copy_bytes(BOOKMARK_STATE_DIR.as_bytes(), out, &mut offset);
    out[offset] = b'/';
    offset += 1;
    copy_bytes(book_id.as_bytes(), out, &mut offset);
    out[offset] = b'.';
    offset += 1;
    copy_bytes(BOOKMARK_STATE_EXTENSION.as_bytes(), out, &mut offset);

    Ok(offset)
}

pub fn write_bookmark_index_path(out: &mut [u8]) -> Result<usize, BookmarkStateCodecError> {
    if out.len() < BOOKMARK_INDEX_PATH_LEN {
        return Err(BookmarkStateCodecError::IndexBufferTooSmall);
    }

    let mut offset = 0;
    copy_bytes(BOOKMARK_STATE_DIR.as_bytes(), out, &mut offset);
    out[offset] = b'/';
    offset += 1;
    copy_bytes(BOOKMARK_INDEX_FILE_NAME.as_bytes(), out, &mut offset);
    Ok(offset)
}

pub fn encode_bookmark_state(
    state: BookmarkState,
    out: &mut [u8],
) -> Result<usize, BookmarkStateCodecError> {
    if out.len() < BOOKMARK_FILE_LEN {
        return Err(BookmarkStateCodecError::BufferTooSmall);
    }

    let count = state.count as usize;
    if count > BOOKMARK_MAX_ENTRIES {
        return Err(BookmarkStateCodecError::InvalidCount);
    }

    for entry in &state.entries[..count] {
        validate_bookmark_entry(*entry)?;
    }

    let record = &mut out[..BOOKMARK_FILE_LEN];
    record.fill(0);

    record[0..4].copy_from_slice(&BOOKMARK_RECORD_MAGIC);
    record[4] = BOOKMARK_RECORD_VERSION;
    record[5] = state.flags;
    record[6..8].copy_from_slice(&state.count.to_le_bytes());
    record[8..16].copy_from_slice(state.book_id.as_bytes());
    record[16..24].copy_from_slice(&state.updated_epoch_seconds.to_le_bytes());

    let mut offset = BOOKMARK_HEADER_LEN;
    for entry in &state.entries {
        encode_bookmark_entry(
            *entry,
            &mut record[offset..offset + BOOKMARK_ENTRY_RECORD_LEN],
        )?;
        offset += BOOKMARK_ENTRY_RECORD_LEN;
    }

    let checksum = additive_checksum(&record[..BOOKMARK_FILE_LEN - 4]);
    record[24..28].copy_from_slice(&checksum.to_le_bytes());

    Ok(BOOKMARK_FILE_LEN)
}

pub fn decode_bookmark_state(input: &[u8]) -> Result<BookmarkState, BookmarkStateCodecError> {
    if input.len() != BOOKMARK_FILE_LEN {
        return Err(BookmarkStateCodecError::InvalidLength);
    }

    if input[0..4] != BOOKMARK_RECORD_MAGIC[..] {
        return Err(BookmarkStateCodecError::InvalidMagic);
    }

    let version = input[4];
    if version != BOOKMARK_RECORD_VERSION {
        return Err(BookmarkStateCodecError::UnsupportedVersion(version));
    }

    let expected = u32::from_le_bytes([input[24], input[25], input[26], input[27]]);
    let actual = additive_checksum(&input[..BOOKMARK_FILE_LEN - 4]);
    if expected != actual {
        return Err(BookmarkStateCodecError::InvalidChecksum);
    }

    let count = u16::from_le_bytes([input[6], input[7]]);
    if count as usize > BOOKMARK_MAX_ENTRIES {
        return Err(BookmarkStateCodecError::InvalidCount);
    }

    let mut book_id = [0u8; BOOK_ID_8_3_LEN];
    book_id.copy_from_slice(&input[8..16]);
    if !book_id.iter().all(|b| is_safe_book_id_byte(*b)) {
        return Err(BookmarkStateCodecError::InvalidBookId);
    }

    let mut entries = [BookmarkEntry::empty(); BOOKMARK_MAX_ENTRIES];
    let mut offset = BOOKMARK_HEADER_LEN;
    let mut idx = 0;
    while idx < BOOKMARK_MAX_ENTRIES {
        entries[idx] = decode_bookmark_entry(&input[offset..offset + BOOKMARK_ENTRY_RECORD_LEN])?;
        offset += BOOKMARK_ENTRY_RECORD_LEN;
        idx += 1;
    }

    let active_count = count as usize;
    for entry in &entries[..active_count] {
        if entry.kind == BookmarkEntryKind::Empty {
            return Err(BookmarkStateCodecError::InvalidEntryKind(0));
        }
        validate_bookmark_entry(*entry)?;
    }
    for entry in &entries[active_count..] {
        if entry.kind != BookmarkEntryKind::Empty {
            return Err(BookmarkStateCodecError::InvalidCount);
        }
    }

    let updated_epoch_seconds = u64::from_le_bytes([
        input[16], input[17], input[18], input[19], input[20], input[21], input[22], input[23],
    ]);

    Ok(BookmarkState {
        book_id: BookId8::from_bytes_unchecked(book_id),
        flags: input[5],
        count,
        entries,
        updated_epoch_seconds,
    })
}

fn validate_bookmark_entry(entry: BookmarkEntry) -> Result<(), BookmarkStateCodecError> {
    let label_len = entry.label_len as usize;
    if label_len > BOOKMARK_LABEL_MAX_LEN {
        return Err(BookmarkStateCodecError::InvalidLabel);
    }
    if core::str::from_utf8(&entry.label[..label_len]).is_err() {
        return Err(BookmarkStateCodecError::InvalidLabel);
    }
    if BookmarkEntryKind::from_u8(entry.kind as u8).is_none() {
        return Err(BookmarkStateCodecError::InvalidEntryKind(entry.kind as u8));
    }
    if BookmarkLocationUnit::from_u8(entry.unit as u8).is_none() {
        return Err(BookmarkStateCodecError::InvalidLocationUnit(
            entry.unit as u8,
        ));
    }
    Ok(())
}

fn encode_bookmark_entry(
    entry: BookmarkEntry,
    out: &mut [u8],
) -> Result<(), BookmarkStateCodecError> {
    if out.len() != BOOKMARK_ENTRY_RECORD_LEN {
        return Err(BookmarkStateCodecError::InvalidLength);
    }
    validate_bookmark_entry(entry)?;

    out.fill(0);
    out[0] = entry.slot;
    out[1] = entry.kind as u8;
    out[2] = entry.unit as u8;
    out[3] = entry.flags;
    out[4..8].copy_from_slice(&entry.page_index.to_le_bytes());
    out[8..12].copy_from_slice(&entry.logical_offset.to_le_bytes());
    out[12..20].copy_from_slice(&entry.created_epoch_seconds.to_le_bytes());
    out[20] = entry.label_len;
    out[21..24].copy_from_slice(&[0u8; 3]);
    out[24..56].copy_from_slice(&entry.label);

    Ok(())
}

fn decode_bookmark_entry(input: &[u8]) -> Result<BookmarkEntry, BookmarkStateCodecError> {
    if input.len() != BOOKMARK_ENTRY_RECORD_LEN {
        return Err(BookmarkStateCodecError::InvalidLength);
    }

    let kind_byte = input[1];
    let kind = BookmarkEntryKind::from_u8(kind_byte)
        .ok_or(BookmarkStateCodecError::InvalidEntryKind(kind_byte))?;

    let unit_byte = input[2];
    let unit = BookmarkLocationUnit::from_u8(unit_byte)
        .ok_or(BookmarkStateCodecError::InvalidLocationUnit(unit_byte))?;

    let label_len = input[20];
    let label_len_usize = label_len as usize;
    if label_len_usize > BOOKMARK_LABEL_MAX_LEN {
        return Err(BookmarkStateCodecError::InvalidLabel);
    }

    let mut label = [0u8; BOOKMARK_LABEL_MAX_LEN];
    label.copy_from_slice(&input[24..56]);
    if core::str::from_utf8(&label[..label_len_usize]).is_err() {
        return Err(BookmarkStateCodecError::InvalidLabel);
    }

    Ok(BookmarkEntry {
        slot: input[0],
        kind,
        unit,
        flags: input[3],
        page_index: u32::from_le_bytes([input[4], input[5], input[6], input[7]]),
        logical_offset: u32::from_le_bytes([input[8], input[9], input[10], input[11]]),
        created_epoch_seconds: u64::from_le_bytes([
            input[12], input[13], input[14], input[15], input[16], input[17], input[18], input[19],
        ]),
        label_len,
        label,
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
    struct MemoryBookmarkIo {
        bookmark_path: heaplessless::String<40>,
        bookmark_bytes: [u8; BOOKMARK_FILE_LEN],
        has_bookmark: bool,
        index_path: heaplessless::String<24>,
        index_bytes: [u8; 64],
        index_len: usize,
        has_index: bool,
    }

    impl Default for MemoryBookmarkIo {
        fn default() -> Self {
            Self {
                bookmark_path: heaplessless::String::default(),
                bookmark_bytes: [0u8; BOOKMARK_FILE_LEN],
                has_bookmark: false,
                index_path: heaplessless::String::default(),
                index_bytes: [0u8; 64],
                index_len: 0,
                has_index: false,
            }
        }
    }

    impl BookmarkStateIo for MemoryBookmarkIo {
        type Error = ();

        fn read_bookmark_record(
            &mut self,
            path: &str,
            out: &mut [u8],
        ) -> Result<Option<usize>, Self::Error> {
            assert_eq!(path, self.bookmark_path.as_str());
            if !self.has_bookmark {
                return Ok(None);
            }
            out[..BOOKMARK_FILE_LEN].copy_from_slice(&self.bookmark_bytes);
            Ok(Some(BOOKMARK_FILE_LEN))
        }

        fn write_bookmark_record(&mut self, path: &str, bytes: &[u8]) -> Result<(), Self::Error> {
            self.bookmark_path.clear();
            self.bookmark_path.push_str(path).unwrap();
            self.bookmark_bytes.copy_from_slice(bytes);
            self.has_bookmark = true;
            Ok(())
        }

        fn read_bookmark_index(
            &mut self,
            path: &str,
            out: &mut [u8],
        ) -> Result<Option<usize>, Self::Error> {
            assert_eq!(path, self.index_path.as_str());
            if !self.has_index {
                return Ok(None);
            }
            out[..self.index_len].copy_from_slice(&self.index_bytes[..self.index_len]);
            Ok(Some(self.index_len))
        }

        fn write_bookmark_index(&mut self, path: &str, bytes: &[u8]) -> Result<(), Self::Error> {
            self.index_path.clear();
            self.index_path.push_str(path).unwrap();
            self.index_bytes[..bytes.len()].copy_from_slice(bytes);
            self.index_len = bytes.len();
            self.has_index = true;
            Ok(())
        }
    }

    #[test]
    fn bookmark_path_is_8_3_safe() {
        let mut path = [0u8; BOOKMARK_STATE_PATH_MAX_LEN];
        let book_id = BookId8::from_candidate("ALICE001");
        let len = write_bookmark_state_path(book_id, &mut path).unwrap();
        assert_eq!(
            core::str::from_utf8(&path[..len]).unwrap(),
            "state/ALICE001.BKM"
        );
    }

    #[test]
    fn bookmark_index_path_is_fixed() {
        let mut path = [0u8; BOOKMARK_INDEX_PATH_LEN];
        let len = write_bookmark_index_path(&mut path).unwrap();
        assert_eq!(
            core::str::from_utf8(&path[..len]).unwrap(),
            "state/BMIDX.TXT"
        );
    }

    #[test]
    fn encode_decode_round_trip() {
        let book_id = BookId8::from_candidate("ALICE001");
        let mut state = BookmarkState::empty(book_id, 1_765_000_000);
        state
            .push(BookmarkEntry::new(
                0,
                BookmarkLocationUnit::PageIndex,
                42,
                2048,
                "Rabbit hole",
                1_765_000_100,
            ))
            .unwrap();

        let mut record = [0u8; BOOKMARK_FILE_LEN];
        let len = encode_bookmark_state(state, &mut record).unwrap();
        assert_eq!(len, BOOKMARK_FILE_LEN);

        let restored = decode_bookmark_state(&record).unwrap();
        assert_eq!(restored, state);
        assert_eq!(restored.entries[0].label_str().unwrap(), "Rabbit hole");
    }

    #[test]
    fn adapter_delegates_read_write_to_io_boundary() {
        let io = MemoryBookmarkIo::default();
        let mut adapter = BookmarkStateIoAdapter::new(io);
        let book_id = BookId8::from_candidate("ALICE001");
        let mut state = BookmarkState::empty(book_id, 100);
        state
            .push(BookmarkEntry::new(
                0,
                BookmarkLocationUnit::PageIndex,
                7,
                700,
                "Chapter 1",
                101,
            ))
            .unwrap();

        adapter.write_bookmarks(state).unwrap();
        let restored = adapter.read_bookmarks(book_id).unwrap().unwrap();

        assert_eq!(restored, state);
    }

    #[test]
    fn corrupted_checksum_is_rejected() {
        let book_id = BookId8::from_candidate("ALICE001");
        let mut state = BookmarkState::empty(book_id, 100);
        state
            .push(BookmarkEntry::new(
                0,
                BookmarkLocationUnit::PageIndex,
                7,
                700,
                "Chapter 1",
                101,
            ))
            .unwrap();

        let mut record = [0u8; BOOKMARK_FILE_LEN];
        encode_bookmark_state(state, &mut record).unwrap();
        record[16] ^= 0xFF;

        assert_eq!(
            decode_bookmark_state(&record),
            Err(BookmarkStateCodecError::InvalidChecksum)
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
