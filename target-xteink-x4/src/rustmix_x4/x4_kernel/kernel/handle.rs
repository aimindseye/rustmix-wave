// kernel handle: synchronous syscall boundary for apps
//
// every storage method calls a single storage::* function and returns
// the unified Error result; apps call these directly
//
// app-specific logic (bookmarks, title scan, etc) accesses the
// underlying caches directly via bookmark_cache() / dir_cache_mut()
// rather than through dedicated handle methods

use alloc::string::String;
use core::fmt::Write as _;

use crate::rustmix_x4::x4_kernel::drivers::storage::{self, DirEntry, DirPage};
use crate::rustmix_x4::x4_kernel::error::{Error, ErrorKind, Result};
use crate::rustmix_x4::x4_kernel::kernel::bookmarks::BookmarkCache;
use crate::rustmix_x4::x4_kernel::kernel::dir_cache::DirCache;
use crate::rustmix_x4::x4_kernel::kernel::wake::uptime_secs;

#[inline]
fn is_safe_flashcards_topic_folder(folder: &str) -> bool {
    let bytes = folder.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 8
        && bytes
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_' || *b == b'-')
}

#[inline]
fn is_safe_flashcards_image_file(file: &str) -> bool {
    let Some((base, ext)) = file.rsplit_once('.') else {
        return false;
    };
    if !ext.eq_ignore_ascii_case("X4B") || base.is_empty() || base.len() > 8 {
        return false;
    }
    base.as_bytes()
        .iter()
        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_' || *b == b'-')
}

// synchronous API surface for apps
//
// borrows the Kernel for the duration of an app lifecycle method;
// no SPI, no generics, no driver types visible to apps
pub struct KernelHandle<'k> {
    pub(crate) kernel: &'k mut super::Kernel,
}

impl<'k> KernelHandle<'k> {
    pub(crate) fn new(kernel: &'k mut super::Kernel) -> Self {
        Self { kernel }
    }

    // smol-epub sync reader bridge
    //
    // smol-epub performs I/O through closures that return
    // Result<usize, &'static str>; these adapters convert
    // Error → &'static str at the boundary via the From impl.

    pub fn with_sync_reader<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(
            &mut dyn FnMut(&str, u32, &mut [u8]) -> core::result::Result<usize, &'static str>,
        ) -> R,
    {
        let sd = &self.kernel.sd;
        let mut reader = |name: &str, offset: u32, buf: &mut [u8]| {
            storage::read_file_chunk(sd, name, offset, buf)
                .map_err(|e: Error| -> &'static str { e.into() })
        };
        f(&mut reader)
    }

    pub fn with_sync_reader_app_subdir<F, R>(&mut self, dir: &str, f: F) -> R
    where
        F: FnOnce(
            &mut dyn FnMut(&str, u32, &mut [u8]) -> core::result::Result<usize, &'static str>,
        ) -> R,
    {
        let sd = &self.kernel.sd;
        let mut reader = |name: &str, offset: u32, buf: &mut [u8]| {
            storage::read_chunk_in_x4_subdir(sd, dir, name, offset, buf)
                .map_err(|e: Error| -> &'static str { e.into() })
        };
        f(&mut reader)
    }

    // storage primitives
    //
    // each calls a single storage::* function; return type is
    // Result<T> (unified Error) throughout

    #[inline]
    pub fn file_size(&mut self, name: &str) -> Result<u32> {
        storage::file_size(&self.kernel.sd, name)
    }

    #[inline]
    pub fn read_chunk(&mut self, name: &str, offset: u32, buf: &mut [u8]) -> Result<usize> {
        storage::read_file_chunk(&self.kernel.sd, name, offset, buf)
    }

    #[inline]
    pub fn read_subdir_chunk(
        &mut self,
        dir: &str,
        subdir: &str,
        name: &str,
        offset: u32,
        buf: &mut [u8],
    ) -> Result<usize> {
        storage::read_file_chunk_in_subdir(&self.kernel.sd, dir, subdir, name, offset, buf)
    }

    #[inline]
    pub fn read_file_start(&mut self, name: &str, buf: &mut [u8]) -> Result<(u32, usize)> {
        storage::read_file_start(&self.kernel.sd, name, buf)
    }

    #[inline]
    pub fn read_sd_font_manifest_start(&mut self, buf: &mut [u8]) -> Result<(u32, usize)> {
        storage::read_file_start_in_path(&self.kernel.sd, "RUSTMIX/FONTS", "MANIFEST.TXT", buf)
    }

    #[inline]
    pub fn read_sd_ui_font_manifest_start(&mut self, buf: &mut [u8]) -> Result<(u32, usize)> {
        storage::read_file_start_in_path(&self.kernel.sd, "RUSTMIX/FONTS", "UIFONTS.TXT", buf)
    }

    #[inline]
    pub fn read_sd_font_file_start(&mut self, name: &str, buf: &mut [u8]) -> Result<(u32, usize)> {
        storage::read_file_start_in_path(&self.kernel.sd, "RUSTMIX/FONTS", name, buf)
    }

    #[inline]
    pub fn read_lua_game_app_file_start(
        &mut self,
        folder: &str,
        name: &str,
        buf: &mut [u8],
    ) -> Result<(u32, usize)> {
        let path = match folder {
            "SUDOKU" => "RUSTMIX/APPS/SUDOKU",
            "MINES" => "RUSTMIX/APPS/MINES",
            "FREECELL" => "RUSTMIX/APPS/FREECELL",
            "MEMCARD" => "RUSTMIX/APPS/MEMCARD",
            "SOLITAIR" => "RUSTMIX/APPS/SOLITAIR",
            "LUDO" => "RUSTMIX/APPS/LUDO",
            "SNAKES" => "RUSTMIX/APPS/SNAKES",
            "DICT" => "RUSTMIX/APPS/DICT",
            "UNITS" => "RUSTMIX/APPS/UNITS",
            "BGQUOTE" => "RUSTMIX/APPS/BGQUOTE",
            _ => "RUSTMIX/APPS/SUDOKU",
        };
        storage::read_file_start_in_path(&self.kernel.sd, path, name, buf)
    }

    #[inline]
    pub fn write_file(&mut self, name: &str, data: &[u8]) -> Result<()> {
        storage::write_file(&self.kernel.sd, name, data)
    }

    #[inline]
    pub fn save_title(&mut self, filename: &str, title: &str) -> Result<()> {
        storage::save_title(&self.kernel.sd, filename, title)
    }

    #[inline]
    pub fn read_app_data_start(&mut self, name: &str, buf: &mut [u8]) -> Result<(u32, usize)> {
        storage::read_app_data_start(&self.kernel.sd, name, buf)
    }

    /// Reads the first chunk of `/RUSTMIX/APPS/<app_id>/<name>` for the
    /// first Daily Mantra SD Lua app proof.
    ///
    /// This is read-only and does not add recursive scanning or any Lua VM
    /// dependency. It delegates to the existing X4 storage layer.
    pub fn read_lua_app_file_start(
        &mut self,
        app_id: &str,
        name: &str,
        buf: &mut [u8],
    ) -> Result<(u32, usize)> {
        storage::read_file_start_in_three_subdir(
            &self.kernel.sd,
            "RUSTMIX",
            "APPS",
            app_id,
            name,
            buf,
        )
    }

    /// Reads the first chunk of `/RUSTMIX/APPS/<app_id>/<data_dir>/<name>`.
    ///
    /// This is used for SD-loaded Lua app data files such as
    /// `/RUSTMIX/APPS/PANCHANG/DATA/Y2026.TXT`. It is fixed-depth and read-only;
    /// it does not add recursive scanning or change raw SD/FAT/SPI behavior.
    #[inline]
    pub fn read_lua_app_data_file_start(
        &mut self,
        app_id: &str,
        data_dir: &str,
        name: &str,
        buf: &mut [u8],
    ) -> Result<(u32, usize)> {
        storage::read_file_start_in_four_subdir(
            &self.kernel.sd,
            "RUSTMIX",
            "APPS",
            app_id,
            data_dir,
            name,
            buf,
        )
    }

    /// Reads `/RUSTMIX/APPS/<APP>/<DATA_DIR>/<NAME>` for SD-loaded Lua apps.
    ///
    /// Used by Panchang for `/RUSTMIX/APPS/PANCHANG/DATA/Y2026.TXT`. This is
    /// fixed-depth/read-only and does not add recursive scanning or app
    /// execution behavior.
    #[inline]
    pub fn read_lua_app_nested_data_file_start(
        &mut self,
        app_folder: &str,
        data_dir: &str,
        name: &str,
        buf: &mut [u8],
    ) -> Result<(u32, usize)> {
        storage::read_file_start_in_rustmix_lua_app_data_file(
            &self.kernel.sd,
            app_folder,
            data_dir,
            name,
            buf,
        )
    }

    /// Reads `/RUSTMIX/APPS/PANCHANG/DATA/Y2026.TXT` for the SD-loaded Lua
    /// Panchang app using explicit 8.3 path segments.
    #[inline]
    pub fn read_lua_panchang_y2026_start(&mut self, buf: &mut [u8]) -> Result<(u32, usize)> {
        storage::read_file_start_in_path(
            &self.kernel.sd,
            "RUSTMIX/APPS/PANCHANG/DATA",
            "Y2026.TXT",
            buf,
        )
    }

    /// Reads `/RUSTMIX/APPS/FLASHCRD/TOPICS/INDEX.TXT`.
    #[inline]
    pub fn read_lua_flashcards_topic_index_start(
        &mut self,
        buf: &mut [u8],
    ) -> Result<(u32, usize)> {
        match storage::read_file_start_in_four_subdir(
            &self.kernel.sd,
            "RUSTMIX",
            "APPS",
            "FLASHCRD",
            "TOPICS",
            "INDEX.TXT",
            buf,
        ) {
            Ok(result) => Ok(result),
            Err(_) => storage::read_file_start_in_path(
                &self.kernel.sd,
                "RUSTMIX/APPS/FLASHCRD/TOPICS",
                "INDEX.TXT",
                buf,
            ),
        }
    }

    /// Reads `/RUSTMIX/APPS/FLASHCRD/TOPICS/<TOPIC>/CARDS.TXT`.
    #[inline]
    pub fn read_lua_flashcards_topic_cards_start(
        &mut self,
        topic_folder: &str,
        buf: &mut [u8],
    ) -> Result<(u32, usize)> {
        if !is_safe_flashcards_topic_folder(topic_folder) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "flashcards_topic_folder",
            ));
        }
        let mut path = String::new();
        let _ = write!(path, "RUSTMIX/APPS/FLASHCRD/TOPICS/{}", topic_folder);
        storage::read_file_start_in_path(&self.kernel.sd, path.as_str(), "CARDS.TXT", buf)
    }

    /// Reads `/RUSTMIX/APPS/FLASHCRD/TOPICS/<TOPIC>/IMG/<NAME>.X4B`.
    #[inline]
    pub fn read_lua_flashcards_topic_image_start(
        &mut self,
        topic_folder: &str,
        image_ref: &str,
        buf: &mut [u8],
    ) -> Result<(u32, usize)> {
        if !is_safe_flashcards_topic_folder(topic_folder) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "flashcards_topic_folder",
            ));
        }
        let image_file = image_ref.strip_prefix("IMG/").unwrap_or(image_ref);
        if !is_safe_flashcards_image_file(image_file) {
            return Err(Error::new(ErrorKind::InvalidData, "flashcards_image_file"));
        }
        let mut path = String::new();
        let _ = write!(path, "RUSTMIX/APPS/FLASHCRD/TOPICS/{}/IMG", topic_folder);
        storage::read_file_start_in_path(&self.kernel.sd, path.as_str(), image_file, buf)
    }

    #[inline]
    pub fn write_app_data(&mut self, name: &str, data: &[u8]) -> Result<()> {
        storage::write_file_in_dir(&self.kernel.sd, storage::X4_DIR, name, data)
    }

    #[inline]
    pub fn ensure_app_subdir(&mut self, dir: &str) -> Result<()> {
        storage::ensure_x4_subdir(&self.kernel.sd, dir)
    }

    #[inline]
    pub fn read_app_subdir_chunk(
        &mut self,
        dir: &str,
        name: &str,
        offset: u32,
        buf: &mut [u8],
    ) -> Result<usize> {
        storage::read_chunk_in_x4_subdir(&self.kernel.sd, dir, name, offset, buf)
    }

    #[inline]
    pub fn write_app_subdir(&mut self, dir: &str, name: &str, data: &[u8]) -> Result<()> {
        storage::write_in_x4_subdir(&self.kernel.sd, dir, name, data)
    }

    #[inline]
    pub fn append_app_subdir(&mut self, dir: &str, name: &str, data: &[u8]) -> Result<()> {
        storage::append_in_x4_subdir(&self.kernel.sd, dir, name, data)
    }

    #[inline]
    pub fn file_size_app_subdir(&mut self, dir: &str, name: &str) -> Result<u32> {
        storage::file_size_in_x4_subdir(&self.kernel.sd, dir, name)
    }

    #[inline]
    pub fn delete_app_subdir(&mut self, dir: &str, name: &str) -> Result<()> {
        storage::delete_in_x4_subdir(&self.kernel.sd, dir, name)
    }

    #[inline]
    pub fn read_lua_dictionary_fallback_json_start(
        &mut self,
        buf: &mut [u8],
    ) -> Result<(u32, usize)> {
        storage::read_file_start_in_path(&self.kernel.sd, "RUSTMIX/APPS/DICT", "DICT.JSN", buf)
    }

    #[inline]
    pub fn read_lua_dictionary_index_start(&mut self, buf: &mut [u8]) -> Result<(u32, usize)> {
        storage::read_file_start_in_path(&self.kernel.sd, "RUSTMIX/APPS/DICT", "INDEX.TXT", buf)
    }

    #[inline]
    pub fn read_lua_dictionary_index_chunk(
        &mut self,
        offset: u32,
        buf: &mut [u8],
    ) -> Result<usize> {
        storage::read_file_chunk_in_path(
            &self.kernel.sd,
            "RUSTMIX/APPS/DICT",
            "INDEX.TXT",
            offset,
            buf,
        )
    }

    #[inline]
    pub fn read_lua_dictionary_shard_start(
        &mut self,
        shard_file: &str,
        buf: &mut [u8],
    ) -> Result<(u32, usize)> {
        let mut name = shard_file;
        if let Some((_, tail)) = shard_file.rsplit_once('/') {
            name = tail;
        }
        if let Some((_, tail)) = name.rsplit_once('\\') {
            name = tail;
        }
        storage::read_file_start_in_path(&self.kernel.sd, "RUSTMIX/APPS/DICT/DATA", name, buf)
    }

    // _x4/ direct file ops (v3 unified cache files)

    #[inline]
    pub fn read_cache_chunk(&mut self, name: &str, offset: u32, buf: &mut [u8]) -> Result<usize> {
        storage::read_chunk_in_x4(&self.kernel.sd, name, offset, buf)
    }

    #[inline]
    pub fn write_cache(&mut self, name: &str, data: &[u8]) -> Result<()> {
        storage::write_in_x4(&self.kernel.sd, name, data)
    }

    #[inline]
    pub fn append_cache(&mut self, name: &str, data: &[u8]) -> Result<()> {
        storage::append_in_x4(&self.kernel.sd, name, data)
    }

    #[inline]
    pub fn write_cache_at(&mut self, name: &str, offset: u32, data: &[u8]) -> Result<()> {
        storage::write_at_in_x4(&self.kernel.sd, name, offset, data)
    }

    #[inline]
    pub fn delete_cache(&mut self, name: &str) -> Result<()> {
        storage::delete_in_x4(&self.kernel.sd, name)
    }

    #[inline]
    pub fn cache_file_size(&mut self, name: &str) -> Result<u32> {
        storage::file_size_in_x4(&self.kernel.sd, name)
    }

    // root directory file deletion
    #[inline]
    pub fn delete_file(&mut self, name: &str) -> Result<()> {
        storage::delete_file(&self.kernel.sd, name)
    }

    #[inline]
    pub fn list_prepared_cache_dirs(&mut self, buf: &mut [DirEntry]) -> Result<usize> {
        storage::list_dir_entries(&self.kernel.sd, "FCACHE", buf)
    }

    pub fn dir_page(&mut self, offset: usize, buf: &mut [DirEntry]) -> Result<DirPage> {
        let k = &mut *self.kernel;
        k.dir_cache.ensure_loaded(&k.sd)?;
        Ok(k.dir_cache.page(offset, buf))
    }

    pub fn invalidate_dir_cache(&mut self) {
        self.kernel.dir_cache.invalidate();
    }

    // system info (sync, no I/O)

    #[inline]
    pub fn battery_mv(&self) -> u16 {
        self.kernel.cached_battery_mv
    }

    #[inline]
    pub fn uptime_secs(&self) -> u32 {
        uptime_secs()
    }

    #[inline]
    pub fn sd_ok(&self) -> bool {
        self.kernel.sd_ok
    }

    pub fn ensure_dir_cache_loaded(&mut self) -> Result<()> {
        let k = &mut *self.kernel;
        k.dir_cache.ensure_loaded(&k.sd)
    }

    // direct cache accessors

    #[inline]
    pub fn bookmark_cache(&self) -> &BookmarkCache {
        &*self.kernel.bm_cache
    }

    #[inline]
    pub fn bookmark_cache_mut(&mut self) -> &mut BookmarkCache {
        &mut *self.kernel.bm_cache
    }

    #[inline]
    pub fn dir_cache_mut(&mut self) -> &mut DirCache {
        &mut *self.kernel.dir_cache
    }
}
