//! Read-only SDMMC storage-browser model.
//!
//! ESP-IDF mounts the SD card at [`SD_MOUNT_POINT`]. This module intentionally
//! owns only read-only directory scans and bounded text previews. It never
//! creates, renames, deletes or writes files.

use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use log::{info, warn};

use crate::buttons::ButtonEvent;

/// VFS mount point used by the ESP-IDF FAT filesystem wrapper.
pub const SD_MOUNT_POINT: &str = "/sdcard";
/// Maximum number of visible rows on the portrait browser screen.
pub const STORAGE_PAGE_SIZE: usize = 7;
/// Maximum number of filesystem entries retained for one directory.
pub const MAX_STORAGE_ENTRIES: usize = 128;
/// Maximum number of bytes loaded for a bounded file preview.
pub const MAX_PREVIEW_BYTES: usize = 384;
/// Conservative SDMMC clock selected after physical timeout smoke testing.
pub const SDMMC_STABLE_SPEED_KHZ: u32 = 10_000;
/// Bounded command timeout used for SDMMC operations.
pub const SDMMC_COMMAND_TIMEOUT_MS: u32 = 1_000;
/// Total attempts for read-only filesystem operations after transient SDMMC errors.
pub const STORAGE_IO_RETRY_ATTEMPTS: usize = 3;
/// Delay between read-only retry attempts.
pub const STORAGE_IO_RETRY_DELAY_MS: u64 = 120;

/// Read-only browser-entry category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageEntryKind {
    /// Synthetic root row that returns to the Tools category.
    BackToHome,
    /// Synthetic row that navigates to the parent directory.
    ParentDirectory,
    /// Synthetic row that retries a failed read-only directory scan.
    RetryScan,
    /// Filesystem directory.
    Directory,
    /// Regular filesystem file.
    File,
}

impl StorageEntryKind {
    /// Compact UI badge.
    #[must_use]
    pub const fn badge(self) -> &'static str {
        match self {
            Self::BackToHome => "BACK",
            Self::ParentDirectory => "UP",
            Self::RetryScan => "RETRY",
            Self::Directory => "DIR",
            Self::File => "FILE",
        }
    }
}

/// One read-only browser row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageEntry {
    pub name: String,
    pub kind: StorageEntryKind,
    pub size_bytes: Option<u64>,
}

impl StorageEntry {
    fn back_to_home() -> Self {
        Self {
            name: "Back to Tools".into(),
            kind: StorageEntryKind::BackToHome,
            size_bytes: None,
        }
    }

    fn parent_directory() -> Self {
        Self {
            name: "..".into(),
            kind: StorageEntryKind::ParentDirectory,
            size_bytes: None,
        }
    }

    fn retry_scan() -> Self {
        Self {
            name: "Retry SD scan".into(),
            kind: StorageEntryKind::RetryScan,
            size_bytes: None,
        }
    }

    /// Compact size label for the right side of a browser row.
    #[must_use]
    pub fn size_label(&self) -> String {
        match self.kind {
            StorageEntryKind::File => self.size_bytes.map_or_else(|| "--".into(), format_bytes),
            _ => self.kind.badge().into(),
        }
    }
}

/// Bounded, read-only regular-file preview.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilePreview {
    pub name: String,
    pub text: String,
    pub truncated: bool,
    pub binary: bool,
}

impl FilePreview {
    /// Render a fixed number of short lines for the portrait preview screen.
    #[must_use]
    pub fn display_lines(&self, max_lines: usize, max_chars: usize) -> Vec<String> {
        if self.binary {
            return vec!["Binary file preview is disabled.".into()];
        }

        let mut lines = Vec::new();
        for raw_line in self.text.lines() {
            let mut remainder = raw_line.trim_end();
            if remainder.is_empty() {
                lines.push(String::new());
                if lines.len() >= max_lines {
                    break;
                }
                continue;
            }
            while !remainder.is_empty() && lines.len() < max_lines {
                let (line, rest) = split_at_char_boundary(remainder, max_chars);
                lines.push(line.to_string());
                remainder = rest;
            }
            if lines.len() >= max_lines {
                break;
            }
        }
        if lines.is_empty() {
            lines.push("(empty text file)".into());
        }
        lines
    }
}

/// Diagnostics captured for the most recent read-only directory scan.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DirectoryScanStats {
    /// Raw directory entries returned by the mounted FAT VFS, excluding `.` and `..`.
    pub raw_entries: usize,
    /// Regular files and directories retained for the product browser.
    pub retained_entries: usize,
    /// Entries classified with metadata because the directory-entry type hint was incomplete.
    pub metadata_fallbacks: usize,
    /// Non-file, non-directory or symlink entries skipped by the browser.
    pub ignored_special: usize,
}

#[derive(Debug)]
struct DirectoryScanResult {
    entries: Vec<StorageEntry>,
    stats: DirectoryScanStats,
}

/// Hardware-independent storage snapshot consumed by product screens.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageSnapshot {
    pub mounted: bool,
    pub current_path: String,
    pub entries: Vec<StorageEntry>,
    pub selected: usize,
    pub page_start: usize,
    pub preview: Option<FilePreview>,
    pub error: Option<String>,
    /// Diagnostics for the most recent read-only directory scan.
    pub scan: DirectoryScanStats,
}

impl Default for StorageSnapshot {
    fn default() -> Self {
        Self {
            mounted: false,
            current_path: SD_MOUNT_POINT.into(),
            entries: vec![StorageEntry::back_to_home()],
            selected: 0,
            page_start: 0,
            preview: None,
            error: None,
            scan: DirectoryScanStats::default(),
        }
    }
}

impl StorageSnapshot {
    /// Product-facing availability label.
    #[must_use]
    pub fn status_label(&self) -> &'static str {
        if !self.mounted {
            "NO SD"
        } else if self.error.is_some() {
            "SD RETRY"
        } else if self.scan.retained_entries == 0 {
            "SD EMPTY"
        } else {
            "SD READY"
        }
    }

    /// Current page number and total page count.
    #[must_use]
    pub fn page_label(&self) -> String {
        let pages = self.entries.len().max(1).div_ceil(STORAGE_PAGE_SIZE);
        let current = (self.page_start / STORAGE_PAGE_SIZE) + 1;
        format!("{current}/{pages}")
    }

    /// Visible rows for the current page.
    #[must_use]
    pub fn visible_entries(&self) -> &[StorageEntry] {
        let end = (self.page_start + STORAGE_PAGE_SIZE).min(self.entries.len());
        &self.entries[self.page_start..end]
    }

    /// Selected row index relative to the current page.
    #[must_use]
    pub fn selected_on_page(&self) -> usize {
        self.selected.saturating_sub(self.page_start)
    }
}

/// Result of one browser-level button event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageUiOutcome {
    None,
    SelectionChanged,
    DirectoryChanged,
    PreviewOpened,
    PreviewClosed,
    RetryRequested,
    ReturnHome,
}

/// Stateful read-only directory browser.
#[derive(Debug)]
pub struct StorageBrowser {
    root: PathBuf,
    current: PathBuf,
    mounted: bool,
    entries: Vec<StorageEntry>,
    selected: usize,
    page_start: usize,
    preview: Option<FilePreview>,
    error: Option<String>,
    scan: DirectoryScanStats,
}

impl StorageBrowser {
    /// Create a browser rooted at the mounted SD-card path.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, mounted: bool) -> Self {
        let root = root.into();
        let mut browser = Self {
            current: root.clone(),
            root,
            mounted,
            entries: Vec::new(),
            selected: 0,
            page_start: 0,
            preview: None,
            error: None,
            scan: DirectoryScanStats::default(),
        };
        browser.refresh();
        browser
    }

    /// Return a cloneable UI snapshot without leaking filesystem handles.
    #[must_use]
    pub fn snapshot(&self) -> StorageSnapshot {
        StorageSnapshot {
            mounted: self.mounted,
            current_path: self.display_path(),
            entries: self.entries.clone(),
            selected: self.selected,
            page_start: self.page_start,
            preview: self.preview.clone(),
            error: self.error.clone(),
            scan: self.scan,
        }
    }

    /// Rescan the current directory. The shell remains navigable when no card
    /// is inserted or a directory cannot be read.
    pub fn refresh(&mut self) {
        self.preview = None;
        self.error = None;
        self.scan = DirectoryScanStats::default();
        self.entries.clear();
        if self.current == self.root {
            self.entries.push(StorageEntry::back_to_home());
        } else {
            self.entries.push(StorageEntry::parent_directory());
        }

        if !self.mounted {
            self.error = Some("Insert a FAT-formatted SD card and reboot.".into());
            self.normalize_selection();
            return;
        }

        match read_directory_entries_with_retry(&self.current) {
            Ok(mut scan) => {
                self.scan = scan.stats;
                info!(
                    "rustmix-wave=storage-directory-scan path={} raw-entries={} retained-entries={} metadata-fallbacks={} ignored-special={}",
                    self.display_path(),
                    self.scan.raw_entries,
                    self.scan.retained_entries,
                    self.scan.metadata_fallbacks,
                    self.scan.ignored_special
                );
                if self.scan.raw_entries == 0 {
                    info!(
                        "rustmix-wave=storage-directory-empty path={} reason=no-fat-entries",
                        self.display_path()
                    );
                }
                self.entries.append(&mut scan.entries);
            }
            Err(error) => {
                self.error = Some(format!(
                    "Directory scan failed after {STORAGE_IO_RETRY_ATTEMPTS} attempts: {error}"
                ));
                self.entries.push(StorageEntry::retry_scan());
            }
        }
        self.normalize_selection();
    }

    /// Apply one debounced app button while the Files route is active.
    pub fn apply_button(&mut self, event: ButtonEvent) -> StorageUiOutcome {
        if self.preview.is_some() {
            if event == ButtonEvent::Select {
                self.preview = None;
                return StorageUiOutcome::PreviewClosed;
            }
            return StorageUiOutcome::None;
        }

        match event {
            ButtonEvent::Up => {
                if self.entries.is_empty() {
                    return StorageUiOutcome::None;
                }
                self.selected = self
                    .selected
                    .checked_sub(1)
                    .unwrap_or(self.entries.len() - 1);
                self.update_page_start();
                StorageUiOutcome::SelectionChanged
            }
            ButtonEvent::Down => {
                if self.entries.is_empty() {
                    return StorageUiOutcome::None;
                }
                self.selected = (self.selected + 1) % self.entries.len();
                self.update_page_start();
                StorageUiOutcome::SelectionChanged
            }
            ButtonEvent::Select => self.activate_selected(),
        }
    }

    fn activate_selected(&mut self) -> StorageUiOutcome {
        let Some(entry) = self.entries.get(self.selected).cloned() else {
            return StorageUiOutcome::None;
        };
        match entry.kind {
            StorageEntryKind::BackToHome => StorageUiOutcome::ReturnHome,
            StorageEntryKind::RetryScan => {
                self.refresh();
                StorageUiOutcome::RetryRequested
            }
            StorageEntryKind::ParentDirectory => {
                if let Some(parent) = self.current.parent() {
                    if parent.starts_with(&self.root) {
                        self.current = parent.to_path_buf();
                    }
                }
                self.selected = 0;
                self.page_start = 0;
                self.refresh();
                StorageUiOutcome::DirectoryChanged
            }
            StorageEntryKind::Directory => {
                let candidate = self.current.join(&entry.name);
                if candidate.starts_with(&self.root) {
                    self.current = candidate;
                    self.selected = 0;
                    self.page_start = 0;
                    self.refresh();
                    StorageUiOutcome::DirectoryChanged
                } else {
                    self.error = Some("Blocked directory traversal outside SD root.".into());
                    StorageUiOutcome::None
                }
            }
            StorageEntryKind::File => {
                let candidate = self.current.join(&entry.name);
                match read_preview_with_retry(&self.root, &candidate) {
                    Ok(preview) => {
                        self.preview = Some(preview);
                        StorageUiOutcome::PreviewOpened
                    }
                    Err(error) => {
                        self.error = Some(format!("Preview unavailable: {error}"));
                        StorageUiOutcome::None
                    }
                }
            }
        }
    }

    fn normalize_selection(&mut self) {
        if self.entries.is_empty() {
            self.selected = 0;
            self.page_start = 0;
        } else {
            self.selected = self.selected.min(self.entries.len() - 1);
            self.update_page_start();
        }
    }

    fn update_page_start(&mut self) {
        self.page_start = (self.selected / STORAGE_PAGE_SIZE) * STORAGE_PAGE_SIZE;
    }

    fn display_path(&self) -> String {
        if self.current == self.root {
            return SD_MOUNT_POINT.into();
        }
        self.current.strip_prefix(&self.root).map_or_else(
            |_| SD_MOUNT_POINT.into(),
            |relative| format!("{SD_MOUNT_POINT}/{}", relative.display()),
        )
    }
}

fn read_directory_entries_with_retry(path: &Path) -> io::Result<DirectoryScanResult> {
    retry_readonly_io("directory-scan", || read_directory_entries(path))
}

fn read_directory_entries(path: &Path) -> io::Result<DirectoryScanResult> {
    let mut entries = Vec::new();
    let mut stats = DirectoryScanStats::default();
    for entry in fs::read_dir(path)? {
        if entries.len() >= MAX_STORAGE_ENTRIES {
            break;
        }
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "." || name == ".." {
            continue;
        }
        stats.raw_entries += 1;

        let hinted_type = entry.file_type()?;
        if hinted_type.is_symlink() {
            stats.ignored_special += 1;
            continue;
        }

        // ESP-IDF FAT VFS caches the FILINFO collected by readdir so the
        // immediately following stat call is inexpensive. Prefer metadata for
        // final classification because some VFS implementations expose an
        // incomplete d_type hint even when the filesystem entry is valid.
        let metadata = entry.metadata()?;
        let metadata_type = metadata.file_type();
        let hint_is_incomplete = !hinted_type.is_dir() && !hinted_type.is_file();
        if hint_is_incomplete {
            stats.metadata_fallbacks += 1;
        }

        let (kind, size_bytes) = if metadata_type.is_dir() {
            (StorageEntryKind::Directory, None)
        } else if metadata_type.is_file() {
            (StorageEntryKind::File, Some(metadata.len()))
        } else {
            stats.ignored_special += 1;
            continue;
        };
        entries.push(StorageEntry {
            name,
            kind,
            size_bytes,
        });
    }
    entries.sort_by(|left, right| {
        storage_sort_rank(left.kind)
            .cmp(&storage_sort_rank(right.kind))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    stats.retained_entries = entries.len();
    Ok(DirectoryScanResult { entries, stats })
}

fn read_preview_with_retry(root: &Path, path: &Path) -> io::Result<FilePreview> {
    retry_readonly_io("preview-read", || read_preview(root, path))
}

fn retry_readonly_io<T>(
    operation: &str,
    mut action: impl FnMut() -> io::Result<T>,
) -> io::Result<T> {
    let mut last_error = None;
    for attempt in 1..=STORAGE_IO_RETRY_ATTEMPTS {
        match action() {
            Ok(value) => {
                if attempt > 1 {
                    warn!(
                        "rustmix-wave=storage-io-recovered operation={operation} attempt={attempt}/{STORAGE_IO_RETRY_ATTEMPTS}"
                    );
                }
                return Ok(value);
            }
            Err(error) => {
                warn!(
                    "rustmix-wave=storage-io-retry operation={operation} attempt={attempt}/{STORAGE_IO_RETRY_ATTEMPTS} error={error}"
                );
                last_error = Some(error);
                if attempt < STORAGE_IO_RETRY_ATTEMPTS {
                    thread::sleep(Duration::from_millis(STORAGE_IO_RETRY_DELAY_MS));
                }
            }
        }
    }
    Err(last_error.unwrap_or_else(|| {
        io::Error::other("read-only storage retry policy executed without an attempt")
    }))
}

fn read_preview(root: &Path, path: &Path) -> io::Result<FilePreview> {
    if !path.starts_with(root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "path is outside the SD-card root",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "preview requires a regular file",
        ));
    }

    let mut file = File::open(path)?;
    let mut bytes = vec![0; MAX_PREVIEW_BYTES + 1];
    let read = file.read(&mut bytes)?;
    bytes.truncate(read);
    let truncated = bytes.len() > MAX_PREVIEW_BYTES;
    if truncated {
        bytes.truncate(MAX_PREVIEW_BYTES);
    }
    let binary = bytes
        .iter()
        .any(|byte| byte.is_ascii_control() && !matches!(*byte, b'\n' | b'\r' | b'\t'));
    let text = if binary {
        String::new()
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    };
    Ok(FilePreview {
        name: path.file_name().map_or_else(
            || "(unnamed)".into(),
            |name| name.to_string_lossy().into_owned(),
        ),
        text,
        truncated,
        binary,
    })
}

const fn storage_sort_rank(kind: StorageEntryKind) -> u8 {
    match kind {
        StorageEntryKind::BackToHome | StorageEntryKind::ParentDirectory => 0,
        StorageEntryKind::RetryScan => 1,
        StorageEntryKind::Directory => 2,
        StorageEntryKind::File => 3,
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{} KB", bytes.div_ceil(1024))
    } else {
        format!("{} MB", bytes.div_ceil(1024 * 1024))
    }
}

fn split_at_char_boundary(value: &str, max_chars: usize) -> (&str, &str) {
    if value.chars().count() <= max_chars {
        return (value, "");
    }
    let byte_index = value
        .char_indices()
        .nth(max_chars)
        .map_or(value.len(), |(index, _)| index);
    (&value[..byte_index], value[byte_index..].trim_start())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        retry_readonly_io, StorageBrowser, StorageEntryKind, StorageUiOutcome, MAX_PREVIEW_BYTES,
    };
    use crate::buttons::ButtonEvent;

    fn fixture_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("epd397-{label}-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn unavailable_card_remains_navigable() {
        let browser = StorageBrowser::new("/missing-sdcard", false);
        let snapshot = browser.snapshot();
        assert!(!snapshot.mounted);
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].kind, StorageEntryKind::BackToHome);
    }

    #[test]
    fn mounted_empty_directory_reports_sd_empty() {
        let root = fixture_root("empty");
        let browser = StorageBrowser::new(&root, true);
        let snapshot = browser.snapshot();
        assert_eq!(snapshot.status_label(), "SD EMPTY");
        assert_eq!(snapshot.scan.raw_entries, 0);
        assert_eq!(snapshot.scan.retained_entries, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn directory_scan_sorts_directories_before_files() {
        let root = fixture_root("sort");
        fs::create_dir(root.join("z-dir")).unwrap();
        fs::write(root.join("a-file.txt"), b"hello").unwrap();
        let browser = StorageBrowser::new(&root, true);
        let snapshot = browser.snapshot();
        assert_eq!(snapshot.entries[0].kind, StorageEntryKind::BackToHome);
        assert_eq!(snapshot.entries[1].kind, StorageEntryKind::Directory);
        assert_eq!(snapshot.entries[2].kind, StorageEntryKind::File);
        assert_eq!(snapshot.scan.raw_entries, 2);
        assert_eq!(snapshot.scan.retained_entries, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn enters_directory_and_returns_to_parent() {
        let root = fixture_root("navigate");
        fs::create_dir(root.join("books")).unwrap();
        let mut browser = StorageBrowser::new(&root, true);
        assert_eq!(
            browser.apply_button(ButtonEvent::Down),
            StorageUiOutcome::SelectionChanged
        );
        assert_eq!(
            browser.apply_button(ButtonEvent::Select),
            StorageUiOutcome::DirectoryChanged
        );
        assert_eq!(
            browser.snapshot().entries[0].kind,
            StorageEntryKind::ParentDirectory
        );
        assert_eq!(
            browser.apply_button(ButtonEvent::Select),
            StorageUiOutcome::DirectoryChanged
        );
        assert_eq!(
            browser.snapshot().entries[0].kind,
            StorageEntryKind::BackToHome
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn opens_bounded_text_preview_and_closes_it() {
        let root = fixture_root("preview");
        fs::write(root.join("notes.txt"), vec![b'a'; MAX_PREVIEW_BYTES + 20]).unwrap();
        let mut browser = StorageBrowser::new(&root, true);
        browser.apply_button(ButtonEvent::Down);
        assert_eq!(
            browser.apply_button(ButtonEvent::Select),
            StorageUiOutcome::PreviewOpened
        );
        let preview = browser.snapshot().preview.unwrap();
        assert!(preview.truncated);
        assert!(!preview.binary);
        assert_eq!(preview.text.len(), MAX_PREVIEW_BYTES);
        assert_eq!(
            browser.apply_button(ButtonEvent::Select),
            StorageUiOutcome::PreviewClosed
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn binary_preview_is_reported_without_rendering_bytes() {
        let root = fixture_root("binary");
        fs::write(root.join("image.bin"), [0, 1, 2, 3]).unwrap();
        let mut browser = StorageBrowser::new(&root, true);
        browser.apply_button(ButtonEvent::Down);
        browser.apply_button(ButtonEvent::Select);
        let preview = browser.snapshot().preview.unwrap();
        assert!(preview.binary);
        assert!(preview.text.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retry_helper_recovers_after_transient_failures() {
        let mut attempts = 0;
        let value = retry_readonly_io("fixture", || {
            attempts += 1;
            if attempts < 3 {
                Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "transient",
                ))
            } else {
                Ok("ready")
            }
        })
        .unwrap();
        assert_eq!(value, "ready");
        assert_eq!(attempts, 3);
    }

    #[test]
    fn failed_scan_exposes_retry_row_and_warning_status() {
        let mut browser = StorageBrowser::new("/definitely-missing-sdcard-root", true);
        let snapshot = browser.snapshot();
        assert_eq!(snapshot.status_label(), "SD RETRY");
        assert_eq!(snapshot.entries[0].kind, StorageEntryKind::BackToHome);
        assert_eq!(snapshot.entries[1].kind, StorageEntryKind::RetryScan);
        assert_eq!(
            browser.apply_button(ButtonEvent::Down),
            StorageUiOutcome::SelectionChanged
        );
        assert_eq!(
            browser.apply_button(ButtonEvent::Select),
            StorageUiOutcome::RetryRequested
        );
    }
}
