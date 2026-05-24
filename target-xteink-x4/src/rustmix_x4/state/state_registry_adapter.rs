//! State Registry Adapter.
//!
//! This module is deliberately pure and hardware-free. It provides a single
//! Rustmix-owned registry/facade for the typed per-book state files introduced
//! in the state I/O adapter set
//!
//! Owned here:
//! - typed state kind registry
//! - path/spec discovery for `state/<BOOKID>.PRG`, `state/<BOOKID>.THM`, `state/<BOOKID>.MTA`, and `state/<BOOKID>.BKM`
//! - bookmark index path discovery for `state/BMIDX.TXT`
//! - optional inventory adapter trait over caller-provided byte/file metadata I/O
//! - validation marker
//!
//! Not owned here:
//! - physical media access
//! - filesystem implementation
//! - reader rendering
//! - live app behavior

#![allow(dead_code)]

use super::bookmark_state_io_adapter::{
    BOOKMARK_FILE_LEN, BOOKMARK_INDEX_FILE_NAME, BOOKMARK_INDEX_PATH_LEN, BOOKMARK_STATE_EXTENSION,
    BOOKMARK_STATE_PATH_MAX_LEN,
};
use super::metadata_state_io_adapter::{
    METADATA_RECORD_LEN, METADATA_STATE_EXTENSION, METADATA_STATE_PATH_MAX_LEN,
};
use super::progress_state_io_adapter::{
    BookId8, PROGRESS_RECORD_LEN, PROGRESS_STATE_EXTENSION, PROGRESS_STATE_PATH_MAX_LEN,
};
use super::theme_state_io_adapter::{
    THEME_RECORD_LEN, THEME_STATE_EXTENSION, THEME_STATE_PATH_MAX_LEN,
};

/// Validation marker emitted by validation / boot marker plumbing.
pub const STATE_REGISTRY_ADAPTER_MARKER: &str = "x4-state-registry-adapter-ok";

pub const STATE_REGISTRY_DIR: &str = "state";
pub const STATE_REGISTRY_FILE_COUNT: usize = 4;
pub const STATE_REGISTRY_MAX_PATH_LEN: usize = max_usize4(
    PROGRESS_STATE_PATH_MAX_LEN,
    THEME_STATE_PATH_MAX_LEN,
    METADATA_STATE_PATH_MAX_LEN,
    BOOKMARK_STATE_PATH_MAX_LEN,
);
pub const STATE_REGISTRY_INDEX_PATH_MAX_LEN: usize = BOOKMARK_INDEX_PATH_LEN;

pub const STATE_REGISTRY_KINDS: [TypedStateFileKind; STATE_REGISTRY_FILE_COUNT] = [
    TypedStateFileKind::Progress,
    TypedStateFileKind::Theme,
    TypedStateFileKind::Metadata,
    TypedStateFileKind::Bookmark,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TypedStateFileKind {
    Progress = 0,
    Theme = 1,
    Metadata = 2,
    Bookmark = 3,
}

impl TypedStateFileKind {
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Progress),
            1 => Some(Self::Theme),
            2 => Some(Self::Metadata),
            3 => Some(Self::Bookmark),
            _ => None,
        }
    }

    pub const fn extension(self) -> &'static str {
        match self {
            Self::Progress => PROGRESS_STATE_EXTENSION,
            Self::Theme => THEME_STATE_EXTENSION,
            Self::Metadata => METADATA_STATE_EXTENSION,
            Self::Bookmark => BOOKMARK_STATE_EXTENSION,
        }
    }

    pub const fn expected_record_len(self) -> usize {
        match self {
            Self::Progress => PROGRESS_RECORD_LEN,
            Self::Theme => THEME_RECORD_LEN,
            Self::Metadata => METADATA_RECORD_LEN,
            Self::Bookmark => BOOKMARK_FILE_LEN,
        }
    }

    pub const fn path_max_len(self) -> usize {
        match self {
            Self::Progress => PROGRESS_STATE_PATH_MAX_LEN,
            Self::Theme => THEME_STATE_PATH_MAX_LEN,
            Self::Metadata => METADATA_STATE_PATH_MAX_LEN,
            Self::Bookmark => BOOKMARK_STATE_PATH_MAX_LEN,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Progress => "progress",
            Self::Theme => "theme",
            Self::Metadata => "metadata",
            Self::Bookmark => "bookmark",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateFileSpec {
    pub kind: TypedStateFileKind,
    pub extension: &'static str,
    pub expected_record_len: usize,
    pub path_max_len: usize,
}

impl StateFileSpec {
    pub const fn for_kind(kind: TypedStateFileKind) -> Self {
        Self {
            kind,
            extension: kind.extension(),
            expected_record_len: kind.expected_record_len(),
            path_max_len: kind.path_max_len(),
        }
    }
}

pub const STATE_FILE_SPECS: [StateFileSpec; STATE_REGISTRY_FILE_COUNT] = [
    StateFileSpec::for_kind(TypedStateFileKind::Progress),
    StateFileSpec::for_kind(TypedStateFileKind::Theme),
    StateFileSpec::for_kind(TypedStateFileKind::Metadata),
    StateFileSpec::for_kind(TypedStateFileKind::Bookmark),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateRegistryCodecError {
    PathBufferTooSmall,
    InvalidBookId,
    InvalidUtf8Path,
    UnknownKind(u8),
}

#[derive(Debug, Eq, PartialEq)]
pub enum StateRegistryAdapterError<E> {
    Io(E),
    Codec(StateRegistryCodecError),
}

impl<E> From<StateRegistryCodecError> for StateRegistryAdapterError<E> {
    fn from(value: StateRegistryCodecError) -> Self {
        Self::Codec(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatePath {
    pub kind: TypedStateFileKind,
    len: usize,
    bytes: [u8; STATE_REGISTRY_MAX_PATH_LEN],
}

impl StatePath {
    pub const fn empty(kind: TypedStateFileKind) -> Self {
        Self {
            kind,
            len: 0,
            bytes: [0; STATE_REGISTRY_MAX_PATH_LEN],
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    pub fn as_str(&self) -> Result<&str, StateRegistryCodecError> {
        core::str::from_utf8(self.as_bytes()).map_err(|_| StateRegistryCodecError::InvalidUtf8Path)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateRecordStat {
    pub len: usize,
    pub updated_epoch_seconds: u64,
}

impl StateRecordStat {
    pub const fn new(len: usize, updated_epoch_seconds: u64) -> Self {
        Self {
            len,
            updated_epoch_seconds,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateInventoryEntry {
    pub kind: TypedStateFileKind,
    pub expected_record_len: usize,
    pub present: bool,
    pub observed_len: usize,
    pub updated_epoch_seconds: u64,
}

impl StateInventoryEntry {
    pub const fn missing(kind: TypedStateFileKind) -> Self {
        Self {
            kind,
            expected_record_len: kind.expected_record_len(),
            present: false,
            observed_len: 0,
            updated_epoch_seconds: 0,
        }
    }

    pub const fn from_stat(kind: TypedStateFileKind, stat: StateRecordStat) -> Self {
        Self {
            kind,
            expected_record_len: kind.expected_record_len(),
            present: true,
            observed_len: stat.len,
            updated_epoch_seconds: stat.updated_epoch_seconds,
        }
    }

    pub const fn length_matches_spec(&self) -> bool {
        !self.present || self.observed_len == self.expected_record_len
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateInventory {
    pub book_id: BookId8,
    pub entries: [StateInventoryEntry; STATE_REGISTRY_FILE_COUNT],
}

impl StateInventory {
    pub const fn empty(book_id: BookId8) -> Self {
        Self {
            book_id,
            entries: [
                StateInventoryEntry::missing(TypedStateFileKind::Progress),
                StateInventoryEntry::missing(TypedStateFileKind::Theme),
                StateInventoryEntry::missing(TypedStateFileKind::Metadata),
                StateInventoryEntry::missing(TypedStateFileKind::Bookmark),
            ],
        }
    }

    pub fn entry(&self, kind: TypedStateFileKind) -> Option<&StateInventoryEntry> {
        let mut idx = 0;
        while idx < self.entries.len() {
            if self.entries[idx].kind == kind {
                return Some(&self.entries[idx]);
            }
            idx += 1;
        }
        None
    }

    pub fn present_count(&self) -> usize {
        let mut count = 0;
        let mut idx = 0;
        while idx < self.entries.len() {
            if self.entries[idx].present {
                count += 1;
            }
            idx += 1;
        }
        count
    }

    pub fn all_present_lengths_match_spec(&self) -> bool {
        let mut idx = 0;
        while idx < self.entries.len() {
            if !self.entries[idx].length_matches_spec() {
                return false;
            }
            idx += 1;
        }
        true
    }
}

/// Storage-facing metadata boundary for the typed state registry.
///
/// A future concrete implementation should bridge this trait to the existing
/// X4 storage runtime. This trait intentionally knows only paths and metadata.
pub trait StateRegistryIo {
    type Error;

    /// Return metadata for `path`.
    ///
    /// Return `Ok(None)` when the record is absent.
    fn stat_state_record(&mut self, path: &str) -> Result<Option<StateRecordStat>, Self::Error>;
}

pub struct StateRegistryAdapter<IO> {
    io: IO,
}

impl<IO> StateRegistryAdapter<IO> {
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

impl<IO> StateRegistryAdapter<IO>
where
    IO: StateRegistryIo,
{
    pub fn scan_book(
        &mut self,
        book_id: BookId8,
    ) -> Result<StateInventory, StateRegistryAdapterError<IO::Error>> {
        let mut inventory = StateInventory::empty(book_id);
        let mut idx = 0;

        while idx < STATE_REGISTRY_KINDS.len() {
            let kind = STATE_REGISTRY_KINDS[idx];
            let path = make_state_path(kind, book_id)?;
            let path = path.as_str()?;
            inventory.entries[idx] = match self.io.stat_state_record(path) {
                Ok(Some(stat)) => StateInventoryEntry::from_stat(kind, stat),
                Ok(None) => StateInventoryEntry::missing(kind),
                Err(err) => return Err(StateRegistryAdapterError::Io(err)),
            };
            idx += 1;
        }

        Ok(inventory)
    }
}

pub fn spec_for_kind(kind: TypedStateFileKind) -> StateFileSpec {
    StateFileSpec::for_kind(kind)
}

pub fn make_state_path(
    kind: TypedStateFileKind,
    book_id: BookId8,
) -> Result<StatePath, StateRegistryCodecError> {
    let mut path = StatePath::empty(kind);
    path.len = write_typed_state_path(kind, book_id, &mut path.bytes)?;
    Ok(path)
}

pub fn write_typed_state_path(
    kind: TypedStateFileKind,
    book_id: BookId8,
    out: &mut [u8],
) -> Result<usize, StateRegistryCodecError> {
    let required = kind.path_max_len();
    if out.len() < required {
        return Err(StateRegistryCodecError::PathBufferTooSmall);
    }

    let book_id_bytes = book_id.as_bytes();
    if !book_id_bytes.iter().all(|byte| is_safe_book_id_byte(*byte)) {
        return Err(StateRegistryCodecError::InvalidBookId);
    }

    let mut idx = 0;
    for byte in STATE_REGISTRY_DIR.as_bytes() {
        out[idx] = *byte;
        idx += 1;
    }
    out[idx] = b'/';
    idx += 1;

    for byte in book_id_bytes {
        out[idx] = *byte;
        idx += 1;
    }

    out[idx] = b'.';
    idx += 1;

    for byte in kind.extension().as_bytes() {
        out[idx] = *byte;
        idx += 1;
    }

    Ok(idx)
}

pub fn write_bookmark_index_path(out: &mut [u8]) -> Result<usize, StateRegistryCodecError> {
    if out.len() < STATE_REGISTRY_INDEX_PATH_MAX_LEN {
        return Err(StateRegistryCodecError::PathBufferTooSmall);
    }

    let mut idx = 0;
    for byte in STATE_REGISTRY_DIR.as_bytes() {
        out[idx] = *byte;
        idx += 1;
    }
    out[idx] = b'/';
    idx += 1;

    for byte in BOOKMARK_INDEX_FILE_NAME.as_bytes() {
        out[idx] = *byte;
        idx += 1;
    }

    Ok(idx)
}

const fn max_usize4(a: usize, b: usize, c: usize, d: usize) -> usize {
    let ab = if a > b { a } else { b };
    let cd = if c > d { c } else { d };
    if ab > cd { ab } else { cd }
}

fn is_safe_book_id_byte(value: u8) -> bool {
    value.is_ascii_uppercase() || value.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lists_all_typed_state_kinds_in_stable_order() {
        assert_eq!(STATE_REGISTRY_KINDS[0], TypedStateFileKind::Progress);
        assert_eq!(STATE_REGISTRY_KINDS[1], TypedStateFileKind::Theme);
        assert_eq!(STATE_REGISTRY_KINDS[2], TypedStateFileKind::Metadata);
        assert_eq!(STATE_REGISTRY_KINDS[3], TypedStateFileKind::Bookmark);
    }

    #[test]
    fn path_builder_uses_83_book_id_and_registered_extension() {
        let book_id = BookId8::from_bytes_unchecked(*b"8A79A61F");
        let path = make_state_path(TypedStateFileKind::Metadata, book_id).unwrap();
        assert_eq!(path.as_str().unwrap(), "state/8A79A61F.MTA");
    }

    #[test]
    fn bookmark_index_path_is_registered() {
        let mut out = [0u8; STATE_REGISTRY_INDEX_PATH_MAX_LEN];
        let len = write_bookmark_index_path(&mut out).unwrap();
        assert_eq!(
            core::str::from_utf8(&out[..len]).unwrap(),
            "state/BMIDX.TXT"
        );
    }

    #[test]
    fn inventory_counts_present_entries() {
        let book_id = BookId8::from_bytes_unchecked(*b"8A79A61F");
        let mut inventory = StateInventory::empty(book_id);
        inventory.entries[0] = StateInventoryEntry::from_stat(
            TypedStateFileKind::Progress,
            StateRecordStat::new(PROGRESS_RECORD_LEN, 12),
        );
        assert_eq!(inventory.present_count(), 1);
        assert!(inventory.all_present_lengths_match_spec());
    }
}
