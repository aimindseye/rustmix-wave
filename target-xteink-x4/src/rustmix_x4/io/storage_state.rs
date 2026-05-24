#![allow(dead_code)]

use crate::rustmix_x4::contracts::storage_path_helpers::{
    RustmixStateFileKind, RustmixStatePath, RustmixStoragePathHelpers,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixStateIoKind {
    Progress,
    Bookmark,
    Theme,
    Metadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixStorageStateIoError {
    InvalidBookId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixStorageStateIoSeamReport {
    pub progress_kind_ok: bool,
    pub bookmark_kind_ok: bool,
    pub theme_kind_ok: bool,
    pub metadata_kind_ok: bool,
    pub path_helpers_used: bool,
    pub physical_storage_io_owned: bool,
    pub reader_cache_io_owned: bool,
}

impl RustmixStorageStateIoSeamReport {
    pub const fn seam_ok(self) -> bool {
        self.progress_kind_ok
            && self.bookmark_kind_ok
            && self.theme_kind_ok
            && self.metadata_kind_ok
            && self.path_helpers_used
            && !self.physical_storage_io_owned
            && !self.reader_cache_io_owned
    }
}

pub trait RustmixStorageStateIo {
    type Error;

    fn read_state(
        &mut self,
        book_id: &[u8],
        kind: RustmixStateIoKind,
        out: &mut [u8],
    ) -> Result<usize, Self::Error>;

    fn write_state(
        &mut self,
        book_id: &[u8],
        kind: RustmixStateIoKind,
        data: &[u8],
    ) -> Result<(), Self::Error>;
}

pub struct RustmixStorageStatePaths;

impl RustmixStateIoKind {
    pub const fn as_file_kind(self) -> RustmixStateFileKind {
        match self {
            Self::Progress => RustmixStateFileKind::Progress,
            Self::Bookmark => RustmixStateFileKind::Bookmark,
            Self::Theme => RustmixStateFileKind::Theme,
            Self::Metadata => RustmixStateFileKind::Metadata,
        }
    }
}

impl RustmixStorageStatePaths {
    pub const IMPLEMENTATION_OWNER: &'static str = "Rustmix-owned storage state IO seam";
    pub const PHYSICAL_STORAGE_IO_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const READER_CACHE_IO_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const PHYSICAL_STORAGE_IO_OWNED_BY_BRIDGE: bool = false;
    pub const READER_CACHE_IO_OWNED_BY_BRIDGE: bool = false;

    pub fn state_path(
        book_id: &[u8],
        kind: RustmixStateIoKind,
    ) -> Result<RustmixStatePath, RustmixStorageStateIoError> {
        if !RustmixStoragePathHelpers::is_valid_upper_book_id(book_id) {
            return Err(RustmixStorageStateIoError::InvalidBookId);
        }

        let mut normalized = [0u8; RustmixStoragePathHelpers::BOOK_ID_LEN];
        normalized.copy_from_slice(book_id);

        Ok(RustmixStoragePathHelpers::state_path(
            normalized,
            kind.as_file_kind(),
        ))
    }

    pub fn seam_report() -> RustmixStorageStateIoSeamReport {
        let book_id = b"8A79A61F";

        RustmixStorageStateIoSeamReport {
            progress_kind_ok: Self::state_path(book_id, RustmixStateIoKind::Progress)
                .is_ok_and(|path| path.as_bytes() == b"state/8A79A61F.PRG"),
            bookmark_kind_ok: Self::state_path(book_id, RustmixStateIoKind::Bookmark)
                .is_ok_and(|path| path.as_bytes() == b"state/8A79A61F.BKM"),
            theme_kind_ok: Self::state_path(book_id, RustmixStateIoKind::Theme)
                .is_ok_and(|path| path.as_bytes() == b"state/8A79A61F.THM"),
            metadata_kind_ok: Self::state_path(book_id, RustmixStateIoKind::Metadata)
                .is_ok_and(|path| path.as_bytes() == b"state/8A79A61F.MTA"),
            path_helpers_used: RustmixStoragePathHelpers::active_runtime_adoption_probe(),
            physical_storage_io_owned: Self::PHYSICAL_STORAGE_IO_OWNED_BY_BRIDGE,
            reader_cache_io_owned: Self::READER_CACHE_IO_OWNED_BY_BRIDGE,
        }
    }

    pub fn seam_ok() -> bool {
        Self::seam_report().seam_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{RustmixStateIoKind, RustmixStorageStateIoError, RustmixStorageStatePaths};

    #[test]
    fn resolves_state_kinds_through_path_helpers() {
        let book_id = b"8A79A61F";

        assert_eq!(
            RustmixStorageStatePaths::state_path(book_id, RustmixStateIoKind::Progress)
                .unwrap()
                .as_bytes(),
            b"state/8A79A61F.PRG"
        );
        assert_eq!(
            RustmixStorageStatePaths::state_path(book_id, RustmixStateIoKind::Bookmark)
                .unwrap()
                .as_bytes(),
            b"state/8A79A61F.BKM"
        );
        assert_eq!(
            RustmixStorageStatePaths::state_path(book_id, RustmixStateIoKind::Theme)
                .unwrap()
                .as_bytes(),
            b"state/8A79A61F.THM"
        );
        assert_eq!(
            RustmixStorageStatePaths::state_path(book_id, RustmixStateIoKind::Metadata)
                .unwrap()
                .as_bytes(),
            b"state/8A79A61F.MTA"
        );
    }

    #[test]
    fn rejects_non_contract_book_ids() {
        assert_eq!(
            RustmixStorageStatePaths::state_path(b"8a79a61f", RustmixStateIoKind::Progress),
            Err(RustmixStorageStateIoError::InvalidBookId)
        );
        assert_eq!(
            RustmixStorageStatePaths::state_path(b"TOO-SHORT", RustmixStateIoKind::Progress),
            Err(RustmixStorageStateIoError::InvalidBookId)
        );
    }

    #[test]
    fn seam_probe_does_not_claim_physical_io() {
        assert!(RustmixStorageStatePaths::seam_ok());
    }
}
