use heapless::String;
use serde::{Deserialize, Serialize};

pub const STORAGE_PATH_MAX: usize = 128;
pub const STORAGE_SEGMENT_MAX: usize = 64;
pub const BOOK_ID_LEN: usize = 8;
pub const SFN_MAX: usize = 13;

pub const CURRENT_LIBRARY_ROOT: &str = "/";
pub const BOOKS_LIBRARY_ROOT: &str = "/books";
pub const STATE_DIR: &str = "state";
pub const STATE_ROOT: &str = "/state";
pub const BOOKMARK_INDEX_FILE: &str = "BMIDX.TXT";
pub const FCACHE_ROOT: &str = "/FCACHE";
pub const X4_ROOT: &str = "/RUSTMIX";
pub const SETTINGS_PATH: &str = "/RUSTMIX/SETTINGS.TXT";
pub const TITLE_CACHE_PATH: &str = "/RUSTMIX/TITLES.BIN";
pub const SLEEP_ROOT: &str = "/sleep";
pub const SLEEP_DAILY_ROOT: &str = "/sleep/daily";
pub const SLEEP_MODE_PATH: &str = "/RUSTMIX/SLPMODE.TXT";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReaderFileExtensionModel {
    Txt,
    Epub,
    Epu,
    Md,
    Bmp,
    Bin,
    Prg,
    Bkm,
    #[default]
    Unknown,
}

impl ReaderFileExtensionModel {
    pub fn from_path(path: &str) -> Self {
        let name = file_name_from_path(path);
        let Some(dot) = name.rfind('.') else {
            return Self::Unknown;
        };
        let ext = &name[dot + 1..];
        if ext.eq_ignore_ascii_case("TXT") {
            Self::Txt
        } else if ext.eq_ignore_ascii_case("EPUB") {
            Self::Epub
        } else if ext.eq_ignore_ascii_case("EPU") {
            Self::Epu
        } else if ext.eq_ignore_ascii_case("MD") {
            Self::Md
        } else if ext.eq_ignore_ascii_case("BMP") {
            Self::Bmp
        } else if ext.eq_ignore_ascii_case("BIN") {
            Self::Bin
        } else if ext.eq_ignore_ascii_case("PRG") {
            Self::Prg
        } else if ext.eq_ignore_ascii_case("BKM") {
            Self::Bkm
        } else {
            Self::Unknown
        }
    }

    pub const fn is_reader_book(self) -> bool {
        matches!(self, Self::Txt | Self::Epub | Self::Epu | Self::Md)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageRootModel {
    #[default]
    CurrentLibrary,
    BooksLibrary,
    State,
    Fcache,
    X4,
    Sleep,
    SleepDaily,
}

impl StorageRootModel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CurrentLibrary => CURRENT_LIBRARY_ROOT,
            Self::BooksLibrary => BOOKS_LIBRARY_ROOT,
            Self::State => STATE_ROOT,
            Self::Fcache => FCACHE_ROOT,
            Self::X4 => X4_ROOT,
            Self::Sleep => SLEEP_ROOT,
            Self::SleepDaily => SLEEP_DAILY_ROOT,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoragePathModel {
    pub path: String<STORAGE_PATH_MAX>,
}

impl StoragePathModel {
    pub fn new(path: &str) -> Option<Self> {
        if has_path_traversal(path) || path.is_empty() {
            return None;
        }
        let mut out = String::new();
        push_all(&mut out, path)?;
        Some(Self { path: out })
    }

    pub fn empty() -> Self {
        Self {
            path: String::new(),
        }
    }
    pub fn as_str(&self) -> &str {
        self.path.as_str()
    }
    pub fn extension(&self) -> ReaderFileExtensionModel {
        ReaderFileExtensionModel::from_path(self.path.as_str())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoragePathClassModel {
    ReaderBook,
    ReaderProgress,
    ReaderBookmark,
    BookmarkIndex,
    PreparedCacheRoot,
    PreparedCacheBook,
    Settings,
    TitleCache,
    SleepRoot,
    SleepDaily,
    SleepMode,
    #[default]
    Unknown,
}

pub fn safe_join(root: &str, segment: &str) -> Option<StoragePathModel> {
    if !is_safe_path_segment(segment) || has_path_traversal(root) {
        return None;
    }
    let mut path: String<STORAGE_PATH_MAX> = String::new();
    if root.is_empty() || root == "/" {
        path.push('/').ok()?;
        path.push_str(segment).ok()?;
    } else {
        path.push_str(root.trim_end_matches('/')).ok()?;
        path.push('/').ok()?;
        path.push_str(segment).ok()?;
    }
    Some(StoragePathModel { path })
}

pub fn library_path_current(file_name: &str) -> Option<StoragePathModel> {
    safe_join(CURRENT_LIBRARY_ROOT, file_name)
}
pub fn library_path_books(file_name: &str) -> Option<StoragePathModel> {
    safe_join(BOOKS_LIBRARY_ROOT, file_name)
}
pub fn state_progress_path(book_id: &str) -> Option<StoragePathModel> {
    state_book_file_path(book_id, "PRG")
}
pub fn state_bookmark_path(book_id: &str) -> Option<StoragePathModel> {
    state_book_file_path(book_id, "BKM")
}
pub fn bookmark_index_path() -> StoragePathModel {
    static_model("state/BMIDX.TXT")
}
pub fn fcache_root_path() -> StoragePathModel {
    static_model(FCACHE_ROOT)
}
pub fn fcache_book_path(book_id: &str) -> Option<StoragePathModel> {
    safe_join(FCACHE_ROOT, normalize_book_id(book_id)?.as_str())
}
pub fn settings_path() -> StoragePathModel {
    static_model(SETTINGS_PATH)
}
pub fn title_cache_path() -> StoragePathModel {
    static_model(TITLE_CACHE_PATH)
}
pub fn sleep_root_path() -> StoragePathModel {
    static_model(SLEEP_ROOT)
}
pub fn sleep_daily_root_path() -> StoragePathModel {
    static_model(SLEEP_DAILY_ROOT)
}
pub fn sleep_mode_path() -> StoragePathModel {
    static_model(SLEEP_MODE_PATH)
}

pub fn sleep_daily_image_path(file_name: &str) -> Option<StoragePathModel> {
    if ReaderFileExtensionModel::from_path(file_name) != ReaderFileExtensionModel::Bmp {
        return None;
    }
    safe_join(SLEEP_DAILY_ROOT, file_name)
}

pub fn classify_storage_path(path: &str) -> StoragePathClassModel {
    let normalized = path.trim_start_matches('/');
    if normalized.eq_ignore_ascii_case("RUSTMIX/SETTINGS.TXT")
        || normalized.eq_ignore_ascii_case("_x4/SETTINGS.TXT")
    {
        return StoragePathClassModel::Settings;
    }
    if normalized.eq_ignore_ascii_case("_x4/TITLES.BIN") {
        return StoragePathClassModel::TitleCache;
    }
    if normalized.eq_ignore_ascii_case("RUSTMIX/SLPMODE.TXT")
        || normalized.eq_ignore_ascii_case("SLPMODE.TXT")
    {
        return StoragePathClassModel::SleepMode;
    }
    if normalized.eq_ignore_ascii_case("sleep") {
        return StoragePathClassModel::SleepRoot;
    }
    if normalized.eq_ignore_ascii_case("sleep/daily") {
        return StoragePathClassModel::SleepDaily;
    }
    if normalized.eq_ignore_ascii_case("FCACHE") {
        return StoragePathClassModel::PreparedCacheRoot;
    }
    if let Some(id) = normalized.strip_prefix("FCACHE/")
        && normalize_book_id(id).is_some()
    {
        return StoragePathClassModel::PreparedCacheBook;
    }
    if normalized.eq_ignore_ascii_case("state/BMIDX.TXT") {
        return StoragePathClassModel::BookmarkIndex;
    }
    if normalized.starts_with("state/") && normalized.ends_with(".PRG") {
        return StoragePathClassModel::ReaderProgress;
    }
    if normalized.starts_with("state/") && normalized.ends_with(".BKM") {
        return StoragePathClassModel::ReaderBookmark;
    }
    if ReaderFileExtensionModel::from_path(normalized).is_reader_book() {
        return StoragePathClassModel::ReaderBook;
    }
    StoragePathClassModel::Unknown
}

pub fn normalize_book_id(book_id: &str) -> Option<String<BOOK_ID_LEN>> {
    let trimmed = book_id.trim();
    if trimmed.len() != BOOK_ID_LEN || !trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = String::new();
    for b in trimmed.bytes() {
        out.push((b as char).to_ascii_uppercase()).ok()?;
    }
    Some(out)
}

pub fn is_safe_path_segment(segment: &str) -> bool {
    if segment.is_empty() || segment == "." || segment == ".." {
        return false;
    }
    !segment
        .chars()
        .any(|ch| matches!(ch, '/' | '\\' | ':' | '\0') || ch.is_control())
}

pub fn has_path_traversal(path: &str) -> bool {
    path.split(['/', '\\']).any(|part| part == "..")
}

pub fn is_8dot3_file_name(name: &str) -> bool {
    let file_name = file_name_from_path(name);
    if file_name.is_empty() || file_name.len() > SFN_MAX || file_name == "." || file_name == ".." {
        return false;
    }
    let mut dots = 0usize;
    let mut base_len = 0usize;
    let mut ext_len = 0usize;
    let mut in_ext = false;
    for b in file_name.bytes() {
        if b == b'.' {
            dots += 1;
            if dots > 1 || base_len == 0 {
                return false;
            }
            in_ext = true;
            continue;
        }
        if !is_8dot3_byte(b) {
            return false;
        }
        if in_ext {
            ext_len += 1;
        } else {
            base_len += 1;
        }
    }
    base_len > 0 && base_len <= 8 && (!in_ext || (ext_len > 0 && ext_len <= 4))
}

pub fn file_name_from_path(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn state_book_file_path(book_id: &str, ext: &str) -> Option<StoragePathModel> {
    let id = normalize_book_id(book_id)?;
    let mut path: String<STORAGE_PATH_MAX> = String::new();
    path.push_str(STATE_DIR).ok()?;
    path.push('/').ok()?;
    path.push_str(id.as_str()).ok()?;
    path.push('.').ok()?;
    path.push_str(ext).ok()?;
    Some(StoragePathModel { path })
}

fn static_model(value: &str) -> StoragePathModel {
    let mut path = String::new();
    let _ = path.push_str(value);
    StoragePathModel { path }
}

fn push_all<const N: usize>(out: &mut String<N>, input: &str) -> Option<()> {
    for ch in input.chars() {
        out.push(ch).ok()?;
    }
    Some(())
}

fn is_8dot3_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'~')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yearly_h_paths_are_current_layout_compatible() {
        assert!(is_8dot3_file_name("YEARLY_H.TXT"));
        assert_eq!(
            ReaderFileExtensionModel::from_path("YEARLY_H.TXT"),
            ReaderFileExtensionModel::Txt
        );
        assert_eq!(
            library_path_current("YEARLY_H.TXT").unwrap().as_str(),
            "/YEARLY_H.TXT"
        );
        assert_eq!(
            state_progress_path("15D1296A").unwrap().as_str(),
            "state/15D1296A.PRG"
        );
        assert_eq!(
            state_bookmark_path("15D1296A").unwrap().as_str(),
            "state/15D1296A.BKM"
        );
        assert_eq!(
            fcache_book_path("15d1296a").unwrap().as_str(),
            "/FCACHE/15D1296A"
        );
    }

    #[test]
    fn alice_short_file_name_paths_are_8dot3_safe() {
        assert!(is_8dot3_file_name("ALICES~1.EPU"));
        assert_eq!(
            ReaderFileExtensionModel::from_path("/books/ALICES~1.EPU"),
            ReaderFileExtensionModel::Epu
        );
        assert_eq!(
            library_path_books("ALICES~1.EPU").unwrap().as_str(),
            "/books/ALICES~1.EPU"
        );
    }

    #[test]
    fn progress_and_bookmark_files_preserve_state_layout() {
        assert_eq!(
            state_progress_path("DEADBEEF").unwrap().as_str(),
            "state/DEADBEEF.PRG"
        );
        assert_eq!(
            state_bookmark_path("DEADBEEF").unwrap().as_str(),
            "state/DEADBEEF.BKM"
        );
        assert_eq!(bookmark_index_path().as_str(), "state/BMIDX.TXT");
        assert_eq!(
            classify_storage_path("state/BMIDX.TXT"),
            StoragePathClassModel::BookmarkIndex
        );
    }

    #[test]
    fn prepared_cache_paths_preserve_fcache_layout() {
        assert_eq!(fcache_root_path().as_str(), "/FCACHE");
        assert_eq!(
            fcache_book_path("15D1296A").unwrap().as_str(),
            "/FCACHE/15D1296A"
        );
        assert_eq!(
            classify_storage_path("/FCACHE/15D1296A"),
            StoragePathClassModel::PreparedCacheBook
        );
    }

    #[test]
    fn title_cache_settings_and_sleep_paths_preserve_layout() {
        assert_eq!(settings_path().as_str(), "/RUSTMIX/SETTINGS.TXT");
        assert_eq!(title_cache_path().as_str(), "/RUSTMIX/TITLES.BIN");
        assert_eq!(sleep_root_path().as_str(), "/sleep");
        assert_eq!(sleep_daily_root_path().as_str(), "/sleep/daily");
        assert_eq!(sleep_mode_path().as_str(), "/RUSTMIX/SLPMODE.TXT");
        assert_eq!(
            sleep_daily_image_path("monday.bmp").unwrap().as_str(),
            "/sleep/daily/monday.bmp"
        );
    }

    #[test]
    fn extension_classification_matches_reader_and_system_files() {
        assert!(ReaderFileExtensionModel::from_path("book.epub").is_reader_book());
        assert!(ReaderFileExtensionModel::from_path("book.EPU").is_reader_book());
        assert_eq!(
            ReaderFileExtensionModel::from_path("/RUSTMIX/TITLES.BIN"),
            ReaderFileExtensionModel::Bin
        );
        assert_eq!(
            ReaderFileExtensionModel::from_path("state/DEADBEEF.PRG"),
            ReaderFileExtensionModel::Prg
        );
        assert_eq!(
            ReaderFileExtensionModel::from_path("state/DEADBEEF.BKM"),
            ReaderFileExtensionModel::Bkm
        );
    }

    #[test]
    fn rejects_path_traversal_and_unsafe_segments() {
        assert!(safe_join("/books", "../SECRET.TXT").is_none());
        assert!(safe_join("/books", "nested/BOOK.TXT").is_none());
        assert!(StoragePathModel::new("/books/../SECRET.TXT").is_none());
        assert!(fcache_book_path("../../AA").is_none());
        assert!(state_progress_path("NOTHEXID").is_none());
    }
}
