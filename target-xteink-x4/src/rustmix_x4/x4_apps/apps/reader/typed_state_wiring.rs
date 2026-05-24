//! Active Reader Typed-State Save Callsite Wiring.
//!
//! This module is intentionally X4-local because the active reader save
//! callsites live inside the imported X4 reader crate.
//!
//! It centralizes active reader writes under one typed-state facade while still
//! delegating to the existing `KernelHandle` file APIs. This avoids introducing
//! a dependency from `target-xteink-x4/src/rustmix_x4` back to `target-xteink-x4`.
//!
//! Covered active reader state files:
//! - `state/<BOOKID>.PRG`
//! - `state/<BOOKID>.THM`
//! - `state/<BOOKID>.MTA`
//! - `state/<BOOKID>.BKM`
//! - `state/BMIDX.TXT`
//! - `state/recent.txt` remains routed through the same facade as reader state
//!
//! This does not move SD/FAT/SPI/display/input/power ownership. It only makes
//! the active reader save callsites route through a single typed-state seam.

#![allow(dead_code)]

use crate::rustmix_x4::x4_apps::apps::reader_state;
use crate::rustmix_x4::x4_kernel::error::Result;
use crate::rustmix_x4::x4_kernel::kernel::KernelHandle;

pub const ACTIVE_READER_SAVE_CALLSITE_WIRING_MARKER: &str =
    "x4-active-reader-save-callsite-wiring-bundle-ok";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReaderStateWriteKind {
    Progress,
    Theme,
    Metadata,
    Bookmark,
    BookmarkIndex,
    Recent,
    UnknownStateFile,
    NonStateSubdir,
}

impl ReaderStateWriteKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Progress => "progress",
            Self::Theme => "theme",
            Self::Metadata => "metadata",
            Self::Bookmark => "bookmark",
            Self::BookmarkIndex => "bookmark-index",
            Self::Recent => "recent",
            Self::UnknownStateFile => "unknown-state-file",
            Self::NonStateSubdir => "non-state-subdir",
        }
    }

    pub const fn is_typed_record(self) -> bool {
        matches!(
            self,
            Self::Progress | Self::Theme | Self::Metadata | Self::Bookmark | Self::BookmarkIndex
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReaderStateWriteReport {
    pub kind: ReaderStateWriteKind,
    pub bytes: usize,
    pub ok: bool,
}

impl ReaderStateWriteReport {
    pub const fn accepted(self) -> bool {
        self.ok
    }
}

/// Centralized active reader `state/` directory creation.
///
/// This intentionally delegates to the existing KernelHandle API so hardware and
/// filesystem behavior remain unchanged.
pub fn ensure_state_dir(k: &mut KernelHandle<'_>) -> Result<()> {
    k.ensure_app_subdir(reader_state::STATE_DIR)
}

/// Centralized active reader subdir write facade.
///
/// This helper classifies active reader `k.write_app_subdir(...)` callsites
/// to call this function. Behavior remains delegated to KernelHandle.
pub fn write_app_subdir(
    k: &mut KernelHandle<'_>,
    dir: &str,
    name: &str,
    data: &[u8],
) -> Result<()> {
    let kind = classify_write(dir, name);
    let result = k.write_app_subdir(dir, name, data);

    match &result {
        Ok(()) => {
            log::debug!(
                "reader-state-io: write kind={} path={}/{} bytes={} ok=true",
                kind.label(),
                dir,
                name,
                data.len()
            );
        }
        Err(e) => {
            log::warn!(
                "reader-state-io: write kind={} path={}/{} bytes={} ok=false err={}",
                kind.label(),
                dir,
                name,
                data.len(),
                e
            );
        }
    }

    result
}

pub fn write_progress_record(k: &mut KernelHandle<'_>, name: &str, data: &[u8]) -> Result<()> {
    write_app_subdir(k, reader_state::STATE_DIR, name, data)
}

pub fn write_theme_record(k: &mut KernelHandle<'_>, name: &str, data: &[u8]) -> Result<()> {
    write_app_subdir(k, reader_state::STATE_DIR, name, data)
}

pub fn write_metadata_record(k: &mut KernelHandle<'_>, name: &str, data: &[u8]) -> Result<()> {
    write_app_subdir(k, reader_state::STATE_DIR, name, data)
}

pub fn write_bookmark_record(k: &mut KernelHandle<'_>, name: &str, data: &[u8]) -> Result<()> {
    write_app_subdir(k, reader_state::STATE_DIR, name, data)
}

pub fn write_bookmark_index(k: &mut KernelHandle<'_>, data: &[u8]) -> Result<()> {
    write_app_subdir(
        k,
        reader_state::STATE_DIR,
        reader_state::BOOKMARKS_INDEX_FILE,
        data,
    )
}

pub fn classify_write(dir: &str, name: &str) -> ReaderStateWriteKind {
    if dir != reader_state::STATE_DIR {
        return ReaderStateWriteKind::NonStateSubdir;
    }

    classify_state_file_name(name)
}

pub fn classify_state_file_name(name: &str) -> ReaderStateWriteKind {
    if name.eq_ignore_ascii_case(reader_state::BOOKMARKS_INDEX_FILE) {
        return ReaderStateWriteKind::BookmarkIndex;
    }

    if name.eq_ignore_ascii_case(reader_state::RECENT_RECORD_FILE) {
        return ReaderStateWriteKind::Recent;
    }

    let bytes = name.as_bytes();
    if bytes.len() < 5 {
        return ReaderStateWriteKind::UnknownStateFile;
    }

    let ext = &bytes[bytes.len() - 4..];
    if ext.eq_ignore_ascii_case(reader_state::PROGRESS_RECORD_EXT.as_bytes()) {
        ReaderStateWriteKind::Progress
    } else if ext.eq_ignore_ascii_case(reader_state::THEME_RECORD_EXT.as_bytes()) {
        ReaderStateWriteKind::Theme
    } else if ext.eq_ignore_ascii_case(reader_state::META_RECORD_EXT.as_bytes()) {
        ReaderStateWriteKind::Metadata
    } else if ext.eq_ignore_ascii_case(b".BKM") {
        ReaderStateWriteKind::Bookmark
    } else {
        ReaderStateWriteKind::UnknownStateFile
    }
}

pub fn active_reader_save_callsite_wiring_marker() -> &'static str {
    ACTIVE_READER_SAVE_CALLSITE_WIRING_MARKER
}
