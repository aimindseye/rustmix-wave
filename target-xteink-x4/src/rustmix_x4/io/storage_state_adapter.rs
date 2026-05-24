#![allow(dead_code)]

use super::storage_state::{
    RustmixStateIoKind, RustmixStorageStateIo, RustmixStorageStateIoError, RustmixStorageStatePaths,
};
use crate::rustmix_x4::contracts::storage_path_helpers::RustmixStatePath;

pub trait RustmixStorageStatePathIo {
    type Error;

    fn read_state_path(
        &mut self,
        path: &RustmixStatePath,
        out: &mut [u8],
    ) -> Result<usize, Self::Error>;

    fn write_state_path(&mut self, path: &RustmixStatePath, data: &[u8])
    -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixStorageStateAdapterError<E> {
    Contract(RustmixStorageStateIoError),
    Backend(E),
}

pub struct RustmixStorageStateIoAdapter<B> {
    backend: B,
}

impl<B> RustmixStorageStateIoAdapter<B> {
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

impl<B> RustmixStorageStateIo for RustmixStorageStateIoAdapter<B>
where
    B: RustmixStorageStatePathIo,
{
    type Error = RustmixStorageStateAdapterError<B::Error>;

    fn read_state(
        &mut self,
        book_id: &[u8],
        kind: RustmixStateIoKind,
        out: &mut [u8],
    ) -> Result<usize, Self::Error> {
        let path =
            RustmixStorageStatePaths::state_path(book_id, kind).map_err(Self::Error::Contract)?;
        self.backend
            .read_state_path(&path, out)
            .map_err(Self::Error::Backend)
    }

    fn write_state(
        &mut self,
        book_id: &[u8],
        kind: RustmixStateIoKind,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        let path =
            RustmixStorageStatePaths::state_path(book_id, kind).map_err(Self::Error::Contract)?;
        self.backend
            .write_state_path(&path, data)
            .map_err(Self::Error::Backend)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RustmixStorageStateAdapterError, RustmixStorageStateIoAdapter, RustmixStorageStatePathIo,
    };
    use crate::rustmix_x4::contracts::storage_path_helpers::RustmixStatePath;
    use crate::rustmix_x4::io::storage_state::{
        RustmixStateIoKind, RustmixStorageStateIo, RustmixStorageStateIoError,
    };

    struct ProbeBackend {
        last_path: RustmixStatePath,
    }

    impl Default for ProbeBackend {
        fn default() -> Self {
            Self {
                last_path: RustmixStatePath::empty(),
            }
        }
    }

    impl RustmixStorageStatePathIo for ProbeBackend {
        type Error = ();

        fn read_state_path(
            &mut self,
            path: &RustmixStatePath,
            _out: &mut [u8],
        ) -> Result<usize, Self::Error> {
            self.last_path = *path;
            Ok(0)
        }

        fn write_state_path(
            &mut self,
            path: &RustmixStatePath,
            _data: &[u8],
        ) -> Result<(), Self::Error> {
            self.last_path = *path;
            Ok(())
        }
    }

    #[test]
    fn adapter_resolves_semantic_kind_before_delegating() {
        let mut adapter = RustmixStorageStateIoAdapter::new(ProbeBackend::default());
        let mut out = [];

        assert_eq!(
            adapter.read_state(b"8A79A61F", RustmixStateIoKind::Bookmark, &mut out),
            Ok(0)
        );
        assert_eq!(
            adapter.backend().last_path.as_bytes(),
            b"state/8A79A61F.BKM"
        );
    }

    #[test]
    fn adapter_rejects_invalid_book_id_before_backend() {
        let mut adapter = RustmixStorageStateIoAdapter::new(ProbeBackend::default());
        let mut out = [];

        assert_eq!(
            adapter.read_state(b"8a79a61f", RustmixStateIoKind::Progress, &mut out),
            Err(RustmixStorageStateAdapterError::Contract(
                RustmixStorageStateIoError::InvalidBookId
            ))
        );
        assert!(adapter.backend().last_path.is_empty());
    }
}
