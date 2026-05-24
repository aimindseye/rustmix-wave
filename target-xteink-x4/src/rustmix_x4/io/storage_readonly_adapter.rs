#![allow(dead_code)]

//! Rustmix-owned read-only storage adapter facade for the Xteink X4 target.
//!
//! This module defines contracts only. It intentionally does not mount SD cards,
//! probe media, arbitrate SPI, parse FAT, initialize display hardware, or call
//! into the active X4 runtime. A future X4-backed implementation can satisfy
//! these traits while keeping the existing `target-xteink-x4/src/rustmix_x4` SD/FAT behavior as
//! the active runtime path.

use core::str;

/// Static ownership record for the current storage facade slice.
pub struct RustmixReadonlyStorageContract;

impl RustmixReadonlyStorageContract {
    /// Marker used by static validation and optional smoke tests.
    pub const CONTRACT_MARKER: &'static str = "x4-storage-readonly-adapter-facade-ok";

    /// The facade is Rustmix-owned, but the active implementation remains imported.
    pub const FACADE_OWNER: &'static str = "target-xteink-x4 Rustmix adapter contract";
    pub const ACTIVE_STORAGE_BACKEND_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const ACTIVE_SD_DRIVER_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const ACTIVE_FAT_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const ACTIVE_SPI_ARBITRATION_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const ACTIVE_DISPLAY_OWNER: &'static str = "Rustmix-owned X4 runtime";

    /// Behavior movement guards. These must remain false until a later explicit
    /// hardware slice moves and validates one behavior path at a time.
    pub const SD_MOUNT_OR_PROBE_MOVED_TO_FACADE: bool = false;
    pub const SD_DRIVER_MOVED_TO_FACADE: bool = false;
    pub const FAT_BEHAVIOR_MOVED_TO_FACADE: bool = false;
    pub const SPI_ARBITRATION_MOVED_TO_FACADE: bool = false;
    pub const DISPLAY_BEHAVIOR_MOVED_TO_FACADE: bool = false;
    pub const WRITABLE_STORAGE_BEHAVIOR_ADDED: bool = false;

    pub const fn physical_behavior_moved() -> bool {
        Self::SD_MOUNT_OR_PROBE_MOVED_TO_FACADE
            || Self::SD_DRIVER_MOVED_TO_FACADE
            || Self::FAT_BEHAVIOR_MOVED_TO_FACADE
            || Self::SPI_ARBITRATION_MOVED_TO_FACADE
            || Self::DISPLAY_BEHAVIOR_MOVED_TO_FACADE
            || Self::WRITABLE_STORAGE_BEHAVIOR_ADDED
    }
}

/// Read-only storage path borrowed from the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixStoragePathRef<'a> {
    bytes: &'a [u8],
}

impl<'a> RustmixStoragePathRef<'a> {
    pub const fn from_static(bytes: &'static [u8]) -> RustmixStoragePathRef<'static> {
        RustmixStoragePathRef { bytes }
    }

    pub fn from_bytes(bytes: &'a [u8]) -> Option<Self> {
        if Self::is_valid_path(bytes) {
            Some(Self { bytes })
        } else {
            None
        }
    }

    pub fn from_str(path: &'a str) -> Option<Self> {
        Self::from_bytes(path.as_bytes())
    }

    pub const fn as_bytes(self) -> &'a [u8] {
        self.bytes
    }

    pub fn as_str(self) -> Option<&'a str> {
        str::from_utf8(self.bytes).ok()
    }

    pub const fn is_empty(self) -> bool {
        self.bytes.is_empty()
    }

    pub fn is_valid_path(bytes: &[u8]) -> bool {
        if bytes.is_empty() || bytes.len() > 255 {
            return false;
        }

        let mut i = 0usize;
        while i < bytes.len() {
            let b = bytes[i];
            if b == 0 || b == b'\\' {
                return false;
            }
            i += 1;
        }

        true
    }
}

/// Known active X4 storage paths as seen by the current X4-backed runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixResolvedStoragePaths<'a> {
    pub root: RustmixStoragePathRef<'a>,
    pub library_root: RustmixStoragePathRef<'a>,
    pub state_root: RustmixStoragePathRef<'a>,
    pub epub_cache_root: RustmixStoragePathRef<'a>,
    pub settings_file: RustmixStoragePathRef<'a>,
    pub title_cache_file: RustmixStoragePathRef<'a>,
    pub sleep_root: RustmixStoragePathRef<'a>,
    pub sleep_daily_root: RustmixStoragePathRef<'a>,
}

impl RustmixResolvedStoragePaths<'static> {
    /// Current path map. It mirrors the active X4 runtime layout but does not
    /// perform any storage IO.
    pub const X4_BACKED_ACTIVE_PATHS: Self = Self {
        root: RustmixStoragePathRef::from_static(b"/"),
        library_root: RustmixStoragePathRef::from_static(b"/"),
        state_root: RustmixStoragePathRef::from_static(b"/state"),
        epub_cache_root: RustmixStoragePathRef::from_static(b"/FCACHE"),
        settings_file: RustmixStoragePathRef::from_static(b"/RUSTMIX/SETTINGS.TXT"),
        title_cache_file: RustmixStoragePathRef::from_static(b"/RUSTMIX/TITLES.BIN"),
        sleep_root: RustmixStoragePathRef::from_static(b"/sleep"),
        sleep_daily_root: RustmixStoragePathRef::from_static(b"/sleep/daily"),
    };
}

/// Directory entry kind exposed by the read-only facade.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixDirectoryEntryKind {
    File,
    Directory,
    Other,
}

/// Directory metadata returned by `list_directory_metadata`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixDirectoryEntry {
    pub name: [u8; Self::MAX_NAME_LEN],
    pub name_len: usize,
    pub kind: RustmixDirectoryEntryKind,
    pub size_bytes: Option<u64>,
}

impl RustmixDirectoryEntry {
    pub const MAX_NAME_LEN: usize = 64;

    pub const fn empty() -> Self {
        Self {
            name: [0; Self::MAX_NAME_LEN],
            name_len: 0,
            kind: RustmixDirectoryEntryKind::Other,
            size_bytes: None,
        }
    }

    pub fn from_name(
        name: &[u8],
        kind: RustmixDirectoryEntryKind,
        size_bytes: Option<u64>,
    ) -> Option<Self> {
        if name.is_empty() || name.len() > Self::MAX_NAME_LEN {
            return None;
        }

        let mut out = Self::empty();
        out.name[..name.len()].copy_from_slice(name);
        out.name_len = name.len();
        out.kind = kind;
        out.size_bytes = size_bytes;
        Some(out)
    }

    pub fn name_bytes(&self) -> &[u8] {
        &self.name[..self.name_len]
    }
}

/// Result of reading a file window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixStorageReadChunk {
    pub offset: u64,
    pub bytes_read: usize,
    pub next_offset: u64,
    pub end_of_file: bool,
}

impl RustmixStorageReadChunk {
    pub const fn empty_at(offset: u64, end_of_file: bool) -> Self {
        Self {
            offset,
            bytes_read: 0,
            next_offset: offset,
            end_of_file,
        }
    }

    pub const fn from_read(offset: u64, bytes_read: usize, end_of_file: bool) -> Self {
        Self {
            offset,
            bytes_read,
            next_offset: offset + bytes_read as u64,
            end_of_file,
        }
    }
}

/// Optional facade-level contract error used by adapters that want a typed
/// preflight before delegating to a backend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixReadonlyStorageContractError {
    InvalidPath,
    EmptyOutputBuffer,
    DirectoryEntryBufferTooSmall,
}

/// Rustmix-owned read-only storage facade trait.
///
/// Implementors provide the actual backend. On X4 today, that backend should be
/// X4-backed and must keep SD driver, FAT, SPI arbitration, and display behavior
/// in `target-xteink-x4/src/rustmix_x4`.
pub trait RustmixReadonlyStorage {
    type Error;

    fn file_exists(&mut self, path: RustmixStoragePathRef<'_>) -> Result<bool, Self::Error>;

    fn read_file_start(
        &mut self,
        path: RustmixStoragePathRef<'_>,
        out: &mut [u8],
    ) -> Result<RustmixStorageReadChunk, Self::Error>;

    fn read_chunk(
        &mut self,
        path: RustmixStoragePathRef<'_>,
        offset: u64,
        out: &mut [u8],
    ) -> Result<RustmixStorageReadChunk, Self::Error>;

    fn list_directory_metadata(
        &mut self,
        path: RustmixStoragePathRef<'_>,
        out: &mut [RustmixDirectoryEntry],
    ) -> Result<usize, Self::Error>;

    fn resolve_current_storage_paths(&self) -> RustmixResolvedStoragePaths<'static>;
}

/// Thin delegating facade. This type owns no hardware behavior; it only exposes
/// the Rustmix contract surface around a backend implementation.
pub struct RustmixReadonlyStorageFacade<B> {
    backend: B,
}

impl<B> RustmixReadonlyStorageFacade<B> {
    pub const fn new(backend: B) -> Self {
        Self { backend }
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    pub fn into_backend(self) -> B {
        self.backend
    }
}

impl<B> RustmixReadonlyStorage for RustmixReadonlyStorageFacade<B>
where
    B: RustmixReadonlyStorage,
{
    type Error = B::Error;

    fn file_exists(&mut self, path: RustmixStoragePathRef<'_>) -> Result<bool, Self::Error> {
        self.backend.file_exists(path)
    }

    fn read_file_start(
        &mut self,
        path: RustmixStoragePathRef<'_>,
        out: &mut [u8],
    ) -> Result<RustmixStorageReadChunk, Self::Error> {
        self.backend.read_file_start(path, out)
    }

    fn read_chunk(
        &mut self,
        path: RustmixStoragePathRef<'_>,
        offset: u64,
        out: &mut [u8],
    ) -> Result<RustmixStorageReadChunk, Self::Error> {
        self.backend.read_chunk(path, offset, out)
    }

    fn list_directory_metadata(
        &mut self,
        path: RustmixStoragePathRef<'_>,
        out: &mut [RustmixDirectoryEntry],
    ) -> Result<usize, Self::Error> {
        self.backend.list_directory_metadata(path, out)
    }

    fn resolve_current_storage_paths(&self) -> RustmixResolvedStoragePaths<'static> {
        self.backend.resolve_current_storage_paths()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RustmixDirectoryEntry, RustmixDirectoryEntryKind, RustmixReadonlyStorage,
        RustmixReadonlyStorageContract, RustmixReadonlyStorageFacade, RustmixResolvedStoragePaths,
        RustmixStoragePathRef, RustmixStorageReadChunk,
    };

    struct ProbeBackend {
        exists_path: [u8; 64],
        exists_path_len: usize,
        read_path: [u8; 64],
        read_path_len: usize,
        read_offset: u64,
        listed_path: [u8; 64],
        listed_path_len: usize,
    }

    impl Default for ProbeBackend {
        fn default() -> Self {
            Self {
                exists_path: [0; 64],
                exists_path_len: 0,
                read_path: [0; 64],
                read_path_len: 0,
                read_offset: 0,
                listed_path: [0; 64],
                listed_path_len: 0,
            }
        }
    }

    fn remember_path(dst: &mut [u8; 64], len: &mut usize, path: RustmixStoragePathRef<'_>) {
        let bytes = path.as_bytes();
        let copy_len = core::cmp::min(bytes.len(), dst.len());
        dst[..copy_len].copy_from_slice(&bytes[..copy_len]);
        *len = copy_len;
    }

    impl ProbeBackend {
        fn remembered_read_path(&self) -> &[u8] {
            &self.read_path[..self.read_path_len]
        }

        fn remembered_listed_path(&self) -> &[u8] {
            &self.listed_path[..self.listed_path_len]
        }
    }

    impl RustmixReadonlyStorage for ProbeBackend {
        type Error = ();

        fn file_exists(&mut self, path: RustmixStoragePathRef<'_>) -> Result<bool, Self::Error> {
            remember_path(&mut self.exists_path, &mut self.exists_path_len, path);
            Ok(true)
        }

        fn read_file_start(
            &mut self,
            path: RustmixStoragePathRef<'_>,
            out: &mut [u8],
        ) -> Result<RustmixStorageReadChunk, Self::Error> {
            remember_path(&mut self.read_path, &mut self.read_path_len, path);
            self.read_offset = 0;
            let bytes_read = if out.is_empty() { 0 } else { 1 };
            if bytes_read > 0 {
                out[0] = b'V';
            }
            Ok(RustmixStorageReadChunk::from_read(0, bytes_read, false))
        }

        fn read_chunk(
            &mut self,
            path: RustmixStoragePathRef<'_>,
            offset: u64,
            out: &mut [u8],
        ) -> Result<RustmixStorageReadChunk, Self::Error> {
            remember_path(&mut self.read_path, &mut self.read_path_len, path);
            self.read_offset = offset;
            let bytes_read = if out.is_empty() { 0 } else { 1 };
            if bytes_read > 0 {
                out[0] = b'X';
            }
            Ok(RustmixStorageReadChunk::from_read(offset, bytes_read, true))
        }

        fn list_directory_metadata(
            &mut self,
            path: RustmixStoragePathRef<'_>,
            out: &mut [RustmixDirectoryEntry],
        ) -> Result<usize, Self::Error> {
            remember_path(&mut self.listed_path, &mut self.listed_path_len, path);
            if !out.is_empty() {
                out[0] = RustmixDirectoryEntry::from_name(
                    b"BOOK.TXT",
                    RustmixDirectoryEntryKind::File,
                    Some(42),
                )
                .unwrap();
                Ok(1)
            } else {
                Ok(0)
            }
        }

        fn resolve_current_storage_paths(&self) -> RustmixResolvedStoragePaths<'static> {
            RustmixResolvedStoragePaths::X4_BACKED_ACTIVE_PATHS
        }
    }

    #[test]
    fn facade_delegates_read_only_contracts() {
        let mut facade = RustmixReadonlyStorageFacade::new(ProbeBackend::default());
        let path = RustmixStoragePathRef::from_str("/BOOK.TXT").unwrap();
        let mut out = [0u8; 4];
        let chunk = facade.read_file_start(path, &mut out).unwrap();
        assert_eq!(chunk.bytes_read, 1);
        assert_eq!(out[0], b'V');
        assert_eq!(facade.backend().remembered_read_path(), b"/BOOK.TXT");

        let chunk = facade.read_chunk(path, 32, &mut out).unwrap();
        assert_eq!(chunk.next_offset, 33);
        assert!(chunk.end_of_file);
        assert_eq!(facade.backend().read_offset, 32);
    }

    #[test]
    fn facade_lists_directory_metadata_without_write_contracts() {
        let mut facade = RustmixReadonlyStorageFacade::new(ProbeBackend::default());
        let path = RustmixStoragePathRef::from_str("/").unwrap();
        let mut entries = [RustmixDirectoryEntry::empty()];

        assert_eq!(facade.list_directory_metadata(path, &mut entries), Ok(1));
        assert_eq!(entries[0].name_bytes(), b"BOOK.TXT");
        assert_eq!(entries[0].size_bytes, Some(42));
        assert_eq!(facade.backend().remembered_listed_path(), b"/");
    }

    #[test]
    fn resolved_paths_match_x4_backed_runtime_layout() {
        let paths = RustmixResolvedStoragePaths::X4_BACKED_ACTIVE_PATHS;
        assert_eq!(paths.root.as_bytes(), b"/");
        assert_eq!(paths.state_root.as_bytes(), b"/state");
        assert_eq!(paths.epub_cache_root.as_bytes(), b"/FCACHE");
        assert_eq!(paths.settings_file.as_bytes(), b"/RUSTMIX/SETTINGS.TXT");
        assert_eq!(paths.title_cache_file.as_bytes(), b"/RUSTMIX/TITLES.BIN");
    }

    #[test]
    fn contract_keeps_physical_behavior_imported() {
        assert!(!RustmixReadonlyStorageContract::physical_behavior_moved());
        assert_eq!(
            RustmixReadonlyStorageContract::ACTIVE_STORAGE_BACKEND_OWNER,
            "Rustmix-owned X4 runtime"
        );
    }

    #[test]
    fn path_ref_rejects_empty_null_and_backslash_paths() {
        assert!(RustmixStoragePathRef::from_str("/state/BMIDX.TXT").is_some());
        assert!(RustmixStoragePathRef::from_str("").is_none());
        assert!(RustmixStoragePathRef::from_bytes(b"/bad\0path").is_none());
        assert!(RustmixStoragePathRef::from_str("\\bad").is_none());
    }
}
