use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenMode {
    Read,
    Write,
    ReadWrite,
    Create,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: heapless::String<64>,
    pub is_dir: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageState {
    Missing,
    Probed,
    Mounted,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageProbe {
    pub state: StorageState,
    pub card_size_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageError {
    NotFound,
    Io,
    NotReady,
    Unsupported,
    BufferTooSmall,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::NotFound => f.write_str("not found"),
            StorageError::Io => f.write_str("i/o error"),
            StorageError::NotReady => f.write_str("not ready"),
            StorageError::Unsupported => f.write_str("unsupported"),
            StorageError::BufferTooSmall => f.write_str("buffer too small"),
        }
    }
}

pub trait StorageHal {
    type File;
    type Dir;

    /// Current storage lifecycle state.
    fn state(&self) -> StorageState;

    /// Probe / initialise the SD card at the device-required low-speed bus mode.
    fn init_card(&mut self) -> Result<StorageProbe, StorageError>;

    /// Mount the filesystem after card probe succeeded.
    fn mount(&mut self) -> Result<(), StorageError>;

    /// Flush writes and close handles before sleep / halt.
    fn flush_and_close(&mut self) -> Result<(), StorageError>;

    fn open(&mut self, path: &str, mode: OpenMode) -> Result<Self::File, StorageError>;
    fn open_dir(&mut self, path: &str) -> Result<Self::Dir, StorageError>;
    fn mkdir(&mut self, path: &str) -> Result<(), StorageError>;
    fn remove(&mut self, path: &str) -> Result<(), StorageError>;
    fn exists(&self, path: &str) -> bool;

    /// Bootstrap Bootstrap byte-store helper.
    ///
    /// Implementations may keep this as `Unsupported` until a real filesystem
    /// adapter is extracted. Models/services can still target this seam.
    fn read_file(&mut self, _path: &str, _buf: &mut [u8]) -> Result<usize, StorageError> {
        Err(StorageError::Unsupported)
    }

    /// Bootstrap Bootstrap byte-store helper.
    fn write_file(&mut self, _path: &str, _bytes: &[u8]) -> Result<(), StorageError> {
        Err(StorageError::Unsupported)
    }

    /// Best-effort recursive directory creation seam.
    fn mkdir_all(&mut self, path: &str) -> Result<(), StorageError> {
        self.mkdir(path)
    }

    fn nvs_get(&self, key: &str, buf: &mut [u8]) -> Result<usize, StorageError>;
    fn nvs_set(&mut self, key: &str, val: &[u8]) -> Result<(), StorageError>;
}
