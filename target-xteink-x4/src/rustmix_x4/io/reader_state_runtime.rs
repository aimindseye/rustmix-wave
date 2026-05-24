#![allow(dead_code)]

extern crate alloc;

use alloc::string::{String, ToString};

use crate::rustmix_x4::apps::reader_state::{
    RustmixBookId, RustmixBookIdentity, RustmixBookMetaRecord, RustmixBookmarkIndexRecord,
    RustmixBookmarkRecord, RustmixReaderStateLayout, RustmixReaderThemePreset,
    RustmixReaderThemeRecord, RustmixReadingProgressRecord, decode_bookmark_jump, decode_bookmarks,
    decode_bookmarks_index, encode_bookmarks, encode_bookmarks_index,
};

pub struct RustmixReaderStateRuntimeBridge;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixReaderStateRuntimePreflightReport {
    pub progress_record_ok: bool,
    pub bookmark_record_ok: bool,
    pub bookmark_index_ok: bool,
    pub theme_record_ok: bool,
    pub metadata_record_ok: bool,
    pub layout_paths_ok: bool,
    pub physical_storage_io_owned: bool,
    pub active_persistence_owned: bool,
}

impl RustmixReaderStateRuntimePreflightReport {
    pub const fn preflight_ok(self) -> bool {
        self.progress_record_ok
            && self.bookmark_record_ok
            && self.bookmark_index_ok
            && self.theme_record_ok
            && self.metadata_record_ok
            && self.layout_paths_ok
            && !self.physical_storage_io_owned
            && !self.active_persistence_owned
    }
}

impl RustmixReaderStateRuntimeBridge {
    pub const PHYSICAL_STORAGE_IO_OWNED_BY_BRIDGED1: bool = false;
    pub const ACTIVE_PERSISTENCE_OWNED_BY_BRIDGE: bool = false;
    pub const REQUIRES_HEAP_ALLOCATOR: bool = true;

    pub fn active_runtime_preflight() -> bool {
        Self::preflight_report().preflight_ok()
    }

    pub fn preflight_report() -> RustmixReaderStateRuntimePreflightReport {
        let identity = RustmixBookIdentity::from_path("/books/Reader State Bridge.epub")
            .with_display_title("Reader State");

        RustmixReaderStateRuntimePreflightReport {
            progress_record_ok: Self::progress_record_round_trips(&identity),
            bookmark_record_ok: Self::bookmark_record_round_trips(&identity),
            bookmark_index_ok: Self::bookmark_index_round_trips(&identity),
            theme_record_ok: Self::theme_record_round_trips(&identity),
            metadata_record_ok: Self::metadata_record_round_trips(&identity),
            layout_paths_ok: Self::layout_paths_match(&identity.book_id),
            physical_storage_io_owned: Self::PHYSICAL_STORAGE_IO_OWNED_BY_BRIDGED1,
            active_persistence_owned: Self::ACTIVE_PERSISTENCE_OWNED_BY_BRIDGE,
        }
    }

    fn progress_record_round_trips(identity: &RustmixBookIdentity) -> bool {
        let record = RustmixReadingProgressRecord::from_identity(identity, 2, 13, 4096, 5);
        RustmixReadingProgressRecord::decode_line(&record.encode_line())
            .is_some_and(|decoded| decoded == record)
    }

    fn bookmark_record_round_trips(identity: &RustmixBookIdentity) -> bool {
        let first = RustmixBookmarkRecord::new(identity, 1, 128, "first|mark".to_string());
        let second = RustmixBookmarkRecord::new(identity, 2, 256, String::new());
        let encoded = encode_bookmarks(&[first.clone(), second.clone()]);
        let decoded = decode_bookmarks(&encoded);
        decoded.len() == 2 && decoded[0] == first && decoded[1] == second
    }

    fn bookmark_index_round_trips(identity: &RustmixBookIdentity) -> bool {
        let bookmark = RustmixBookmarkRecord::new(identity, 3, 2048, "chapter start".to_string());
        let entry = RustmixBookmarkIndexRecord::from_bookmark(&bookmark, &identity.display_title);
        let encoded = encode_bookmarks_index(core::slice::from_ref(&entry));
        let decoded = decode_bookmarks_index(&encoded);
        decoded.len() == 1
            && decoded[0] == entry
            && decode_bookmark_jump(&entry.jump_message()).is_some_and(|(path, chapter, offset)| {
                path == identity.source_path && chapter == 3 && offset == 2048
            })
    }

    fn theme_record_round_trips(identity: &RustmixBookIdentity) -> bool {
        let preset = RustmixReaderThemePreset {
            font_size_idx: 5,
            margin_px: 8,
            line_spacing_pct: 100,
            alignment: "justify".to_string(),
            theme_name: "classic".to_string(),
        };
        let record = RustmixReaderThemeRecord::from_identity(identity, preset);
        RustmixReaderThemeRecord::decode_line(&record.encode_line())
            .is_some_and(|decoded| decoded == record)
    }

    fn metadata_record_round_trips(identity: &RustmixBookIdentity) -> bool {
        let record = RustmixBookMetaRecord::from_identity(identity);
        RustmixBookMetaRecord::decode_line(&record.encode_line())
            .is_some_and(|decoded| decoded == record && decoded.display_title == "Reader State")
    }

    fn layout_paths_match(book_id: &RustmixBookId) -> bool {
        RustmixReaderStateLayout::for_book_id(book_id).is_some_and(|layout| {
            layout.state_dir == "state"
                && layout.progress_file.ends_with(".PRG")
                && layout.bookmark_file.ends_with(".BKM")
                && layout.theme_file.ends_with(".THM")
                && layout.meta_file.ends_with(".MTA")
                && layout.bookmarks_index_file == "BMIDX.TXT"
        })
    }
}

#[cfg(test)]
mod tests {
    use super::RustmixReaderStateRuntimeBridge;

    #[test]
    fn active_runtime_preflight_is_format_only_and_valid() {
        assert!(RustmixReaderStateRuntimeBridge::active_runtime_preflight());
    }
}
