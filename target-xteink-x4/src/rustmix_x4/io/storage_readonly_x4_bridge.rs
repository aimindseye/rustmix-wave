#![allow(dead_code)]

//! X4-backed read-only implementation bridge for Rustmix storage facade.
//!
//! This module adds an adapter boundary only. The generic bridge is pure and can
//! be checked without SD hardware. The embedded X4 backend, enabled for the
//! active firmware target, delegates read/list/size calls to the existing X4
//! storage helpers in `crate::rustmix_x4::x4_kernel::drivers::storage`.
//!
//! It intentionally does not mount or probe SD, arbitrate SPI, drive FAT
//! directly, refresh display hardware, or change reader/file-browser behavior.

use crate::rustmix_x4::io::storage_readonly_adapter::{
    RustmixDirectoryEntry, RustmixDirectoryEntryKind, RustmixReadonlyStorage,
    RustmixReadonlyStorageContractError, RustmixResolvedStoragePaths, RustmixStoragePathRef,
    RustmixStorageReadChunk,
};

pub const STORAGE_READONLY_X4_BRIDGE_MARKER: &str = "x4-storage-readonly-x4-bridge-ok";
pub const X4_READONLY_BRIDGE_OWNER: &str = "target-xteink-x4 Rustmix adapter bridge";
pub const X4_READONLY_ACTIVE_BACKEND_OWNER: &str = "Rustmix-owned X4 runtime";
pub const X4_READONLY_MAX_DIR_ENTRIES: usize = 32;
pub const X4_READONLY_X4_DIR: &str = "RUSTMIX";

pub struct RustmixStorageReadonlyX4BridgeContract;

impl RustmixStorageReadonlyX4BridgeContract {
    pub const BRIDGE_MARKER: &'static str = STORAGE_READONLY_X4_BRIDGE_MARKER;
    pub const BRIDGE_OWNER: &'static str = X4_READONLY_BRIDGE_OWNER;
    pub const ACTIVE_BACKEND_OWNER: &'static str = X4_READONLY_ACTIVE_BACKEND_OWNER;

    pub const FILE_EXISTS_MAPPING: &'static str = "file_exists -> X4 file_size helpers";
    pub const READ_FILE_START_MAPPING: &'static str =
        "read_file_start -> X4 read_file_start helpers";
    pub const READ_CHUNK_MAPPING: &'static str = "read_chunk -> X4 read_file_chunk helpers";
    pub const LIST_DIRECTORY_MAPPING: &'static str =
        "list_directory_metadata -> X4 list_*_entries helpers";
    pub const RESOLVE_PATHS_MAPPING: &'static str =
        "resolve_current_storage_paths -> facade X4_BACKED_ACTIVE_PATHS";

    pub const SD_MOUNT_OR_PROBE_MOVED_TO_BRIDGE: bool = false;
    pub const SD_DRIVER_MOVED_TO_BRIDGE: bool = false;
    pub const FAT_BEHAVIOR_MOVED_TO_BRIDGE: bool = false;
    pub const SPI_ARBITRATION_MOVED_TO_BRIDGE: bool = false;
    pub const DISPLAY_BEHAVIOR_MOVED_TO_BRIDGE: bool = false;
    pub const READER_OR_FILE_BROWSER_BEHAVIOR_CHANGED: bool = false;
    pub const WRITABLE_STORAGE_BEHAVIOR_ADDED: bool = false;

    pub const fn physical_behavior_moved() -> bool {
        Self::SD_MOUNT_OR_PROBE_MOVED_TO_BRIDGE
            || Self::SD_DRIVER_MOVED_TO_BRIDGE
            || Self::FAT_BEHAVIOR_MOVED_TO_BRIDGE
            || Self::SPI_ARBITRATION_MOVED_TO_BRIDGE
            || Self::DISPLAY_BEHAVIOR_MOVED_TO_BRIDGE
            || Self::READER_OR_FILE_BROWSER_BEHAVIOR_CHANGED
            || Self::WRITABLE_STORAGE_BEHAVIOR_ADDED
    }

    pub const fn active_runtime_preflight() -> StorageReadonlyX4BridgePreflight {
        StorageReadonlyX4BridgePreflight {
            bridge_marker_present: !Self::BRIDGE_MARKER.is_empty(),
            active_backend_is_x4: !Self::ACTIVE_BACKEND_OWNER.is_empty(),
            physical_behavior_moved: Self::physical_behavior_moved(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageReadonlyX4BridgePreflight {
    pub bridge_marker_present: bool,
    pub active_backend_is_x4: bool,
    pub physical_behavior_moved: bool,
}

impl StorageReadonlyX4BridgePreflight {
    pub const fn ok(self) -> bool {
        self.bridge_marker_present && self.active_backend_is_x4 && !self.physical_behavior_moved
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X4ReadonlyStorageBridgeError<E> {
    Contract(RustmixReadonlyStorageContractError),
    Backend(E),
    NonUtf8Path,
    UnsupportedPath,
    NotFile,
    NotDirectory,
    OffsetTooLarge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X4ReadonlyDirectoryEntry {
    pub name: [u8; RustmixDirectoryEntry::MAX_NAME_LEN],
    pub name_len: usize,
    pub is_dir: bool,
    pub size_bytes: u64,
}

impl X4ReadonlyDirectoryEntry {
    pub const EMPTY: Self = Self {
        name: [0; RustmixDirectoryEntry::MAX_NAME_LEN],
        name_len: 0,
        is_dir: false,
        size_bytes: 0,
    };

    pub fn from_name(name: &[u8], is_dir: bool, size_bytes: u64) -> Option<Self> {
        if name.is_empty() || name.len() > RustmixDirectoryEntry::MAX_NAME_LEN {
            return None;
        }

        let mut out = Self::EMPTY;
        out.name[..name.len()].copy_from_slice(name);
        out.name_len = name.len();
        out.is_dir = is_dir;
        out.size_bytes = size_bytes;
        Some(out)
    }

    pub fn name_bytes(&self) -> &[u8] {
        &self.name[..self.name_len]
    }

    pub fn to_rustmix_entry(self) -> Option<RustmixDirectoryEntry> {
        let kind = if self.is_dir {
            RustmixDirectoryEntryKind::Directory
        } else {
            RustmixDirectoryEntryKind::File
        };
        let size = if self.is_dir {
            None
        } else {
            Some(self.size_bytes)
        };
        RustmixDirectoryEntry::from_name(self.name_bytes(), kind, size)
    }
}

/// Minimal backend contract consumed by the bridge.
///
/// The active embedded implementation delegates to X4 storage helpers. Tests
/// and host/static checks can use a fake backend without constructing SD
/// hardware, mounted FAT state, SPI, or display objects.
pub trait X4ReadonlyStorageBackend {
    type Error;

    fn file_size_root(&mut self, name: &str) -> Result<u32, Self::Error>;
    fn file_size_in_dir(&mut self, dir: &str, name: &str) -> Result<u32, Self::Error>;
    fn file_size_in_subdir(
        &mut self,
        dir: &str,
        subdir: &str,
        name: &str,
    ) -> Result<u32, Self::Error>;

    fn read_file_start_root(
        &mut self,
        name: &str,
        out: &mut [u8],
    ) -> Result<(u32, usize), Self::Error>;
    fn read_file_start_in_dir(
        &mut self,
        dir: &str,
        name: &str,
        out: &mut [u8],
    ) -> Result<(u32, usize), Self::Error>;
    fn read_file_start_in_subdir(
        &mut self,
        dir: &str,
        subdir: &str,
        name: &str,
        out: &mut [u8],
    ) -> Result<(u32, usize), Self::Error>;

    fn read_file_chunk_root(
        &mut self,
        name: &str,
        offset: u32,
        out: &mut [u8],
    ) -> Result<usize, Self::Error>;
    fn read_file_chunk_in_dir(
        &mut self,
        dir: &str,
        name: &str,
        offset: u32,
        out: &mut [u8],
    ) -> Result<usize, Self::Error>;
    fn read_file_chunk_in_subdir(
        &mut self,
        dir: &str,
        subdir: &str,
        name: &str,
        offset: u32,
        out: &mut [u8],
    ) -> Result<usize, Self::Error>;

    fn list_root_entries(
        &mut self,
        out: &mut [X4ReadonlyDirectoryEntry],
    ) -> Result<usize, Self::Error>;
    fn list_dir_entries(
        &mut self,
        dir: &str,
        out: &mut [X4ReadonlyDirectoryEntry],
    ) -> Result<usize, Self::Error>;
    fn list_subdir_entries(
        &mut self,
        dir: &str,
        subdir: &str,
        out: &mut [X4ReadonlyDirectoryEntry],
    ) -> Result<usize, Self::Error>;
}

pub struct X4ReadonlyStorageBridge<B> {
    backend: B,
}

impl<B> X4ReadonlyStorageBridge<B> {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum X4ReadonlyPath<'a> {
    Root,
    RootFile(&'a str),
    RootDirectory(&'a str),
    DirFile {
        dir: &'a str,
        name: &'a str,
    },
    Subdir {
        dir: &'a str,
        subdir: &'a str,
    },
    SubdirFile {
        dir: &'a str,
        subdir: &'a str,
        name: &'a str,
    },
}

fn normalize_x4_dir_name(name: &str) -> &str {
    if name.eq_ignore_ascii_case("_x4") {
        X4_READONLY_X4_DIR
    } else {
        name
    }
}

fn parse_storage_path<'a, E>(
    path: RustmixStoragePathRef<'a>,
    for_directory: bool,
) -> Result<X4ReadonlyPath<'a>, X4ReadonlyStorageBridgeError<E>> {
    let path = path
        .as_str()
        .ok_or(X4ReadonlyStorageBridgeError::NonUtf8Path)?;
    let rest = path.strip_prefix('/').unwrap_or(path);

    if rest.is_empty() {
        return if for_directory {
            Ok(X4ReadonlyPath::Root)
        } else {
            Err(X4ReadonlyStorageBridgeError::NotFile)
        };
    }

    let mut parts = [""; 3];
    let mut count = 0usize;
    for part in rest.split('/') {
        if part.is_empty() {
            return Err(X4ReadonlyStorageBridgeError::UnsupportedPath);
        }
        if count >= parts.len() {
            return Err(X4ReadonlyStorageBridgeError::UnsupportedPath);
        }
        parts[count] = part;
        count += 1;
    }

    match (count, for_directory) {
        (1, true) => Ok(X4ReadonlyPath::RootDirectory(normalize_x4_dir_name(
            parts[0],
        ))),
        (1, false) => Ok(X4ReadonlyPath::RootFile(parts[0])),
        (2, true) => Ok(X4ReadonlyPath::Subdir {
            dir: normalize_x4_dir_name(parts[0]),
            subdir: parts[1],
        }),
        (2, false) => Ok(X4ReadonlyPath::DirFile {
            dir: normalize_x4_dir_name(parts[0]),
            name: parts[1],
        }),
        (3, true) => Err(X4ReadonlyStorageBridgeError::NotDirectory),
        (3, false) => Ok(X4ReadonlyPath::SubdirFile {
            dir: normalize_x4_dir_name(parts[0]),
            subdir: parts[1],
            name: parts[2],
        }),
        _ => Err(X4ReadonlyStorageBridgeError::UnsupportedPath),
    }
}

impl<B> X4ReadonlyStorageBridge<B>
where
    B: X4ReadonlyStorageBackend,
{
    fn file_size(
        &mut self,
        path: X4ReadonlyPath<'_>,
    ) -> Result<u32, X4ReadonlyStorageBridgeError<B::Error>> {
        match path {
            X4ReadonlyPath::RootFile(name) => self
                .backend
                .file_size_root(name)
                .map_err(X4ReadonlyStorageBridgeError::Backend),
            X4ReadonlyPath::DirFile { dir, name } => self
                .backend
                .file_size_in_dir(dir, name)
                .map_err(X4ReadonlyStorageBridgeError::Backend),
            X4ReadonlyPath::SubdirFile { dir, subdir, name } => self
                .backend
                .file_size_in_subdir(dir, subdir, name)
                .map_err(X4ReadonlyStorageBridgeError::Backend),
            X4ReadonlyPath::Root
            | X4ReadonlyPath::RootDirectory(_)
            | X4ReadonlyPath::Subdir { .. } => Err(X4ReadonlyStorageBridgeError::NotFile),
        }
    }

    fn read_start(
        &mut self,
        path: X4ReadonlyPath<'_>,
        out: &mut [u8],
    ) -> Result<RustmixStorageReadChunk, X4ReadonlyStorageBridgeError<B::Error>> {
        if out.is_empty() {
            return Err(X4ReadonlyStorageBridgeError::Contract(
                RustmixReadonlyStorageContractError::EmptyOutputBuffer,
            ));
        }

        let (size, bytes_read) = match path {
            X4ReadonlyPath::RootFile(name) => self.backend.read_file_start_root(name, out),
            X4ReadonlyPath::DirFile { dir, name } => {
                self.backend.read_file_start_in_dir(dir, name, out)
            }
            X4ReadonlyPath::SubdirFile { dir, subdir, name } => self
                .backend
                .read_file_start_in_subdir(dir, subdir, name, out),
            X4ReadonlyPath::Root
            | X4ReadonlyPath::RootDirectory(_)
            | X4ReadonlyPath::Subdir { .. } => {
                return Err(X4ReadonlyStorageBridgeError::NotFile);
            }
        }
        .map_err(X4ReadonlyStorageBridgeError::Backend)?;

        Ok(RustmixStorageReadChunk::from_read(
            0,
            bytes_read,
            bytes_read as u64 >= size as u64,
        ))
    }

    fn read_at(
        &mut self,
        path: X4ReadonlyPath<'_>,
        offset: u64,
        out: &mut [u8],
    ) -> Result<RustmixStorageReadChunk, X4ReadonlyStorageBridgeError<B::Error>> {
        if offset > u32::MAX as u64 {
            return Err(X4ReadonlyStorageBridgeError::OffsetTooLarge);
        }
        if out.is_empty() {
            return Err(X4ReadonlyStorageBridgeError::Contract(
                RustmixReadonlyStorageContractError::EmptyOutputBuffer,
            ));
        }

        let file_size = self.file_size(path)?;
        if offset >= file_size as u64 {
            return Ok(RustmixStorageReadChunk::empty_at(offset, true));
        }

        let offset32 = offset as u32;
        let bytes_read = match path {
            X4ReadonlyPath::RootFile(name) => {
                self.backend.read_file_chunk_root(name, offset32, out)
            }
            X4ReadonlyPath::DirFile { dir, name } => self
                .backend
                .read_file_chunk_in_dir(dir, name, offset32, out),
            X4ReadonlyPath::SubdirFile { dir, subdir, name } => self
                .backend
                .read_file_chunk_in_subdir(dir, subdir, name, offset32, out),
            X4ReadonlyPath::Root
            | X4ReadonlyPath::RootDirectory(_)
            | X4ReadonlyPath::Subdir { .. } => {
                return Err(X4ReadonlyStorageBridgeError::NotFile);
            }
        }
        .map_err(X4ReadonlyStorageBridgeError::Backend)?;

        Ok(RustmixStorageReadChunk::from_read(
            offset,
            bytes_read,
            offset + bytes_read as u64 >= file_size as u64,
        ))
    }

    fn list_entries(
        &mut self,
        path: X4ReadonlyPath<'_>,
        out: &mut [RustmixDirectoryEntry],
    ) -> Result<usize, X4ReadonlyStorageBridgeError<B::Error>> {
        if out.is_empty() {
            return Err(X4ReadonlyStorageBridgeError::Contract(
                RustmixReadonlyStorageContractError::DirectoryEntryBufferTooSmall,
            ));
        }

        let mut scratch = [X4ReadonlyDirectoryEntry::EMPTY; X4_READONLY_MAX_DIR_ENTRIES];
        let limit = core::cmp::min(out.len(), scratch.len());
        let count = match path {
            X4ReadonlyPath::Root => self.backend.list_root_entries(&mut scratch[..limit]),
            X4ReadonlyPath::RootDirectory(dir) => {
                self.backend.list_dir_entries(dir, &mut scratch[..limit])
            }
            X4ReadonlyPath::Subdir { dir, subdir } => {
                self.backend
                    .list_subdir_entries(dir, subdir, &mut scratch[..limit])
            }
            X4ReadonlyPath::RootFile(_)
            | X4ReadonlyPath::DirFile { .. }
            | X4ReadonlyPath::SubdirFile { .. } => {
                return Err(X4ReadonlyStorageBridgeError::NotDirectory);
            }
        }
        .map_err(X4ReadonlyStorageBridgeError::Backend)?;

        let mut written = 0usize;
        let capped = core::cmp::min(count, limit);
        while written < capped {
            if let Some(entry) = scratch[written].to_rustmix_entry() {
                out[written] = entry;
                written += 1;
            } else {
                break;
            }
        }

        Ok(written)
    }
}

impl<B> RustmixReadonlyStorage for X4ReadonlyStorageBridge<B>
where
    B: X4ReadonlyStorageBackend,
{
    type Error = X4ReadonlyStorageBridgeError<B::Error>;

    fn file_exists(&mut self, path: RustmixStoragePathRef<'_>) -> Result<bool, Self::Error> {
        let path = parse_storage_path(path, false)?;
        Ok(self.file_size(path).is_ok())
    }

    fn read_file_start(
        &mut self,
        path: RustmixStoragePathRef<'_>,
        out: &mut [u8],
    ) -> Result<RustmixStorageReadChunk, Self::Error> {
        let path = parse_storage_path(path, false)?;
        self.read_start(path, out)
    }

    fn read_chunk(
        &mut self,
        path: RustmixStoragePathRef<'_>,
        offset: u64,
        out: &mut [u8],
    ) -> Result<RustmixStorageReadChunk, Self::Error> {
        let path = parse_storage_path(path, false)?;
        self.read_at(path, offset, out)
    }

    fn list_directory_metadata(
        &mut self,
        path: RustmixStoragePathRef<'_>,
        out: &mut [RustmixDirectoryEntry],
    ) -> Result<usize, Self::Error> {
        let path = parse_storage_path(path, true)?;
        self.list_entries(path, out)
    }

    fn resolve_current_storage_paths(&self) -> RustmixResolvedStoragePaths<'static> {
        RustmixResolvedStoragePaths::X4_BACKED_ACTIVE_PATHS
    }
}

#[cfg(target_arch = "riscv32")]
pub struct X4X4ReadonlyStorageBackend<'a> {
    sd: &'a crate::rustmix_x4::x4_kernel::drivers::sdcard::SdStorage,
}

#[cfg(target_arch = "riscv32")]
impl<'a> X4X4ReadonlyStorageBackend<'a> {
    pub const fn new(sd: &'a crate::rustmix_x4::x4_kernel::drivers::sdcard::SdStorage) -> Self {
        Self { sd }
    }

    pub const fn sd(&self) -> &'a crate::rustmix_x4::x4_kernel::drivers::sdcard::SdStorage {
        self.sd
    }
}

#[cfg(target_arch = "riscv32")]
impl X4ReadonlyStorageBackend for X4X4ReadonlyStorageBackend<'_> {
    type Error = crate::rustmix_x4::x4_kernel::error::Error;

    fn file_size_root(&mut self, name: &str) -> Result<u32, Self::Error> {
        crate::rustmix_x4::x4_kernel::drivers::storage::file_size(self.sd, name)
    }

    fn file_size_in_dir(&mut self, dir: &str, name: &str) -> Result<u32, Self::Error> {
        crate::rustmix_x4::x4_kernel::drivers::storage::file_size_in_dir(self.sd, dir, name)
    }

    fn file_size_in_subdir(
        &mut self,
        dir: &str,
        subdir: &str,
        name: &str,
    ) -> Result<u32, Self::Error> {
        crate::rustmix_x4::x4_kernel::drivers::storage::file_size_in_subdir(
            self.sd, dir, subdir, name,
        )
    }

    fn read_file_start_root(
        &mut self,
        name: &str,
        out: &mut [u8],
    ) -> Result<(u32, usize), Self::Error> {
        crate::rustmix_x4::x4_kernel::drivers::storage::read_file_start(self.sd, name, out)
    }

    fn read_file_start_in_dir(
        &mut self,
        dir: &str,
        name: &str,
        out: &mut [u8],
    ) -> Result<(u32, usize), Self::Error> {
        crate::rustmix_x4::x4_kernel::drivers::storage::read_file_start_in_dir(
            self.sd, dir, name, out,
        )
    }

    fn read_file_start_in_subdir(
        &mut self,
        dir: &str,
        subdir: &str,
        name: &str,
        out: &mut [u8],
    ) -> Result<(u32, usize), Self::Error> {
        crate::rustmix_x4::x4_kernel::drivers::storage::read_file_start_in_subdir(
            self.sd, dir, subdir, name, out,
        )
    }

    fn read_file_chunk_root(
        &mut self,
        name: &str,
        offset: u32,
        out: &mut [u8],
    ) -> Result<usize, Self::Error> {
        crate::rustmix_x4::x4_kernel::drivers::storage::read_file_chunk(self.sd, name, offset, out)
    }

    fn read_file_chunk_in_dir(
        &mut self,
        dir: &str,
        name: &str,
        offset: u32,
        out: &mut [u8],
    ) -> Result<usize, Self::Error> {
        crate::rustmix_x4::x4_kernel::drivers::storage::read_file_chunk_in_dir(
            self.sd, dir, name, offset, out,
        )
    }

    fn read_file_chunk_in_subdir(
        &mut self,
        dir: &str,
        subdir: &str,
        name: &str,
        offset: u32,
        out: &mut [u8],
    ) -> Result<usize, Self::Error> {
        crate::rustmix_x4::x4_kernel::drivers::storage::read_file_chunk_in_subdir(
            self.sd, dir, subdir, name, offset, out,
        )
    }

    fn list_root_entries(
        &mut self,
        out: &mut [X4ReadonlyDirectoryEntry],
    ) -> Result<usize, Self::Error> {
        let mut raw = [crate::rustmix_x4::x4_kernel::drivers::storage::DirEntry::EMPTY;
            X4_READONLY_MAX_DIR_ENTRIES];
        let limit = core::cmp::min(out.len(), raw.len());
        let count = crate::rustmix_x4::x4_kernel::drivers::storage::list_root_entries(
            self.sd,
            &mut raw[..limit],
        )?;
        copy_x4_entries(&raw[..core::cmp::min(count, limit)], out)
    }

    fn list_dir_entries(
        &mut self,
        dir: &str,
        out: &mut [X4ReadonlyDirectoryEntry],
    ) -> Result<usize, Self::Error> {
        let mut raw = [crate::rustmix_x4::x4_kernel::drivers::storage::DirEntry::EMPTY;
            X4_READONLY_MAX_DIR_ENTRIES];
        let limit = core::cmp::min(out.len(), raw.len());
        let count = crate::rustmix_x4::x4_kernel::drivers::storage::list_dir_entries(
            self.sd,
            dir,
            &mut raw[..limit],
        )?;
        copy_x4_entries(&raw[..core::cmp::min(count, limit)], out)
    }

    fn list_subdir_entries(
        &mut self,
        dir: &str,
        subdir: &str,
        out: &mut [X4ReadonlyDirectoryEntry],
    ) -> Result<usize, Self::Error> {
        let mut raw = [crate::rustmix_x4::x4_kernel::drivers::storage::DirEntry::EMPTY;
            X4_READONLY_MAX_DIR_ENTRIES];
        let limit = core::cmp::min(out.len(), raw.len());
        let count = crate::rustmix_x4::x4_kernel::drivers::storage::list_subdir_entries(
            self.sd,
            dir,
            subdir,
            &mut raw[..limit],
        )?;
        copy_x4_entries(&raw[..core::cmp::min(count, limit)], out)
    }
}

#[cfg(target_arch = "riscv32")]
fn copy_x4_entries(
    raw: &[crate::rustmix_x4::x4_kernel::drivers::storage::DirEntry],
    out: &mut [X4ReadonlyDirectoryEntry],
) -> Result<usize, crate::rustmix_x4::x4_kernel::error::Error> {
    let mut count = 0usize;
    while count < raw.len() && count < out.len() {
        let src = raw[count];
        let name = &src.name[..src.name_len as usize];
        if let Some(entry) = X4ReadonlyDirectoryEntry::from_name(name, src.is_dir, src.size as u64)
        {
            out[count] = entry;
            count += 1;
        } else {
            break;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::{
        RustmixStorageReadonlyX4BridgeContract, X4ReadonlyDirectoryEntry, X4ReadonlyStorageBackend,
        X4ReadonlyStorageBridge,
    };
    use crate::rustmix_x4::io::storage_readonly_adapter::{
        RustmixDirectoryEntry, RustmixReadonlyStorage, RustmixStoragePathRef,
    };

    #[derive(Default)]
    struct FakeX4Backend {
        last_dir: &'static str,
        last_subdir: &'static str,
        last_name: &'static str,
        last_offset: u32,
    }

    impl X4ReadonlyStorageBackend for FakeX4Backend {
        type Error = ();

        fn file_size_root(&mut self, name: &str) -> Result<u32, Self::Error> {
            self.last_name = if name == "BOOK.TXT" {
                "BOOK.TXT"
            } else {
                "missing"
            };
            if name == "BOOK.TXT" { Ok(8) } else { Err(()) }
        }

        fn file_size_in_dir(&mut self, dir: &str, name: &str) -> Result<u32, Self::Error> {
            self.last_dir = if dir == "state" { "state" } else { "other" };
            self.last_name = if name == "BMIDX.TXT" {
                "BMIDX.TXT"
            } else {
                "missing"
            };
            if dir == "state" && name == "BMIDX.TXT" {
                Ok(4)
            } else {
                Err(())
            }
        }

        fn file_size_in_subdir(
            &mut self,
            dir: &str,
            subdir: &str,
            name: &str,
        ) -> Result<u32, Self::Error> {
            self.last_dir = if dir == "sleep" { "sleep" } else { "other" };
            self.last_subdir = if subdir == "daily" { "daily" } else { "other" };
            self.last_name = if name == "MON.BMP" {
                "MON.BMP"
            } else {
                "missing"
            };
            if dir == "sleep" && subdir == "daily" && name == "MON.BMP" {
                Ok(9)
            } else {
                Err(())
            }
        }

        fn read_file_start_root(
            &mut self,
            name: &str,
            out: &mut [u8],
        ) -> Result<(u32, usize), Self::Error> {
            self.file_size_root(name)?;
            out[..4].copy_from_slice(b"book");
            Ok((8, 4))
        }

        fn read_file_start_in_dir(
            &mut self,
            dir: &str,
            name: &str,
            out: &mut [u8],
        ) -> Result<(u32, usize), Self::Error> {
            self.file_size_in_dir(dir, name)?;
            out[..4].copy_from_slice(b"bmid");
            Ok((4, 4))
        }

        fn read_file_start_in_subdir(
            &mut self,
            dir: &str,
            subdir: &str,
            name: &str,
            out: &mut [u8],
        ) -> Result<(u32, usize), Self::Error> {
            self.file_size_in_subdir(dir, subdir, name)?;
            out[..3].copy_from_slice(b"bmp");
            Ok((9, 3))
        }

        fn read_file_chunk_root(
            &mut self,
            name: &str,
            offset: u32,
            out: &mut [u8],
        ) -> Result<usize, Self::Error> {
            self.file_size_root(name)?;
            self.last_offset = offset;
            out[0] = b'R';
            Ok(1)
        }

        fn read_file_chunk_in_dir(
            &mut self,
            dir: &str,
            name: &str,
            offset: u32,
            out: &mut [u8],
        ) -> Result<usize, Self::Error> {
            self.file_size_in_dir(dir, name)?;
            self.last_offset = offset;
            out[0] = b'D';
            Ok(1)
        }

        fn read_file_chunk_in_subdir(
            &mut self,
            dir: &str,
            subdir: &str,
            name: &str,
            offset: u32,
            out: &mut [u8],
        ) -> Result<usize, Self::Error> {
            self.file_size_in_subdir(dir, subdir, name)?;
            self.last_offset = offset;
            out[0] = b'S';
            Ok(1)
        }

        fn list_root_entries(
            &mut self,
            out: &mut [X4ReadonlyDirectoryEntry],
        ) -> Result<usize, Self::Error> {
            out[0] = X4ReadonlyDirectoryEntry::from_name(b"BOOK.TXT", false, 8).unwrap();
            Ok(1)
        }

        fn list_dir_entries(
            &mut self,
            dir: &str,
            out: &mut [X4ReadonlyDirectoryEntry],
        ) -> Result<usize, Self::Error> {
            self.last_dir = if dir == "state" { "state" } else { "other" };
            out[0] = X4ReadonlyDirectoryEntry::from_name(b"BMIDX.TXT", false, 4).unwrap();
            Ok(1)
        }

        fn list_subdir_entries(
            &mut self,
            dir: &str,
            subdir: &str,
            out: &mut [X4ReadonlyDirectoryEntry],
        ) -> Result<usize, Self::Error> {
            self.last_dir = if dir == "sleep" { "sleep" } else { "other" };
            self.last_subdir = if subdir == "daily" { "daily" } else { "other" };
            out[0] = X4ReadonlyDirectoryEntry::from_name(b"MON.BMP", false, 9).unwrap();
            Ok(1)
        }
    }

    #[test]
    fn bridge_maps_root_file_reads_to_backend() {
        let mut bridge = X4ReadonlyStorageBridge::new(FakeX4Backend::default());
        let path = RustmixStoragePathRef::from_str("/BOOK.TXT").unwrap();
        let mut out = [0u8; 8];

        assert_eq!(bridge.file_exists(path), Ok(true));
        let chunk = bridge.read_file_start(path, &mut out).unwrap();
        assert_eq!(chunk.bytes_read, 4);
        assert_eq!(&out[..4], b"book");

        let chunk = bridge.read_chunk(path, 2, &mut out).unwrap();
        assert_eq!(chunk.offset, 2);
        assert_eq!(bridge.backend().last_offset, 2);
    }

    #[test]
    fn bridge_maps_directory_and_subdirectory_paths() {
        let mut bridge = X4ReadonlyStorageBridge::new(FakeX4Backend::default());
        let path = RustmixStoragePathRef::from_str("/sleep/daily/MON.BMP").unwrap();
        let mut out = [0u8; 8];

        let chunk = bridge.read_file_start(path, &mut out).unwrap();
        assert_eq!(chunk.bytes_read, 3);
        assert_eq!(bridge.backend().last_dir, "sleep");
        assert_eq!(bridge.backend().last_subdir, "daily");
    }

    #[test]
    fn bridge_lists_metadata_without_mutating_contracts() {
        let mut bridge = X4ReadonlyStorageBridge::new(FakeX4Backend::default());
        let mut out = [RustmixDirectoryEntry::empty(); 2];

        assert_eq!(
            bridge.list_directory_metadata(RustmixStoragePathRef::from_str("/").unwrap(), &mut out),
            Ok(1)
        );
        assert_eq!(out[0].name_bytes(), b"BOOK.TXT");
    }

    #[test]
    fn bridge_preflight_keeps_physical_behavior_imported() {
        let report = RustmixStorageReadonlyX4BridgeContract::active_runtime_preflight();
        assert!(report.ok());
        assert!(!RustmixStorageReadonlyX4BridgeContract::physical_behavior_moved());
    }
}
