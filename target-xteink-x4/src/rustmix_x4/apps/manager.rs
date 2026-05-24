// Rustmix-owned app lifecycle manager: nav stack, dispatch, font propagation, draw
//
// all dispatch is static (monomorphized via with_app!); no dyn, no vtable
// loading indicator is drawn between app content and overlays so it
// sits on top of page content but under quick menu and button bumps

use crate::rustmix_x4::apps::home::HomeApp;
use crate::rustmix_x4::x4_apps::apps::files::FilesApp;
use crate::rustmix_x4::x4_apps::apps::reader::ReaderApp;
use crate::rustmix_x4::x4_apps::apps::settings::SettingsApp;
use crate::rustmix_x4::x4_apps::apps::widgets::button_feedback::LabelMode;
use crate::rustmix_x4::x4_apps::apps::{
    App, AppContext, AppId, Launcher, PendingSetting, Redraw, Transition,
};
use crate::rustmix_x4::x4_kernel::app::{AppScreen, AppShell};
use esp_hal::delay::Delay;

use crate::rustmix_x4::text::static_font_assets;
use crate::rustmix_x4::x4_apps::apps::widgets::quick_menu::QuickMenuResult;
use crate::rustmix_x4::x4_apps::apps::widgets::{ButtonFeedback, QuickMenu};
use crate::rustmix_x4::x4_apps::fonts;
use crate::rustmix_x4::x4_apps::ui::Region;
use crate::rustmix_x4::x4_kernel::board::action::{Action, ActionEvent, ButtonMapper};
use crate::rustmix_x4::x4_kernel::board::{Epd, SCREEN_H, SCREEN_W};
use crate::rustmix_x4::x4_kernel::drivers::input::Event;
use crate::rustmix_x4::x4_kernel::drivers::sdcard::SdStorage;
use crate::rustmix_x4::x4_kernel::drivers::ssd1677::Rotation;
use crate::rustmix_x4::x4_kernel::drivers::storage;
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;
use crate::rustmix_x4::x4_kernel::kernel::KernelHandle;
use crate::rustmix_x4::x4_kernel::kernel::app::AppLayer;
use crate::rustmix_x4::x4_kernel::kernel::bookmarks::BookmarkCache;
use crate::rustmix_x4::x4_kernel::kernel::config::{
    self, SystemSettings, WifiConfig, parse_settings_txt,
};

// monomorphized dispatch from AppId to concrete app type
macro_rules! with_app {
    ($id:expr, $mgr:expr, |$app:ident| $body:expr) => {
        match $id {
            AppId::Home => {
                let $app = &mut *$mgr.home;
                $body
            }
            AppId::Files => {
                let $app = &mut *$mgr.files;
                $body
            }
            AppId::Reader => {
                let $app = &mut *$mgr.reader;
                $body
            }
            AppId::Settings => {
                let $app = &mut *$mgr.settings;
                $body
            }
            AppId::Upload | AppId::TimeSync | AppId::WifiScan | AppId::BiscuitWifi => {
                unreachable!("special modes are handled outside the app dispatch loop");
            }
        }
    };
}

// shared-ref variant for read-only dispatch (draw, quick_actions)
macro_rules! with_app_ref {
    ($id:expr, $mgr:expr, |$app:ident| $body:expr) => {
        match $id {
            AppId::Home => {
                let $app = &*$mgr.home;
                $body
            }
            AppId::Files => {
                let $app = &*$mgr.files;
                $body
            }
            AppId::Reader => {
                let $app = &*$mgr.reader;
                $body
            }
            AppId::Settings => {
                let $app = &*$mgr.settings;
                $body
            }
            AppId::Upload | AppId::TimeSync | AppId::WifiScan | AppId::BiscuitWifi => {
                unreachable!("special modes are handled outside the app dispatch loop");
            }
        }
    };
}

#[allow(unused_imports)]
pub(crate) use with_app;
#[allow(unused_imports)]
pub(crate) use with_app_ref;

pub struct AppManager {
    pub launcher: &'static mut Launcher,

    pub home: &'static mut HomeApp,
    pub files: &'static mut FilesApp,
    pub reader: &'static mut ReaderApp,
    pub settings: &'static mut SettingsApp,

    pub app_shell: AppShell,

    pub quick_menu: &'static mut QuickMenu,
    pub bumps: &'static mut ButtonFeedback,

    pub mapper: ButtonMapper,
}

impl AppManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        launcher: &'static mut Launcher,
        home: &'static mut HomeApp,
        files: &'static mut FilesApp,
        reader: &'static mut ReaderApp,
        settings: &'static mut SettingsApp,
        app_shell: AppShell,
        quick_menu: &'static mut QuickMenu,
        bumps: &'static mut ButtonFeedback,
        mapper: ButtonMapper,
    ) -> Self {
        let mut this = Self {
            launcher,
            home,
            files,
            reader,
            settings,
            app_shell,
            quick_menu,
            bumps,
            mapper,
        };
        this.sync_shell_from_runtime();
        let _ = this.bumps.set_label_mode(this.active_button_label_mode());
        this
    }

    #[inline]
    pub fn active(&self) -> AppId {
        self.launcher.active()
    }

    #[inline]
    pub fn app_shell(&self) -> &AppShell {
        &self.app_shell
    }

    #[inline]
    pub fn app_shell_mut(&mut self) -> &mut AppShell {
        &mut self.app_shell
    }

    pub fn sync_shell_from_runtime(&mut self) {
        self.sync_shell_home();
        self.sync_shell_files();
        self.sync_shell_reader();
        self.sync_shell_from_active();
    }

    fn sync_shell_home(&mut self) {
        self.app_shell
            .set_home(self.home.shell_menu_items(), self.home.selected());

        if let Some(path) = self.home.recent_book_str() {
            self.app_shell.set_continue_target(path);
        } else {
            self.app_shell.clear_continue_target();
        }
    }

    fn sync_shell_files(&mut self) {
        self.app_shell.set_browser_state(
            "/",
            self.files.scroll(),
            self.files.selected(),
            self.files.total(),
            self.files.shell_entries(),
        );
    }

    fn sync_shell_reader(&mut self) {
        let filename = core::str::from_utf8(self.reader.filename_bytes()).unwrap_or("");
        if filename.is_empty() {
            return;
        }

        self.app_shell.update_reader_progress_with_offset(
            filename,
            self.reader.page() as u32,
            self.reader.chapter(),
            self.reader.is_epub(),
            self.reader.byte_offset(),
        );
    }

    fn seed_reader_handoff_from_files(&mut self) {
        if let Some(entry) = self.files.selected_shell_entry() {
            let is_epub = matches!(
                entry.kind,
                crate::rustmix_x4::x4_kernel::app::BrowserEntryKind::File
            ) && entry.name.as_bytes().ends_with(b".epub");
            self.app_shell.begin_reader_handoff(entry.name, is_epub);
        }
    }

    fn seed_reader_handoff_from_home(&mut self) {
        if let Some(path) = self.home.recent_source_path() {
            let is_epub = path.as_bytes().ends_with(b".epub");
            self.app_shell.begin_reader_handoff(path, is_epub);
        }
    }

    pub fn sync_shell_from_active(&mut self) {
        let next = match self.launcher.active() {
            AppId::Home => AppScreen::Home,
            AppId::Files => AppScreen::Browser,
            AppId::Reader => AppScreen::Reader,
            AppId::Settings => AppScreen::Settings,
            AppId::Upload | AppId::TimeSync | AppId::WifiScan | AppId::BiscuitWifi => {
                self.app_shell.screen()
            }
        };
        self.app_shell.set_screen(next);
    }

    #[inline]
    pub fn ctx(&self) -> &AppContext {
        &self.launcher.ctx
    }

    #[inline]
    pub fn ctx_mut(&mut self) -> &mut AppContext {
        &mut self.launcher.ctx
    }

    #[inline]
    pub fn has_redraw(&self) -> bool {
        self.launcher.ctx.has_redraw()
    }

    #[inline]
    pub fn take_redraw(&mut self) -> Redraw {
        self.launcher.ctx.take_redraw()
    }

    #[inline]
    pub fn request_full_redraw(&mut self) {
        self.launcher.ctx.request_full_redraw();
    }

    #[inline]
    pub fn apply_nav(
        &mut self,
        transition: Transition,
    ) -> Option<crate::rustmix_x4::x4_apps::apps::NavEvent> {
        self.launcher.apply(transition)
    }

    pub fn load_eager_settings(&mut self, k: &mut KernelHandle<'_>) {
        self.settings.load_eager(k);
        self.propagate_fonts();
        self.sync_button_config();
    }

    fn active_button_label_mode(&self) -> LabelMode {
        match self.launcher.active() {
            AppId::Home if self.home.is_game_screen() => LabelMode::Games,
            AppId::Reader => LabelMode::Reader,
            AppId::Settings => LabelMode::Settings,
            _ => LabelMode::Default,
        }
    }

    // sync button mapper and label widget from settings and active app
    pub fn sync_button_config(&mut self) {
        let swap = self.settings.system_settings().swap_buttons;
        self.mapper.set_swap(swap);

        let swap_changed = self.bumps.set_swap(swap);
        let mode_changed = self.bumps.set_label_mode(self.active_button_label_mode());
        let settings_state_changed = self
            .bumps
            .set_settings_state(self.settings.selected_tab(), self.settings.focus_tabs());

        if swap_changed || mode_changed || settings_state_changed {
            // labels changed, need to redraw the button bar
            self.launcher
                .ctx
                .mark_dirty(crate::rustmix_x4::x4_apps::ui::Region::new(
                    0,
                    crate::rustmix_x4::x4_kernel::board::SCREEN_H
                        - crate::rustmix_x4::x4_apps::ui::BUTTON_BAR_H,
                    crate::rustmix_x4::x4_kernel::board::SCREEN_W,
                    crate::rustmix_x4::x4_apps::ui::BUTTON_BAR_H,
                ));
        }
    }

    pub fn desired_display_rotation(&self) -> Rotation {
        match self.launcher.active() {
            AppId::Reader => self.reader.display_rotation(),
            _ => Rotation::Deg270,
        }
    }

    pub fn load_home_recent(&mut self, k: &mut KernelHandle<'_>) {
        self.home.load_recent(k);
        self.sync_shell_home();
    }

    pub fn enter_initial(&mut self, k: &mut KernelHandle<'_>) {
        self.home.on_enter(&mut self.launcher.ctx, k);
        self.sync_shell_from_runtime();
    }

    // collect session state to RTC memory struct before sleep
    pub fn collect_session(
        &self,
        session: &mut crate::rustmix_x4::x4_kernel::kernel::rtc_session::RtcSession,
    ) {
        use crate::rustmix_x4::x4_kernel::kernel::rtc_session::MAX_NAV_STACK;

        // save navigation stack
        session.nav_depth = self.launcher.depth() as u8;
        for i in 0..MAX_NAV_STACK {
            session.nav_stack[i] = if i < self.launcher.depth() {
                self.launcher.stack_at(i) as u8
            } else {
                0
            };
        }

        // save reader state
        session.reader_filename_len = self.reader.filename_len() as u8;
        let len = session.reader_filename_len as usize;
        session.reader_filename[..len].copy_from_slice(self.reader.filename_bytes());
        session.reader_is_epub = self.reader.is_epub() as u8;
        session.reader_chapter = self.reader.chapter();
        session.reader_page = self.reader.page() as u16;
        session.reader_byte_offset = self.reader.byte_offset();
        session.reader_font_size = self.reader.font_size_idx();

        // save files state
        session.files_scroll = self.files.scroll() as u16;
        session.files_selected = self.files.selected() as u8;
        session.files_total = self.files.total() as u16;

        // save home state
        session.home_state = self.home.state_id();
        session.home_selected = self.home.selected() as u8;
        session.home_bm_selected = self.home.bm_selected() as u8;
        session.home_bm_scroll = self.home.bm_scroll() as u8;

        // save settings cache
        let ss = self.settings.system_settings();
        session.settings_sleep_timeout = ss.sleep_timeout;
        session.settings_ghost_clear = ss.ghost_clear_every;
        session.settings_book_font = ss.book_font_size_idx;
        session.settings_ui_font = ss.ui_font_size_idx;
        session.settings_valid = 1;

        log::info!(
            "session: collected nav_depth={} active={:?}",
            session.nav_depth,
            self.launcher.active()
        );
    }

    // restore session from RTC memory; returns true if successful
    pub fn apply_session(
        &mut self,
        session: &crate::rustmix_x4::x4_kernel::kernel::rtc_session::RtcSession,
        k: &mut KernelHandle<'_>,
    ) -> bool {
        // validate session data
        if session.nav_depth == 0 || session.nav_depth > 4 {
            log::warn!("session: invalid nav_depth {}", session.nav_depth);
            return false;
        }

        // restore navigation stack
        self.launcher.restore_stack(
            session.nav_depth as usize,
            &session.nav_stack,
            |id| match id {
                0 => AppId::Home,
                1 => AppId::Files,
                2 => AppId::Reader,
                3 => AppId::Settings,
                _ => AppId::Home,
            },
        );

        log::info!(
            "session: restored nav stack depth={} active={:?}",
            session.nav_depth,
            self.launcher.active()
        );

        // restore home state (always in stack)
        self.home.restore_state(
            session.home_state,
            session.home_selected as usize,
            session.home_bm_selected as usize,
            session.home_bm_scroll as usize,
        );

        // restore files state if in stack
        if self.launcher.contains(AppId::Files) {
            self.files.restore_state(
                session.files_scroll as usize,
                session.files_selected as usize,
                session.files_total as usize,
            );
        }

        // restore reader state if active or in stack
        if self.launcher.active() == AppId::Reader || self.launcher.contains(AppId::Reader) {
            let filename = &session.reader_filename[..session.reader_filename_len as usize];
            self.reader.restore_state(
                filename,
                session.reader_is_epub != 0,
                session.reader_chapter,
                session.reader_page as usize,
                session.reader_byte_offset,
                session.reader_font_size,
            );
        }

        // propagate fonts before entering apps
        self.propagate_fonts();

        // enter apps in stack order (bottom to top)
        // Home is always at bottom
        self.home.on_enter(&mut self.launcher.ctx, k);

        // for apps above Home, call on_suspend for suspended ones, on_enter for active
        let depth = self.launcher.depth();
        for i in 1..depth {
            let app_id = self.launcher.stack_at(i);
            let is_active = i == depth - 1;

            match app_id {
                AppId::Files => {
                    if is_active {
                        self.files.on_enter(&mut self.launcher.ctx, k);
                    } else {
                        // Files was pushed then another app pushed on top
                        self.files.on_enter(&mut self.launcher.ctx, k);
                        self.files.on_suspend();
                    }
                }
                AppId::Reader => {
                    if is_active {
                        // set message for reader to know filename
                        let filename =
                            &session.reader_filename[..session.reader_filename_len as usize];
                        self.launcher.ctx.set_message(filename);
                        self.reader.on_enter(&mut self.launcher.ctx, k);
                    }
                }
                AppId::Settings => {
                    if is_active {
                        self.settings.on_enter(&mut self.launcher.ctx, k);
                    }
                }
                _ => {}
            }

            // suspend apps that aren't the top
            if !is_active {
                match app_id {
                    AppId::Home => self.home.on_suspend(),
                    AppId::Files => {} // already handled above
                    _ => {}
                }
            }
        }

        // mark full redraw needed
        self.launcher.ctx.request_full_redraw();

        true
    }

    // power-button long-press must be intercepted by the scheduler
    // before calling this method
    pub fn dispatch_event(&mut self, hw_event: Event, bm_cache: &mut BookmarkCache) -> Transition {
        let event = self.mapper.map_event(hw_event);

        let transition = if self.quick_menu.open {
            self.handle_quick_menu(event, bm_cache)
        } else if matches!(event, ActionEvent::Press(Action::Menu)) {
            let active = self.launcher.active();
            // after Settings propagation. Re-apply shared Settings prefs before the
            // quick menu snapshots Reader quick actions.
            if active == AppId::Reader {
                self.propagate_fonts();
            }

            let actions: &[_] = with_app!(active, self, |app| app.quick_actions());
            self.quick_menu.show(actions);
            self.launcher.ctx.mark_dirty(self.quick_menu.region());
            Transition::None
        } else {
            let active = self.launcher.active();
            let transition = with_app!(active, self, |app| {
                app.on_event(event, &mut self.launcher.ctx)
            });

            if active == AppId::Settings {
                self.sync_active_pending_setting();
                self.propagate_fonts();
                self.sync_button_config();
            }

            transition
        };

        self.sync_shell_from_runtime();
        transition
    }

    fn handle_quick_menu(
        &mut self,
        event: ActionEvent,
        bm_cache: &mut BookmarkCache,
    ) -> Transition {
        let action = match event {
            ActionEvent::Press(a) | ActionEvent::Repeat(a) => a,
            _ => return Transition::None,
        };

        let result = self.quick_menu.on_action(action);

        match result {
            QuickMenuResult::Consumed => {
                if self.quick_menu.dirty {
                    self.launcher.ctx.mark_dirty(self.quick_menu.region());
                    self.quick_menu.dirty = false;
                }
                Transition::None
            }

            QuickMenuResult::Close => {
                let region = self.quick_menu.region();
                self.sync_quick_menu();
                self.launcher.ctx.mark_dirty(region);
                Transition::None
            }

            QuickMenuResult::RefreshScreen => {
                self.sync_quick_menu();
                self.launcher.ctx.request_full_redraw();
                Transition::None
            }

            QuickMenuResult::AppCycleChanged(_id) => {
                let region = self.quick_menu.region();
                self.sync_quick_menu();
                if self.quick_menu.dirty {
                    self.launcher.ctx.mark_dirty(region);
                    self.quick_menu.dirty = false;
                }
                Transition::None
            }

            QuickMenuResult::GoHome => {
                self.sync_quick_menu();
                Transition::Home
            }

            QuickMenuResult::AppTrigger(id) => {
                let active = self.launcher.active();
                let region = self.quick_menu.region();
                self.sync_quick_menu();

                with_app!(active, self, |app| {
                    app.on_quick_trigger(id, &mut self.launcher.ctx);
                    // Save app state after trigger (e.g. font change
                    // may invalidate the reader's current page offset).
                    app.save_state(bm_cache);
                });

                self.launcher.ctx.mark_dirty(region);
                Transition::None
            }
        }
    }

    pub fn apply_transition(&mut self, transition: Transition, k: &mut KernelHandle<'_>) {
        if let Some(nav) = self.launcher.apply(transition) {
            log::info!("app: {:?} -> {:?}", nav.from, nav.to);

            if !matches!(
                nav.from,
                AppId::Upload | AppId::TimeSync | AppId::WifiScan | AppId::BiscuitWifi
            ) {
                if nav.from == AppId::Reader {
                    self.reader.persist_progress_records(k);
                }

                with_app!(nav.from, self, |app| {
                    app.save_state(k.bookmark_cache_mut());
                    if nav.suspend {
                        app.on_suspend();
                    } else {
                        app.on_exit();
                    }
                });
            }

            self.propagate_fonts();
            self.launcher.ctx.clear_loading();

            if nav.to == AppId::Reader {
                match nav.from {
                    AppId::Files => self.seed_reader_handoff_from_files(),
                    AppId::Home => self.seed_reader_handoff_from_home(),
                    _ => {}
                }
            }

            if !matches!(
                nav.to,
                AppId::Upload | AppId::TimeSync | AppId::WifiScan | AppId::BiscuitWifi
            ) {
                if nav.resume {
                    with_app!(nav.to, self, |app| {
                        app.on_resume(&mut self.launcher.ctx, k)
                    });
                } else {
                    with_app!(nav.to, self, |app| {
                        app.on_enter(&mut self.launcher.ctx, k)
                    });
                }
            }
            // Reader on_enter/restore can load per-book state, so global Settings must
            // be pushed after entry to keep Main Settings and Reader quick settings in sync.
            self.propagate_fonts();

            if nav.resume {
                self.launcher
                    .ctx
                    .mark_dirty(Region::new(0, 0, SCREEN_W, SCREEN_H));
            } else {
                self.launcher.ctx.request_full_redraw();
            }

            self.sync_shell_from_runtime();
            self.sync_button_config();
        }
    }

    pub async fn pre_render_full_redraw(&mut self, k: &mut KernelHandle<'_>) {
        let active = self.launcher.active();

        match active {
            AppId::Home | AppId::Files => {
                with_app!(active, self, |app| {
                    app.background(&mut self.launcher.ctx, k).await
                });

                // Keep shared chrome and shell mirrors aligned with any data that
                // was loaded before the full frame is written. This turns
                // Home category/app-list and Files/book-list transitions into a
                // single visible refresh when the data can be loaded before draw.
                self.propagate_fonts();
                self.sync_button_config();
                self.sync_shell_from_runtime();
            }
            AppId::Reader
            | AppId::Settings
            | AppId::Upload
            | AppId::TimeSync
            | AppId::WifiScan
            | AppId::BiscuitWifi => {}
        }
    }

    pub async fn run_background(&mut self, k: &mut KernelHandle<'_>) {
        let active = self.launcher.active();
        with_app!(active, self, |app| {
            app.background(&mut self.launcher.ctx, k).await
        });
        // after app entry. Re-apply shared Settings prefs before the next draw/event.
        if active == AppId::Reader || active == AppId::Settings {
            self.propagate_fonts();
        }

        for &id in &[AppId::Home, AppId::Files, AppId::Reader, AppId::Settings] {
            if id != active {
                with_app!(id, self, |app| {
                    if app.has_background_when_suspended() {
                        app.background_suspended(k);
                    }
                });
            }
        }
        // Keep the live Reader quick-setting cache aligned with the shared settings.
        self.propagate_fonts();

        // sync button configuration from settings (may have changed)
        self.sync_button_config();

        // Thin polish: refresh shell mirrors after app background work has
        // loaded recent books, file pages, or reader position.
        self.sync_shell_from_runtime();
    }

    pub fn draw(&self, strip: &mut StripBuffer) {
        let active = self.launcher.active();
        with_app_ref!(active, self, |app| app.draw(strip));

        // loading indicator: after app content, before overlays
        if self.launcher.ctx.loading_active() {
            let region = self.launcher.ctx.loading_region();
            if region.intersects(strip.logical_window()) {
                crate::rustmix_x4::x4_apps::ui::draw_loading_indicator(
                    strip,
                    region,
                    self.launcher.ctx.loading_msg(),
                    self.launcher.ctx.loading_pct(),
                );
            }
        }

        if self.quick_menu.open {
            self.quick_menu.draw(strip);
        }

        self.bumps
            .draw_with_reader_quick_menu(strip, active == AppId::Reader && self.quick_menu.open);
    }

    pub fn propagate_fonts(&mut self) {
        let ss = self.settings.system_settings();
        let _ui_idx = ss.ui_font_size_idx;
        let _ui_source = ss.ui_font_source;
        let book_idx = ss.book_font_size_idx;
        let theme_idx = ss.reading_theme;
        let show_progress = ss.reader_show_progress;
        let sunlight_fading_fix = ss.reader_sunlight_fading_fix;
        let bionic_mode = ss.reader_bionic_mode;
        let guide_dots_mode = ss.reader_guide_dots_mode;
        let reader_orientation = ss.reader_orientation;
        let prepared_profile = ss.prepared_font_profile;
        let fallback_policy = ss.prepared_fallback_policy;
        let reader_font_source = ss.reader_font_source;
        let reader_sd_font_slot = ss.reader_sd_font_slot;
        let reader_sd_font_id = ss.reader_sd_font_id;
        let _reader_sd_font_id_len = ss
            .reader_sd_font_id_len
            .min(config::READER_SD_FONT_ID_CAP as u8) as usize;
        // Design option 1: OS chrome uses compiled Inter bitmap fonts.
        // Keep the older VFN UI bridge disabled so stale SETTINGS.TXT values
        // cannot mix Inter/Lexend VFN metrics into Settings/Home chrome.
        static_font_assets::set_ui_font_source(0);
        self.reader.set_book_font_size(book_idx);
        self.reader.set_reading_theme(theme_idx);
        self.reader.set_reader_orientation(reader_orientation);
        self.reader.set_show_progress_chrome(show_progress);
        self.reader.set_sunlight_fading_fix(sunlight_fading_fix);
        self.reader.set_bionic_mode(bionic_mode);
        self.reader.set_guide_dots_mode(guide_dots_mode);
        self.reader.set_prepared_font_profile(prepared_profile);
        self.reader.set_prepared_fallback_policy(fallback_policy);
        self.reader.set_sd_font_selection(
            reader_font_source,
            reader_sd_font_slot,
            &reader_sd_font_id,
        );
        let chrome = fonts::chrome_font();
        self.reader.set_chrome_font(chrome);
        self.quick_menu
            .set_chrome_font(fonts::ui_list_section_font_fixed());
        self.bumps.set_chrome_font(chrome);
    }

    fn apply_pending_setting(&mut self, setting: PendingSetting) {
        match setting {
            PendingSetting::BookFontSize(idx) => {
                let ss = self.settings.system_settings_mut();
                if ss.book_font_size_idx != idx {
                    ss.book_font_size_idx = idx;
                    self.settings.mark_save_needed();
                }
            }
            PendingSetting::ReaderPreferences(prefs) => {
                let ss = self.settings.system_settings_mut();
                if ss.reader_preferences() != prefs {
                    ss.set_reader_preferences(prefs);
                    self.settings.mark_save_needed();
                }
            }
        }
    }

    fn sync_active_pending_setting(&mut self) {
        let active = self.launcher.active();
        let pending = with_app!(active, self, |app| app.pending_setting());
        if let Some(setting) = pending {
            self.apply_pending_setting(setting);
        }
    }

    fn sync_quick_menu(&mut self) {
        let active = self.launcher.active();
        let rotation_before = self.desired_display_rotation();

        for id in 0..=u8::MAX {
            if let Some(value) = self.quick_menu.app_cycle_value(id) {
                with_app!(active, self, |app| {
                    app.on_quick_cycle_update(id, value, &mut self.launcher.ctx);
                });
            }
        }

        self.sync_active_pending_setting();

        if active == AppId::Reader {
            self.propagate_fonts();
            let rotation_after = self.desired_display_rotation();
            if rotation_before != rotation_after {
                self.launcher.ctx.request_full_redraw();
            }
        }
    }

    fn wifi_config_for_special_mode(&self, sd: &SdStorage) -> WifiConfig {
        let mut buf = [0u8; 1024];
        let mut parsed_settings = SystemSettings::defaults();
        let mut parsed_wifi = WifiConfig::empty();

        let read_len = match storage::read_chunk_in_x4(sd, config::SETTINGS_FILE, 0, &mut buf) {
            Ok(n) => Some(n),
            Err(_) => match storage::read_file_start(sd, config::SETTINGS_FILE, &mut buf) {
                Ok((_size, n)) => Some(n),
                Err(_) => None,
            },
        };

        if let Some(n) = read_len {
            if n > 0 {
                parse_settings_txt(&buf[..n], &mut parsed_settings, &mut parsed_wifi);
                if parsed_wifi.has_credentials() && !parsed_wifi.password().is_empty() {
                    return parsed_wifi;
                }
            }
        }

        *self.settings.wifi_config()
    }

    #[inline]
    pub fn system_settings(&self) -> &crate::rustmix_x4::x4_kernel::kernel::config::SystemSettings {
        self.settings.system_settings()
    }

    #[inline]
    pub fn settings_loaded(&self) -> bool {
        self.settings.is_loaded()
    }

    #[inline]
    pub fn wifi_config(&self) -> &crate::rustmix_x4::x4_kernel::kernel::config::WifiConfig {
        self.settings.wifi_config()
    }

    #[inline]
    pub fn reader_sunlight_fading_fix_active(&self) -> bool {
        self.launcher.active() == AppId::Reader && self.reader.sunlight_fading_fix_enabled()
    }
    pub fn ghost_clear_every(&self) -> u32 {
        let configured = if self.settings.is_loaded() {
            self.settings.system_settings().ghost_clear_every as u32
        } else {
            crate::rustmix_x4::x4_kernel::kernel::DEFAULT_GHOST_CLEAR_EVERY
        };

        if self.reader_sunlight_fading_fix_active() {
            configured.min(2)
        } else {
            configured
        }
    }
}

impl AppLayer for AppManager {
    type Id = AppId;

    #[inline]
    fn active(&self) -> AppId {
        self.launcher.active()
    }

    fn dispatch_event(&mut self, event: Event, bm: &mut BookmarkCache) -> Transition {
        AppManager::dispatch_event(self, event, bm)
    }

    fn apply_transition(&mut self, t: Transition, k: &mut KernelHandle<'_>) {
        AppManager::apply_transition(self, t, k);
    }

    async fn run_background(&mut self, k: &mut KernelHandle<'_>) {
        AppManager::run_background(self, k).await;
    }

    async fn pre_render_full_redraw(&mut self, k: &mut KernelHandle<'_>) {
        AppManager::pre_render_full_redraw(self, k).await;
    }

    fn draw(&self, strip: &mut StripBuffer) {
        AppManager::draw(self, strip);
    }

    fn desired_display_rotation(&self) -> Rotation {
        AppManager::desired_display_rotation(self)
    }

    #[inline]
    fn has_redraw(&self) -> bool {
        self.launcher.ctx.has_redraw()
    }

    #[inline]
    fn take_redraw(&mut self) -> Redraw {
        self.launcher.ctx.take_redraw()
    }

    #[inline]
    fn request_full_redraw(&mut self) {
        self.launcher.ctx.request_full_redraw();
    }

    #[inline]
    fn ctx_mut(&mut self) -> &mut AppContext {
        &mut self.launcher.ctx
    }

    fn system_settings(&self) -> &SystemSettings {
        self.settings.system_settings()
    }

    fn settings_loaded(&self) -> bool {
        self.settings.is_loaded()
    }

    fn ghost_clear_every(&self) -> u32 {
        AppManager::ghost_clear_every(self)
    }

    fn wifi_config(&self) -> &WifiConfig {
        self.settings.wifi_config()
    }

    fn load_eager_settings(&mut self, k: &mut KernelHandle<'_>) {
        AppManager::load_eager_settings(self, k);
    }

    fn load_initial_state(&mut self, k: &mut KernelHandle<'_>) {
        AppManager::load_home_recent(self, k);
    }

    fn enter_initial(&mut self, k: &mut KernelHandle<'_>) {
        AppManager::enter_initial(self, k);
    }

    fn collect_session(
        &self,
        session: &mut crate::rustmix_x4::x4_kernel::kernel::rtc_session::RtcSession,
    ) {
        AppManager::collect_session(self, session);
    }

    fn apply_session(
        &mut self,
        session: &crate::rustmix_x4::x4_kernel::kernel::rtc_session::RtcSession,
        k: &mut KernelHandle<'_>,
    ) -> bool {
        AppManager::apply_session(self, session, k)
    }

    fn needs_special_mode(&self) -> bool {
        matches!(
            self.launcher.active(),
            AppId::Upload | AppId::TimeSync | AppId::WifiScan | AppId::BiscuitWifi
        )
    }

    async fn run_special_mode(
        &mut self,
        epd: &mut Epd,
        strip: &mut StripBuffer,
        delay: &mut Delay,
        sd: &SdStorage,
    ) {
        match self.launcher.active() {
            AppId::Upload => {
                // Safety: WIFI is not owned by any other driver. Network special
                // modes run in isolation and tear down before returning.
                let wifi = unsafe { esp_hal::peripherals::WIFI::steal() };
                let wifi_cfg = self.wifi_config_for_special_mode(sd);

                crate::rustmix_x4::network::upload::run_upload_mode(
                    wifi,
                    epd,
                    strip,
                    delay,
                    sd,
                    self.settings.system_settings().ui_font_size_idx,
                    &*self.bumps,
                    &wifi_cfg,
                )
                .await;
            }
            AppId::TimeSync => {
                // Safety: WIFI is not owned by any other driver. Network special
                // modes run in isolation and tear down before returning.
                let wifi = unsafe { esp_hal::peripherals::WIFI::steal() };
                let wifi_cfg = self.wifi_config_for_special_mode(sd);

                crate::rustmix_x4::network::network_time::run_time_sync_mode(
                    wifi,
                    epd,
                    strip,
                    delay,
                    sd,
                    self.settings.system_settings().ui_font_size_idx,
                    &*self.bumps,
                    &wifi_cfg,
                )
                .await;
            }
            AppId::WifiScan => {
                // Safety: scanner is isolated like upload/time-sync and tears down
                // the Wi-Fi controller before returning to Home.
                let wifi = unsafe { esp_hal::peripherals::WIFI::steal() };
                crate::rustmix_x4::network::wifi_scan::run_wifi_scan_mode(
                    wifi,
                    epd,
                    strip,
                    delay,
                    sd,
                    self.settings.system_settings().ui_font_size_idx,
                    &*self.bumps,
                )
                .await;
            }
            AppId::BiscuitWifi => {
                // Safety: WIFI is not owned by any other driver. The Biscuit-style
                // Wi-Fi setup activity runs as an isolated special mode, owns
                // scan/password UI, writes SETTINGS.TXT, then returns to Home.
                let wifi = unsafe { esp_hal::peripherals::WIFI::steal() };

                crate::rustmix_x4::network::biscuit_wifi::run_biscuit_wifi_setup_mode(
                    wifi,
                    epd,
                    strip,
                    delay,
                    sd,
                    self.settings.system_settings().ui_font_size_idx,
                    &*self.bumps,
                )
                .await;
            }
            _ => {}
        }
    }

    fn suppress_deferred_input(&self) -> bool {
        self.quick_menu.open
    }
}
