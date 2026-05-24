use heapless::String;
use serde::{Deserialize, Serialize};

use super::BookId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageLayoutKind {
    /// Long-term Rustmix layout.
    CanonicalRustmix,
    /// Proven X4-safe flat 8.3-style state files.
    X4CompatFlat83,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderStoragePaths {
    pub meta: String<128>,
    pub progress: String<128>,
    pub bookmarks: String<128>,
    pub theme: String<128>,
    pub sections_dir: String<128>,
    pub bookmark_index: String<128>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageLayout {
    pub kind: StorageLayoutKind,
}

impl StorageLayout {
    pub const fn canonical() -> Self {
        Self {
            kind: StorageLayoutKind::CanonicalRustmix,
        }
    }

    pub const fn x4_compat() -> Self {
        Self {
            kind: StorageLayoutKind::X4CompatFlat83,
        }
    }

    pub fn paths_for(&self, book_id: &BookId) -> ReaderStoragePaths {
        match self.kind {
            StorageLayoutKind::CanonicalRustmix => canonical_paths(book_id),
            StorageLayoutKind::X4CompatFlat83 => x4_compat_paths(book_id),
        }
    }
}

fn canonical_paths(book_id: &BookId) -> ReaderStoragePaths {
    let mut root = String::new();
    push(&mut root, "/.rustmixos/books/");
    push(&mut root, book_id.as_hex());

    ReaderStoragePaths {
        meta: join(&root, "meta.bin"),
        progress: join(&root, "progress.bin"),
        bookmarks: join(&root, "bookmarks.bin"),
        theme: join(&root, "theme.bin"),
        sections_dir: join(&root, "sections"),
        bookmark_index: str_path("/.rustmixos/bookmarks/index.bin"),
    }
}

fn x4_compat_paths(book_id: &BookId) -> ReaderStoragePaths {
    let hex8 = book_id.compat_hex8_upper();

    ReaderStoragePaths {
        meta: state_file(hex8.as_str(), "MTA"),
        progress: state_file(hex8.as_str(), "PRG"),
        bookmarks: state_file(hex8.as_str(), "BKM"),
        theme: state_file(hex8.as_str(), "THM"),
        sections_dir: join(&str_path("cache"), hex8.as_str()),
        bookmark_index: str_path("state/BMIDX.TXT"),
    }
}

fn state_file(stem: &str, ext: &str) -> String<128> {
    let mut out = String::new();
    push(&mut out, "state/");
    push(&mut out, stem);
    push(&mut out, ".");
    push(&mut out, ext);
    out
}

fn str_path(path: &str) -> String<128> {
    let mut out = String::new();
    push(&mut out, path);
    out
}

fn join(base: &String<128>, leaf: &str) -> String<128> {
    let mut out = String::new();
    push(&mut out, base.as_str());
    push(&mut out, "/");
    push(&mut out, leaf);
    out
}

fn push(out: &mut String<128>, s: &str) {
    let _ = out.push_str(s);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BookId, BookIdScheme};

    #[test]
    fn x4_layout_uses_flat_83_state_files() {
        let book_id = BookId::new(BookIdScheme::ContentSampleFnv1a32V1, "8a79a61f");
        let paths = StorageLayout::x4_compat().paths_for(&book_id);
        assert_eq!(paths.meta.as_str(), "state/8A79A61F.MTA");
        assert_eq!(paths.progress.as_str(), "state/8A79A61F.PRG");
        assert_eq!(paths.bookmarks.as_str(), "state/8A79A61F.BKM");
        assert_eq!(paths.theme.as_str(), "state/8A79A61F.THM");
        assert_eq!(paths.bookmark_index.as_str(), "state/BMIDX.TXT");
        assert_eq!(paths.sections_dir.as_str(), "cache/8A79A61F");
    }

    #[test]
    fn canonical_layout_uses_rustmixos_book_root() {
        let book_id = BookId::new(BookIdScheme::ContentSampleFnv1a32V1, "8a79a61f");
        let paths = StorageLayout::canonical().paths_for(&book_id);
        assert_eq!(paths.meta.as_str(), "/.rustmixos/books/8a79a61f/meta.bin");
        assert_eq!(
            paths.progress.as_str(),
            "/.rustmixos/books/8a79a61f/progress.bin"
        );
        assert_eq!(
            paths.sections_dir.as_str(),
            "/.rustmixos/books/8a79a61f/sections"
        );
    }
}
