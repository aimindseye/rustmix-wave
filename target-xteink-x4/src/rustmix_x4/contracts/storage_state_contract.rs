#![allow(dead_code)]

use crate::rustmix_x4::contracts::storage_path_helpers::{
    RustmixStateFileKind as HelperStateFileKind, RustmixStoragePathHelpers,
};

/// Rustmix-owned storage state contract smoke.
///
/// This module makes state-file naming and reserved-file expectations explicit
/// without moving SD/SPI/FAT/EPUB-cache behavior out of the imported X4
/// runtime yet.
pub struct RustmixStorageStateContract;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixStateRecordKind {
    Progress,
    Bookmark,
    Theme,
    Metadata,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixStateContractError {
    InvalidBookId,
    InvalidExtension,
}

impl RustmixStorageStateContract {
    pub const STORAGE_CONTRACT_SMOKE_MARKER: &'static str = "x4-storage-contract-smoke-ok";

    pub const STATE_DIR: &'static str = RustmixStoragePathHelpers::STATE_DIR_STR;
    pub const BOOK_ID_LEN: usize = RustmixStoragePathHelpers::BOOK_ID_LEN;
    pub const STATE_FILE_NAME_LEN: usize = RustmixStoragePathHelpers::STATE_FILE_NAME_LEN;

    pub const PROGRESS_EXT: &'static str = RustmixStoragePathHelpers::PROGRESS_EXTENSION;
    pub const BOOKMARK_EXT: &'static str = RustmixStoragePathHelpers::BOOKMARK_EXTENSION;
    pub const THEME_EXT: &'static str = RustmixStoragePathHelpers::THEME_EXTENSION;
    pub const METADATA_EXT: &'static str = RustmixStoragePathHelpers::METADATA_EXTENSION;

    pub const BOOKMARK_INDEX_FILE: &'static str = RustmixStoragePathHelpers::BOOKMARK_INDEX_FILE;
    pub const EPUB_CACHE_OWNER: &'static str = "Rustmix-owned X4 runtime";
    pub const PHYSICAL_SD_IO_MOVED_TO_BOUNDARY: bool = false;
    pub const EPUB_CACHE_IO_MOVED_TO_BOUNDARY: bool = false;

    pub const fn extension_for(kind: RustmixStateRecordKind) -> &'static str {
        match kind {
            RustmixStateRecordKind::Progress => Self::PROGRESS_EXT,
            RustmixStateRecordKind::Bookmark => Self::BOOKMARK_EXT,
            RustmixStateRecordKind::Theme => Self::THEME_EXT,
            RustmixStateRecordKind::Metadata => Self::METADATA_EXT,
        }
    }

    pub fn is_known_state_extension(ext: &str) -> bool {
        RustmixStoragePathHelpers::is_supported_state_extension(ext.as_bytes())
    }

    pub fn is_reserved_state_file(name: &str) -> bool {
        RustmixStoragePathHelpers::is_reserved_state_file(name.as_bytes())
    }

    pub fn is_valid_book_id(book_id: &str) -> bool {
        RustmixStoragePathHelpers::is_valid_upper_book_id(book_id.as_bytes())
    }

    pub fn is_valid_state_file_name(name: &str) -> bool {
        if Self::is_reserved_state_file(name) {
            return true;
        }

        let bytes = name.as_bytes();
        if bytes.len() != Self::STATE_FILE_NAME_LEN || bytes[Self::BOOK_ID_LEN] != b'.' {
            return false;
        }

        let Some(book_id) = core::str::from_utf8(&bytes[..Self::BOOK_ID_LEN]).ok() else {
            return false;
        };
        let Some(ext) = core::str::from_utf8(&bytes[Self::BOOK_ID_LEN + 1..]).ok() else {
            return false;
        };

        Self::is_valid_book_id(book_id) && Self::is_known_state_extension(ext)
    }

    pub fn state_file_name(
        book_id: &str,
        kind: RustmixStateRecordKind,
    ) -> Result<[u8; Self::STATE_FILE_NAME_LEN], RustmixStateContractError> {
        if !Self::is_valid_book_id(book_id) {
            return Err(RustmixStateContractError::InvalidBookId);
        }

        let ext = Self::extension_for(kind);
        if !Self::is_known_state_extension(ext) {
            return Err(RustmixStateContractError::InvalidExtension);
        }

        let helper_kind = match kind {
            RustmixStateRecordKind::Progress => HelperStateFileKind::Progress,
            RustmixStateRecordKind::Bookmark => HelperStateFileKind::Bookmark,
            RustmixStateRecordKind::Theme => HelperStateFileKind::Theme,
            RustmixStateRecordKind::Metadata => HelperStateFileKind::Metadata,
        };
        let Some(path) = RustmixStoragePathHelpers::state_file_name_from_str(book_id, helper_kind)
        else {
            return Err(RustmixStateContractError::InvalidBookId);
        };

        let mut out = [0u8; Self::STATE_FILE_NAME_LEN];
        out.copy_from_slice(path.as_bytes());
        Ok(out)
    }

    pub fn smoke_validate_contract() -> bool {
        Self::is_valid_book_id("8A79A61F")
            && !Self::is_valid_book_id("8a79a61f")
            && Self::is_known_state_extension(Self::PROGRESS_EXT)
            && Self::is_known_state_extension(Self::BOOKMARK_EXT)
            && Self::is_known_state_extension(Self::THEME_EXT)
            && Self::is_known_state_extension(Self::METADATA_EXT)
            && Self::is_reserved_state_file(Self::BOOKMARK_INDEX_FILE)
            && Self::is_valid_state_file_name("8A79A61F.PRG")
            && Self::is_valid_state_file_name("8A79A61F.BKM")
            && Self::is_valid_state_file_name("8A79A61F.THM")
            && Self::is_valid_state_file_name("8A79A61F.MTA")
            && Self::is_valid_state_file_name(Self::BOOKMARK_INDEX_FILE)
            && !Self::PHYSICAL_SD_IO_MOVED_TO_BOUNDARY
            && !Self::EPUB_CACHE_IO_MOVED_TO_BOUNDARY
    }

    pub fn emit_contract_marker() {
        if Self::smoke_validate_contract() {
            esp_println::println!("{}", Self::STORAGE_CONTRACT_SMOKE_MARKER);
        } else {
            esp_println::println!("storage-contract-smoke-failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RustmixStateRecordKind, RustmixStorageStateContract};

    #[test]
    fn validates_book_ids_and_state_names() {
        assert!(RustmixStorageStateContract::is_valid_book_id("8A79A61F"));
        assert!(!RustmixStorageStateContract::is_valid_book_id("8a79a61f"));
        assert!(RustmixStorageStateContract::is_valid_state_file_name(
            "8A79A61F.PRG"
        ));
        assert!(RustmixStorageStateContract::is_valid_state_file_name(
            "8A79A61F.BKM"
        ));
        assert!(RustmixStorageStateContract::is_valid_state_file_name(
            "8A79A61F.THM"
        ));
        assert!(RustmixStorageStateContract::is_valid_state_file_name(
            "8A79A61F.MTA"
        ));
        assert!(RustmixStorageStateContract::is_valid_state_file_name(
            "BMIDX.TXT"
        ));
        assert!(!RustmixStorageStateContract::is_valid_state_file_name(
            "8A79A61F.EPC"
        ));
    }

    #[test]
    fn builds_8_3_state_names() {
        let name = RustmixStorageStateContract::state_file_name(
            "8A79A61F",
            RustmixStateRecordKind::Progress,
        )
        .unwrap();
        assert_eq!(&name, b"8A79A61F.PRG");
    }
}
