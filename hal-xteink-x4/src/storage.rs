use rustmix_core::hal::{OpenMode, StorageError, StorageHal, StorageProbe, StorageState};

pub const X4_PLACEHOLDER_CARD_SIZE_BYTES: u64 = 15_634_268_160;

pub struct X4Storage {
    state: StorageState,
    card_size_bytes: Option<u64>,
}

pub struct X4File;
pub struct X4Dir;

impl X4Storage {
    pub fn card_size_bytes(&self) -> Option<u64> {
        self.card_size_bytes
    }
}

impl StorageHal for X4Storage {
    type File = X4File;
    type Dir = X4Dir;

    fn state(&self) -> StorageState {
        self.state
    }

    fn init_card(&mut self) -> Result<StorageProbe, StorageError> {
        self.state = StorageState::Probed;
        self.card_size_bytes = Some(X4_PLACEHOLDER_CARD_SIZE_BYTES);
        Ok(StorageProbe {
            state: self.state,
            card_size_bytes: self.card_size_bytes,
        })
    }

    fn mount(&mut self) -> Result<(), StorageError> {
        if !matches!(self.state, StorageState::Probed | StorageState::Mounted) {
            return Err(StorageError::NotReady);
        }
        self.state = StorageState::Mounted;
        Ok(())
    }

    fn flush_and_close(&mut self) -> Result<(), StorageError> {
        if matches!(self.state, StorageState::Mounted | StorageState::Probed) {
            self.state = StorageState::Closed;
            Ok(())
        } else {
            Err(StorageError::NotReady)
        }
    }

    fn open(&mut self, _path: &str, _mode: OpenMode) -> Result<Self::File, StorageError> {
        Err(StorageError::Unsupported)
    }

    fn open_dir(&mut self, _path: &str) -> Result<Self::Dir, StorageError> {
        Err(StorageError::Unsupported)
    }

    fn mkdir(&mut self, _path: &str) -> Result<(), StorageError> {
        Err(StorageError::Unsupported)
    }

    fn remove(&mut self, _path: &str) -> Result<(), StorageError> {
        Err(StorageError::Unsupported)
    }

    fn exists(&self, _path: &str) -> bool {
        false
    }

    fn nvs_get(&self, _key: &str, _buf: &mut [u8]) -> Result<usize, StorageError> {
        Err(StorageError::Unsupported)
    }

    fn nvs_set(&mut self, _key: &str, _val: &[u8]) -> Result<(), StorageError> {
        Err(StorageError::Unsupported)
    }
}

impl Default for X4Storage {
    fn default() -> Self {
        Self {
            state: StorageState::Missing,
            card_size_bytes: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustmix_core::hal::StorageHal;

    #[test]
    fn storage_lifecycle_matches_bootstrap_order() {
        let mut storage = X4Storage::default();
        assert_eq!(storage.state(), StorageState::Missing);

        let probe = storage.init_card().unwrap();
        assert_eq!(probe.state, StorageState::Probed);
        assert_eq!(probe.card_size_bytes, Some(X4_PLACEHOLDER_CARD_SIZE_BYTES));

        storage.mount().unwrap();
        assert_eq!(storage.state(), StorageState::Mounted);

        storage.flush_and_close().unwrap();
        assert_eq!(storage.state(), StorageState::Closed);
    }

    #[test]
    fn byte_store_helpers_are_not_claimed_until_real_fs_adapter_exists() {
        let mut storage = X4Storage::default();
        let mut buf = [0u8; 8];
        assert_eq!(
            storage.read_file("state/TEST.PRG", &mut buf),
            Err(StorageError::Unsupported)
        );
        assert_eq!(
            storage.write_file("state/TEST.PRG", b"data"),
            Err(StorageError::Unsupported)
        );
    }
}
