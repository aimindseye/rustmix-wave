// paginated file browser for SD card root directory
// background title scanner resolves EPUB titles from OPF metadata

// Library title layout polish is intentionally limited to
// display/layout treatment. Title source/cache behavior remains unchanged.
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use crate::rustmix_x4::ui::crossink_internal;
use crate::rustmix_x4::x4_apps::apps::{
    App, AppContext, AppId, RECENT_FILE, Transition, reader_state,
};
use crate::rustmix_x4::x4_kernel::app::BrowserEntry;
use crate::rustmix_x4::x4_kernel::board::action::{Action, ActionEvent};
use crate::rustmix_x4::x4_kernel::board::{SCREEN_H, SCREEN_W};
use crate::rustmix_x4::x4_kernel::drivers::storage::DirEntry;
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;

pub const UI_SELECTION_FLASH_REDUCTION_MARKER: &str = "ui-selection-flash-reduction-files-ok";
pub const FILES_LIBRARY_VISUAL_PARITY_MARKER: &str =
    "crossink-files-library-visual-parity-files-ok";
pub const READER_UNIFIED_TABS_MARKER: &str = "crossink-reader-unified-tabs-files-ok";
use crate::rustmix_x4::x4_apps::fonts;
use crate::rustmix_x4::x4_apps::ui::{Alignment, BitmapDynLabel, CONTENT_TOP, Region};
use crate::rustmix_x4::x4_kernel::error::{Error, ErrorKind};
use crate::rustmix_x4::x4_kernel::kernel::KernelHandle;
use crate::rustmix_x4::x4_kernel::kernel::QuickAction;
use smol_epub::cache;
use smol_epub::epub::{self, EpubMeta, EpubSpine};
use smol_epub::zip::ZipIndex;

const MAX_PAGE_SIZE: usize = 15;
const TITLE_KIND_TEXT: u8 = 2;
const TEXT_TITLE_SCAN_BYTES: usize = 768;
const TEXT_TITLE_MAX_BYTES: usize = 96;

const QA_DELETE_FILE: u8 = 1;
const QA_DELETE_CACHE: u8 = 2;
const QA_MAX: usize = 2;

fn compute_page_size(_list_y: u16) -> usize {
    crossink_internal::reader_visible_rows().min(MAX_PAGE_SIZE)
}

impl Default for FilesApp {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FilesApp {
    entries: [DirEntry; MAX_PAGE_SIZE],
    page_size: usize,
    count: usize,
    total: usize,
    scroll: usize,
    selected: usize,
    needs_load: bool,
    stale_cache: bool,
    error: Option<Error>,
    ui_fonts: fonts::UiFonts,
    ui_font_size_idx: u8,
    ui_font_source: u8,
    list_y: u16,
    reader_tab: usize,
    focus_tabs: bool,
    recent_record: Option<reader_state::RecentBookRecord>,
    bookmark_entries: Vec<reader_state::BookmarkIndexRecord>,
    bookmark_selected: usize,
    bookmark_scroll: usize,
    needs_load_reader_state: bool,

    title_scan_idx: usize,
    title_scanning: bool,
    title_reload: bool,

    qa_buf: [QuickAction; QA_MAX],
    qa_count: usize,
    pending_delete_file: bool,
    pending_delete_cache: bool,
}

impl FilesApp {
    pub fn new() -> Self {
        let uf = fonts::UiFonts::for_size(0);
        let list_y = crossink_internal::READER_LIST_TOP_WITH_TABS;
        Self {
            entries: [DirEntry::EMPTY; MAX_PAGE_SIZE],
            page_size: compute_page_size(list_y),
            count: 0,
            total: 0,
            scroll: 0,
            selected: 0,
            needs_load: false,
            stale_cache: false,
            error: None,
            ui_fonts: uf,
            ui_font_size_idx: 0,
            ui_font_source: 0,
            list_y,
            reader_tab: 2,
            focus_tabs: true,
            recent_record: None,
            bookmark_entries: Vec::new(),
            bookmark_selected: 0,
            bookmark_scroll: 0,
            needs_load_reader_state: true,
            title_scan_idx: 0,
            title_scanning: false,
            title_reload: false,
            qa_buf: [QuickAction::trigger(0, "", ""); QA_MAX],
            qa_count: 0,
            pending_delete_file: false,
            pending_delete_cache: false,
        }
    }

    pub fn set_ui_font_style(&mut self, _source: u8, _idx: u8) {
        self.ui_fonts = fonts::UiFonts::for_size(0);
    }

    pub fn set_ui_font_size(&mut self, _idx: u8) {
        self.ui_fonts = fonts::UiFonts::for_size(0);
    }

    // Session state accessors for RTC persistence
    #[inline]
    pub fn scroll(&self) -> usize {
        self.scroll
    }

    #[inline]
    pub fn selected(&self) -> usize {
        self.selected
    }

    #[inline]
    pub fn total(&self) -> usize {
        self.total
    }

    pub fn shell_entries(&self) -> Vec<BrowserEntry> {
        let mut out = Vec::with_capacity(self.count);
        for entry in self.entries[..self.count].iter() {
            let label = entry.display_name();
            if entry.is_dir {
                out.push(BrowserEntry::directory(label));
            } else {
                out.push(BrowserEntry::file(label));
            }
        }
        out
    }
    pub fn selected_shell_entry(&self) -> Option<BrowserEntry> {
        self.selected_entry().map(|entry| {
            let label = entry.display_name();
            if entry.is_dir {
                BrowserEntry::directory(label)
            } else {
                BrowserEntry::file(label)
            }
        })
    }
    pub fn restore_state(&mut self, scroll: usize, selected: usize, total: usize) {
        self.scroll = scroll;
        self.selected = selected;
        self.total = total;
        self.needs_load = true; // trigger page reload
        log::info!(
            "files: restore_state scroll={} selected={} total={}",
            scroll,
            selected,
            total
        );
    }

    fn selected_entry(&self) -> Option<&DirEntry> {
        if self.selected < self.count {
            Some(&self.entries[self.selected])
        } else {
            None
        }
    }

    fn reader_tab_from_message(ctx: &AppContext) -> Option<usize> {
        match ctx.message_str().trim() {
            "reader-tab:0" => Some(0),
            "reader-tab:1" => Some(1),
            "reader-tab:2" => Some(2),
            "reader-tab:3" => Some(3),
            _ => None,
        }
    }

    fn file_rows_active(&self) -> bool {
        matches!(self.reader_tab, 1 | 2)
    }

    fn current_tab_has_rows(&self) -> bool {
        match self.reader_tab {
            0 => self.recent_record.is_some(),
            1 | 2 => self.count > 0,
            3 => !self.bookmark_entries.is_empty(),
            _ => false,
        }
    }

    fn bookmark_visible_rows(&self) -> usize {
        self.page_size
            .min(crossink_internal::reader_visible_rows())
            .max(1)
    }

    fn bookmark_row_region(&self, index: usize) -> Region {
        crossink_internal::reader_list_row_region(index)
    }

    fn bookmark_list_region(&self) -> Region {
        crossink_internal::reader_list_region(self.bookmark_visible_rows())
    }

    fn move_bookmark_up(&mut self, ctx: &mut AppContext) {
        let total = self.bookmark_entries.len();
        if total == 0 {
            return;
        }
        let visible = self.bookmark_visible_rows();
        let old = self.bookmark_selected;
        if self.bookmark_selected > 0 {
            self.bookmark_selected -= 1;
            if self.bookmark_selected < self.bookmark_scroll {
                self.bookmark_scroll = self.bookmark_selected;
                ctx.mark_dirty(self.bookmark_list_region());
            } else {
                ctx.mark_dirty(self.bookmark_row_region(old.saturating_sub(self.bookmark_scroll)));
                ctx.mark_dirty(self.bookmark_row_region(
                    self.bookmark_selected.saturating_sub(self.bookmark_scroll),
                ));
            }
        } else {
            self.bookmark_selected = total - 1;
            if self.bookmark_selected >= visible {
                self.bookmark_scroll = self.bookmark_selected + 1 - visible;
            }
            ctx.mark_dirty(self.bookmark_list_region());
        }
        ctx.mark_dirty(crossink_internal::header_status_region());
    }

    fn move_bookmark_down(&mut self, ctx: &mut AppContext) {
        let total = self.bookmark_entries.len();
        if total == 0 {
            return;
        }
        let visible = self.bookmark_visible_rows();
        let old = self.bookmark_selected;
        if self.bookmark_selected + 1 < total {
            self.bookmark_selected += 1;
            if self.bookmark_selected >= self.bookmark_scroll + visible {
                self.bookmark_scroll = self.bookmark_selected + 1 - visible;
                ctx.mark_dirty(self.bookmark_list_region());
            } else {
                ctx.mark_dirty(self.bookmark_row_region(old.saturating_sub(self.bookmark_scroll)));
                ctx.mark_dirty(self.bookmark_row_region(
                    self.bookmark_selected.saturating_sub(self.bookmark_scroll),
                ));
            }
        } else {
            self.bookmark_selected = 0;
            self.bookmark_scroll = 0;
            ctx.mark_dirty(self.bookmark_list_region());
        }
        ctx.mark_dirty(crossink_internal::header_status_region());
    }

    fn load_reader_tab_state(&mut self, k: &mut KernelHandle<'_>) {
        self.recent_record = Self::load_recent_record(k);
        self.bookmark_entries.clear();

        let mut buf = [0u8; 4096];
        if let Ok(n) = k.read_app_subdir_chunk(
            reader_state::STATE_DIR,
            reader_state::BOOKMARKS_INDEX_FILE,
            0,
            &mut buf,
        ) {
            if n > 0 {
                if let Ok(payload) = core::str::from_utf8(&buf[..n]) {
                    self.bookmark_entries = reader_state::decode_bookmarks_index(payload);
                }
            }
        }

        let total = self.bookmark_entries.len();
        if total == 0 {
            self.bookmark_selected = 0;
            self.bookmark_scroll = 0;
        } else if self.bookmark_selected >= total {
            self.bookmark_selected = total - 1;
            let visible = self.bookmark_visible_rows();
            if self.bookmark_scroll + visible <= self.bookmark_selected {
                self.bookmark_scroll = self
                    .bookmark_selected
                    .saturating_sub(visible.saturating_sub(1));
            }
        }
    }

    fn load_recent_record(k: &mut KernelHandle<'_>) -> Option<reader_state::RecentBookRecord> {
        let mut typed = [0u8; 192];
        if let Ok(n) = k.read_app_subdir_chunk(
            reader_state::STATE_DIR,
            reader_state::RECENT_RECORD_FILE,
            0,
            &mut typed,
        ) {
            if n > 0 {
                if let Ok(text) = core::str::from_utf8(&typed[..n]) {
                    if let Some(record) = reader_state::RecentBookRecord::decode_line(text.trim()) {
                        if !record.source_path.trim().is_empty() {
                            return Some(record);
                        }
                    }
                }
            }
        }

        let mut legacy = [0u8; 160];
        if let Ok((_, n)) = k.read_app_data_start(RECENT_FILE, &mut legacy) {
            if n > 0 {
                if let Ok(path) = core::str::from_utf8(&legacy[..n.min(legacy.len())]) {
                    let path = path.trim();
                    if !path.is_empty() && !path.contains('|') {
                        return Some(reader_state::RecentBookRecord::from_path(path));
                    }
                }
            }
        }

        None
    }

    fn bookmark_row_label(entry: &reader_state::BookmarkIndexRecord) -> String {
        let mut out = String::new();
        let title = entry.display_title.trim();
        if title.is_empty() {
            out.push_str(entry.source_path.trim());
        } else {
            out.push_str(title);
        }

        let detail = entry.label.trim();
        if !detail.is_empty() {
            out.push_str(" · ");
            out.push_str(detail);
        } else {
            let _ = write!(out, " · Ch {}", u32::from(entry.chapter) + 1);
        }
        out
    }

    fn load_page(&mut self, entries: &[DirEntry], total: usize) {
        let n = entries.len().min(self.page_size);
        self.entries[..n].clone_from_slice(&entries[..n]);
        self.count = n;
        self.total = total;
        self.needs_load = false;
        self.error = None;
        if self.selected >= self.count && self.count > 0 {
            self.selected = self.count - 1;
        }
        self.rebuild_quick_actions();
    }

    fn load_failed(&mut self, e: Error) {
        self.needs_load = false;
        self.error = Some(e);
        self.count = 0;
    }

    fn row_region(&self, index: usize) -> Region {
        crossink_internal::reader_list_row_region(index)
    }

    fn list_region(&self) -> Region {
        crossink_internal::reader_list_region(self.page_size)
    }

    fn move_up(&mut self, ctx: &mut AppContext) {
        if self.selected > 0 {
            ctx.mark_dirty(self.row_region(self.selected));
            self.selected -= 1;
            ctx.mark_dirty(self.row_region(self.selected));
            self.rebuild_quick_actions();
        } else if self.scroll > 0 {
            self.scroll = self.scroll.saturating_sub(1);
            self.needs_load = true;
        } else if self.total > 0 {
            self.scroll = self.total.saturating_sub(self.page_size);
            self.selected = self.total.saturating_sub(self.scroll) - 1;
            self.needs_load = true;
        }
    }

    fn move_down(&mut self, ctx: &mut AppContext) {
        if self.selected + 1 < self.count {
            ctx.mark_dirty(self.row_region(self.selected));
            self.selected += 1;
            ctx.mark_dirty(self.row_region(self.selected));
            self.rebuild_quick_actions();
        } else if self.scroll + self.count < self.total {
            self.scroll += 1;
            self.needs_load = true;
        } else if self.total > 0 {
            self.scroll = 0;
            self.selected = 0;
            self.needs_load = true;
        }
    }

    fn jump_up(&mut self) {
        if self.scroll > 0 {
            self.scroll = self.scroll.saturating_sub(self.page_size);
            self.selected = 0;
            self.needs_load = true;
        } else {
            self.selected = 0;
        }
    }

    fn rebuild_quick_actions(&mut self) {
        let mut n = 0usize;
        let (is_file, is_epub) = if self.selected < self.count {
            let e = &self.entries[self.selected];
            let nm = &e.name[..e.name_len as usize];
            let epub = !e.is_dir && is_epub_or_epu_name(nm);
            (!e.is_dir, epub)
        } else {
            (false, false)
        };

        if is_file {
            self.qa_buf[n] = QuickAction::trigger(QA_DELETE_FILE, "Delete File", "Delete");
            n += 1;
            if is_epub {
                self.qa_buf[n] = QuickAction::trigger(QA_DELETE_CACHE, "Delete Cache", "Delete");
                n += 1;
            }
        }
        self.qa_count = n;
    }

    fn jump_down(&mut self) {
        let remaining = self.total.saturating_sub(self.scroll + self.count);
        if remaining > 0 {
            self.scroll += self.page_size.min(remaining + self.count - 1);
            self.selected = 0;
            self.needs_load = true;
        } else if self.count > 0 {
            self.selected = self.count - 1;
        }
    }

    fn switch_reader_tab(&mut self, delta: isize, ctx: &mut AppContext) {
        let len = crossink_internal::READER_TABS.len() as isize;
        if len == 0 {
            return;
        }
        self.reader_tab = (self.reader_tab as isize + delta).rem_euclid(len) as usize;
        self.focus_tabs = true;
        self.rebuild_quick_actions();
        ctx.request_full_redraw();
    }
}

impl App<AppId> for FilesApp {
    fn on_enter(&mut self, ctx: &mut AppContext, _k: &mut KernelHandle<'_>) {
        self.scroll = 0;
        self.selected = 0;
        self.needs_load = true;
        self.reader_tab = Self::reader_tab_from_message(ctx).unwrap_or(2);
        if Self::reader_tab_from_message(ctx).is_some() {
            ctx.clear_message();
        }
        self.focus_tabs = true;
        self.page_size = compute_page_size(self.list_y);
        self.stale_cache = true;
        self.error = None;
        self.title_scan_idx = 0;
        self.title_scanning = true;
        self.needs_load_reader_state = true;
        ctx.mark_dirty(Region::new(
            0,
            CONTENT_TOP,
            SCREEN_W,
            SCREEN_H - CONTENT_TOP,
        ));
    }

    fn on_exit(&mut self) {
        self.count = 0;
        self.title_scanning = false;
    }

    fn on_suspend(&mut self) {}

    fn on_resume(&mut self, ctx: &mut AppContext, _k: &mut KernelHandle<'_>) {
        ctx.mark_dirty(Region::new(
            0,
            CONTENT_TOP,
            SCREEN_W,
            SCREEN_H - CONTENT_TOP,
        ));
    }

    async fn background(&mut self, ctx: &mut AppContext, k: &mut KernelHandle<'_>) {
        if self.needs_load_reader_state {
            self.load_reader_tab_state(k);
            self.needs_load_reader_state = false;
            ctx.mark_dirty(self.list_region());
            ctx.mark_dirty(crossink_internal::header_status_region());
        }

        if self.pending_delete_file {
            self.pending_delete_file = false;
            if let Some(entry) = self.selected_entry() {
                if !entry.is_dir {
                    let mut nb = [0u8; 13];
                    let nl = entry.name_len as usize;
                    nb[..nl].copy_from_slice(&entry.name[..nl]);
                    let name = core::str::from_utf8(&nb[..nl]).unwrap_or("");
                    log::info!("files: deleting {}", name);

                    // also remove bookmark
                    k.bookmark_cache_mut().remove(&nb[..nl]);

                    match k.delete_file(name) {
                        Ok(()) => {
                            log::info!("files: deleted {}", name);
                            k.invalidate_dir_cache();
                            self.needs_load = true;
                            self.stale_cache = true;
                            self.title_scan_idx = 0;
                            self.title_scanning = true;
                            self.needs_load_reader_state = true;
                        }
                        Err(e) => {
                            log::warn!("files: delete failed: {}", e);
                        }
                    }
                }
            }
            ctx.mark_dirty(self.list_region());
            ctx.mark_dirty(crossink_internal::header_status_region());
            return;
        }

        if self.pending_delete_cache {
            self.pending_delete_cache = false;
            if let Some(entry) = self.selected_entry() {
                if !entry.is_dir {
                    let nl = entry.name_len as usize;
                    let name = core::str::from_utf8(&entry.name[..nl]).unwrap_or("");
                    let hash = cache::fnv1a(name.as_bytes());
                    let cf = cache::cache_filename(hash);
                    let cf_str = cache::cache_filename_str(&cf);
                    log::info!("files: deleting cache for {} ({})", name, cf_str);

                    // delete v3 flat cache file (best effort)
                    match k.delete_cache(cf_str) {
                        Ok(()) => log::info!("files: cache deleted for {}", name),
                        Err(e) => log::warn!("files: cache delete failed: {}", e),
                    }
                }
            }
            return;
        }

        if self.needs_load {
            if self.stale_cache {
                k.invalidate_dir_cache();
                self.stale_cache = false;
            }

            let mut buf = [DirEntry::EMPTY; MAX_PAGE_SIZE];
            match k.dir_page(self.scroll, &mut buf[..self.page_size]) {
                Ok(page) => {
                    self.load_page(&buf[..page.count], page.total);
                }
                Err(e) => {
                    log::info!("SD load failed: {}", e);
                    self.load_failed(e);
                }
            }

            if self.title_reload {
                self.title_reload = false;
                ctx.mark_dirty_coalesced(self.list_region());
                ctx.mark_dirty_coalesced(crossink_internal::header_status_region());
            } else {
                ctx.mark_dirty(self.list_region());
                ctx.mark_dirty(crossink_internal::header_status_region());
            }
            return;
        }

        if self.title_scanning {
            if let Some(dirty) = scan_one_reader_title(k, self.title_scan_idx) {
                self.title_scan_idx = dirty.next_idx;
                if dirty.resolved {
                    self.needs_load = true;
                    self.title_reload = true;
                }
            } else {
                self.title_scanning = false;
                log::info!("titles: scan complete");
            }
        }
    }

    fn on_event(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back) => {
                if self.focus_tabs {
                    Transition::Pop
                } else {
                    self.focus_tabs = true;
                    ctx.request_full_redraw();
                    Transition::None
                }
            }
            ActionEvent::LongPress(Action::Back) => Transition::Home,

            ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev) => {
                if self.focus_tabs {
                    if self.current_tab_has_rows() {
                        self.focus_tabs = false;
                    }
                    ctx.request_full_redraw();
                } else if self.file_rows_active() {
                    self.move_up(ctx);
                } else if self.reader_tab == 3 {
                    self.move_bookmark_up(ctx);
                }
                Transition::None
            }

            ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next) => {
                if self.focus_tabs {
                    if self.current_tab_has_rows() {
                        self.focus_tabs = false;
                    }
                    ctx.request_full_redraw();
                } else if self.file_rows_active() {
                    self.move_down(ctx);
                } else if self.reader_tab == 3 {
                    self.move_bookmark_down(ctx);
                }
                Transition::None
            }

            ActionEvent::Press(Action::PrevJump) | ActionEvent::Repeat(Action::PrevJump) => {
                self.switch_reader_tab(-1, ctx);
                Transition::None
            }

            ActionEvent::Press(Action::NextJump) | ActionEvent::Repeat(Action::NextJump) => {
                self.switch_reader_tab(1, ctx);
                Transition::None
            }

            ActionEvent::Press(Action::Select) => {
                if self.focus_tabs {
                    self.switch_reader_tab(1, ctx);
                    return Transition::None;
                }

                match self.reader_tab {
                    0 => {
                        if let Some(record) = &self.recent_record {
                            ctx.set_message(record.open_path().as_bytes());
                            Transition::Push(AppId::Reader)
                        } else {
                            Transition::None
                        }
                    }
                    1 | 2 => {
                        if let Some(entry) = self.selected_entry() {
                            if entry.is_dir {
                                Transition::None
                            } else {
                                ctx.set_message(entry.name_str().as_bytes());
                                Transition::Push(AppId::Reader)
                            }
                        } else {
                            Transition::None
                        }
                    }
                    3 => {
                        if let Some(entry) = self.bookmark_entries.get(self.bookmark_selected) {
                            let jump = entry.jump_message();
                            ctx.set_message(jump.as_bytes());
                            Transition::Push(AppId::Reader)
                        } else {
                            Transition::None
                        }
                    }
                    _ => Transition::None,
                }
            }

            _ => Transition::None,
        }
    }

    fn draw(&self, strip: &mut StripBuffer) {
        crossink_internal::draw_header(strip, "Reader", "");
        crossink_internal::draw_reader_tabs_focused(strip, self.reader_tab, self.focus_tabs);

        let mut status = BitmapDynLabel::<32>::new(
            crossink_internal::header_status_region(),
            fonts::chrome_font(),
        )
        .alignment(Alignment::CenterRight);
        match self.reader_tab {
            0 => {
                if self.recent_record.is_some() {
                    let _ = write!(status, "1/1");
                }
            }
            1 | 2 => {
                if self.total > 0 {
                    let _ = write!(status, "{}/{}", self.scroll + self.selected + 1, self.total);
                    if self.title_scanning {
                        let _ = write!(status, " scan");
                    }
                }
            }
            3 => {
                let total = self.bookmark_entries.len();
                if total > 0 {
                    let _ = write!(status, "{}/{}", self.bookmark_selected + 1, total);
                }
            }
            _ => {}
        }
        status.draw(strip).unwrap();

        match self.reader_tab {
            0 => {
                if let Some(record) = &self.recent_record {
                    crossink_internal::draw_reader_compact_item(
                        strip,
                        self.row_region(0),
                        record.ui_title(),
                        record.format.as_str(),
                        !self.focus_tabs,
                    );
                    for i in 1..self.page_size {
                        crossink_internal::clear_row(strip, self.row_region(i));
                    }
                } else {
                    crossink_internal::draw_reader_status_message(
                        strip,
                        "No recent book",
                        "Open a book to populate Recent",
                    );
                }
            }
            1 | 2 => {
                if let Some(e) = self.error {
                    let mut label =
                        BitmapDynLabel::<48>::new(self.row_region(0), fonts::ui_body_font_fixed())
                            .alignment(Alignment::CenterLeft);
                    let _ = core::fmt::Write::write_fmt(&mut label, format_args!("{}", e));
                    label.draw(strip).unwrap();
                    return;
                }

                if self.count == 0 && self.needs_load {
                    crossink_internal::draw_reader_status_message(strip, "Loading files...", "");
                    return;
                }

                if self.count == 0 && !self.needs_load {
                    crossink_internal::draw_reader_status_message(strip, "No files on SD card", "");
                    return;
                }

                for i in 0..self.page_size {
                    let region = self.row_region(i);
                    if i < self.count {
                        let entry = &self.entries[i];
                        let name = entry.display_name();
                        let value = if self.reader_tab == 1 {
                            book_or_file_label(entry)
                        } else {
                            entry_kind_label(entry)
                        };
                        crossink_internal::draw_reader_compact_item(
                            strip,
                            region,
                            name,
                            value,
                            i == self.selected && !self.focus_tabs,
                        );
                    } else {
                        crossink_internal::clear_row(strip, region);
                    }
                }
            }
            3 => {
                let total = self.bookmark_entries.len();
                if total == 0 {
                    crossink_internal::draw_reader_status_message(
                        strip,
                        "No bookmarks yet",
                        "Add bookmarks from the reader",
                    );
                    return;
                }

                let visible = self
                    .bookmark_visible_rows()
                    .min(total.saturating_sub(self.bookmark_scroll));
                for i in 0..self.page_size {
                    let region = self.bookmark_row_region(i);
                    if i < visible {
                        let idx = self.bookmark_scroll + i;
                        let label = Self::bookmark_row_label(&self.bookmark_entries[idx]);
                        crossink_internal::draw_reader_compact_item(
                            strip,
                            region,
                            &label,
                            "Mark",
                            idx == self.bookmark_selected && !self.focus_tabs,
                        );
                    } else {
                        crossink_internal::clear_row(strip, region);
                    }
                }
            }
            _ => {}
        }
    }

    fn quick_actions(&self) -> &[QuickAction] {
        if self.file_rows_active() {
            &self.qa_buf[..self.qa_count]
        } else {
            &[]
        }
    }

    fn on_quick_trigger(&mut self, id: u8, _ctx: &mut AppContext) {
        match id {
            QA_DELETE_FILE => {
                self.pending_delete_file = true;
            }
            QA_DELETE_CACHE => {
                self.pending_delete_cache = true;
            }
            _ => {}
        }
    }
}

struct TitleScanResult {
    next_idx: usize,
    resolved: bool,
}

fn is_ascii_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

#[allow(dead_code)]
fn contains_icase(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }

    let limit = haystack.len() - needle.len();
    let mut start = 0usize;
    while start <= limit {
        let mut matched = true;
        let mut index = 0usize;
        while index < needle.len() {
            if !haystack[start + index].eq_ignore_ascii_case(&needle[index]) {
                matched = false;
                break;
            }
            index += 1;
        }

        if matched {
            return true;
        }

        start += 1;
    }

    false
}

#[allow(dead_code)]
fn skip_text_title_line(line: &[u8]) -> bool {
    line.is_empty()
        || contains_icase(line, b"project gutenberg")
        || contains_icase(line, b"ebook")
        || contains_icase(line, b"produced by")
        || contains_icase(line, b"transcribed by")
        || contains_icase(line, b"***")
        || contains_icase(line, b"chapter ")
}

fn copy_text_title(line: &[u8], out: &mut [u8]) -> usize {
    let mut start = 0usize;
    let mut end = line.len();

    while start < end && is_ascii_space(line[start]) {
        start += 1;
    }
    while end > start && is_ascii_space(line[end - 1]) {
        end -= 1;
    }

    let title_prefix = b"Title:";
    if end.saturating_sub(start) > title_prefix.len()
        && line[start..start + title_prefix.len()].eq_ignore_ascii_case(title_prefix)
    {
        start += title_prefix.len();
        while start < end && is_ascii_space(line[start]) {
            start += 1;
        }
    }

    let mut written = 0usize;
    let mut prev_space = false;
    let max = out.len().min(TEXT_TITLE_MAX_BYTES);

    let mut index = start;
    while index < end && written < max {
        let byte = line[index];
        let normalized = if matches!(byte, b'_' | b'-') {
            b' '
        } else {
            byte
        };

        if is_ascii_space(normalized) {
            if written > 0 && !prev_space {
                out[written] = b' ';
                written += 1;
            }
            prev_space = true;
        } else if normalized.is_ascii() && !normalized.is_ascii_control() {
            out[written] = normalized;
            written += 1;
            prev_space = false;
        }

        index += 1;
    }

    while written > 0 && out[written - 1] == b' ' {
        written -= 1;
    }

    written
}

fn extract_text_title(data: &[u8], out: &mut [u8]) -> usize {
    let mut start = 0usize;
    let mut lines_seen = 0usize;

    while start < data.len() && lines_seen < 80 {
        let end = data[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| start + p)
            .unwrap_or(data.len());

        let mut line = &data[start..end];
        if line.ends_with(b"\r") {
            line = &line[..line.len() - 1];
        }

        let mut trimmed_start = 0usize;
        let mut trimmed_end = line.len();
        while trimmed_start < trimmed_end && is_ascii_space(line[trimmed_start]) {
            trimmed_start += 1;
        }
        while trimmed_end > trimmed_start && is_ascii_space(line[trimmed_end - 1]) {
            trimmed_end -= 1;
        }

        let trimmed = &line[trimmed_start..trimmed_end];
        let title_prefix = b"Title:";
        if trimmed.len() > title_prefix.len()
            && trimmed[..title_prefix.len()].eq_ignore_ascii_case(title_prefix)
        {
            let title_len = copy_text_title(trimmed, out);
            if title_len >= 3 {
                return title_len;
            }
        }

        start = end.saturating_add(1);
        lines_seen += 1;
    }

    0
}

fn scan_one_text_title(
    k: &mut KernelHandle<'_>,
    idx: usize,
    name: &str,
    next_idx: usize,
) -> Option<TitleScanResult> {
    let mut buf = [0u8; TEXT_TITLE_SCAN_BYTES];
    let mut title = [0u8; TEXT_TITLE_MAX_BYTES];

    let result = (|| -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
        let n = k.read_chunk(name, 0, &mut buf)?;
        let title_len = extract_text_title(&buf[..n], &mut title);
        if title_len == 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "text_title_scan: no title",
            ));
        }

        let title_str = core::str::from_utf8(&title[..title_len])
            .map_err(|_| Error::new(ErrorKind::BadEncoding, "text_title_scan: title"))?;

        log::info!("titles: {} -> \"{}\" (text)", name, title_str);
        let _ = k.save_title(name, title_str);
        k.dir_cache_mut().set_entry_title(idx, &title[..title_len]);
        Ok(title_len)
    })();

    if let Err(e) = &result {
        log::warn!("titles: {} text title failed: {}", name, e);
    }

    Some(TitleScanResult {
        next_idx,
        resolved: result.is_ok(),
    })
}

fn is_text_name(name: &[u8]) -> bool {
    name.len() >= 4
        && name[name.len() - 4] == b'.'
        && name[name.len() - 3..].eq_ignore_ascii_case(b"TXT")
}

fn entry_kind_label(entry: &DirEntry) -> &'static str {
    if entry.is_dir {
        return "Folder";
    }

    let name = &entry.name[..entry.name_len as usize];
    if is_epub_or_epu_name(name) {
        "Book"
    } else if is_text_name(name) {
        "Text"
    } else {
        "File"
    }
}

fn book_or_file_label(entry: &DirEntry) -> &'static str {
    if entry.is_dir {
        return "Folder";
    }

    let name = &entry.name[..entry.name_len as usize];
    if is_epub_or_epu_name(name) || is_text_name(name) {
        "Book"
    } else {
        "File"
    }
}

fn is_epub_or_epu_name(name: &[u8]) -> bool {
    if name.len() >= 5
        && name[name.len() - 5] == b'.'
        && name[name.len() - 4..].eq_ignore_ascii_case(b"EPUB")
    {
        return true;
    }

    name.len() >= 4
        && name[name.len() - 4] == b'.'
        && name[name.len() - 3..].eq_ignore_ascii_case(b"EPU")
}

fn scan_one_reader_title(k: &mut KernelHandle<'_>, from: usize) -> Option<TitleScanResult> {
    let (idx, name_buf, name_len, title_kind) =
        k.dir_cache_mut().next_untitled_reader_title(from)?;
    let name = core::str::from_utf8(&name_buf[..name_len as usize]).unwrap_or("");
    let next_idx = idx + 1;

    if title_kind == TITLE_KIND_TEXT {
        return scan_one_text_title(k, idx, name, next_idx);
    }

    log::info!("titles: scanning {} (idx {})", name, idx);

    let result = (|| -> crate::rustmix_x4::x4_kernel::error::Result<()> {
        let file_size = k.file_size(name)?;
        if file_size < 22 {
            return Err(Error::new(ErrorKind::InvalidData, "title_scan: too small"));
        }

        let tail_size = (file_size as usize).min(512);
        let tail_offset = file_size - tail_size as u32;
        let mut buf = [0u8; 512];
        let n = k.read_chunk(name, tail_offset, &mut buf[..tail_size])?;

        // ZipIndex::parse_eocd returns Result<_, &'static str>;
        // the From<&'static str> impl on Error converts automatically via ?
        let (cd_offset, cd_size) = ZipIndex::parse_eocd(&buf[..n], file_size)?;

        let mut cd_buf = Vec::new();
        cd_buf
            .try_reserve_exact(cd_size as usize)
            .map_err(|_| Error::new(ErrorKind::OutOfMemory, "title_scan: CD alloc"))?;
        cd_buf.resize(cd_size as usize, 0);

        let mut total = 0usize;
        while total < cd_buf.len() {
            let rd = k.read_chunk(name, cd_offset + total as u32, &mut cd_buf[total..])?;
            if rd == 0 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "title_scan: CD truncated",
                ));
            }
            total += rd;
        }

        let mut zip = ZipIndex::new();
        zip.parse_central_directory(&cd_buf)?;
        drop(cd_buf);

        let mut opf_path_buf = [0u8; epub::OPF_PATH_CAP];
        let opf_path_len = if let Some(ci) = zip.find("META-INF/container.xml") {
            let container = smol_epub::zip::extract_entry(
                zip.entry(ci),
                zip.entry(ci).local_offset,
                |off, b| {
                    k.read_chunk(name, off, b)
                        .map_err(|e: Error| -> &'static str { e.into() })
                },
            )?;
            let len = epub::parse_container(&container, &mut opf_path_buf)?;
            drop(container);
            len
        } else {
            epub::find_opf_in_zip(&zip, &mut opf_path_buf)?
        };

        let opf_path = core::str::from_utf8(&opf_path_buf[..opf_path_len])
            .map_err(|_| Error::new(ErrorKind::BadEncoding, "title_scan: OPF path"))?;

        let opf_idx = zip
            .find(opf_path)
            .or_else(|| zip.find_icase(opf_path))
            .ok_or(Error::new(ErrorKind::NotFound, "title_scan: OPF entry"))?;

        let opf_data = smol_epub::zip::extract_entry(
            zip.entry(opf_idx),
            zip.entry(opf_idx).local_offset,
            |off, b| {
                k.read_chunk(name, off, b)
                    .map_err(|e: Error| -> &'static str { e.into() })
            },
        )?;

        let opf_dir = opf_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let mut meta = EpubMeta::new();
        let mut spine = EpubSpine::new();
        epub::parse_opf(&opf_data, opf_dir, &zip, &mut meta, &mut spine)?;
        drop(opf_data);

        let title = meta.title_str();
        if title.is_empty() {
            return Err(Error::new(
                ErrorKind::ParseFailed,
                "title_scan: no title in OPF",
            ));
        }

        log::info!("titles: {} -> \"{}\"", name, title);
        let _ = k.save_title(name, title);
        k.dir_cache_mut().set_entry_title(idx, title.as_bytes());

        Ok(())
    })();

    if let Err(e) = &result {
        log::warn!("titles: {} failed: {}", name, e);
    }

    Some(TitleScanResult {
        next_idx,
        resolved: result.is_ok(),
    })
}
