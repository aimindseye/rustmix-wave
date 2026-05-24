// Rustmix-owned launcher screen: menu, bookmarks browser, network entry points

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use crate::rustmix_x4::lua::board_games::{LudoState, SnakesState};
use crate::rustmix_x4::lua::card_games::{FreeCellState, SolitaireState};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::PrimitiveStyle;

pub const X4_KEYBOARD_CROSSPOINT_SESSION_REPAIR_MARKER: &str =
    "x4-keyboard-crosspoint-session-repair-ok";

pub const X4_KEYBOARD_CROSSPOINT_DIRTY_FIELD_COMPILE_REPAIR_MARKER: &str =
    "x4-keyboard-crosspoint-dirty-field-compile-repair-ok";

use crate::rustmix_x4::lua::dictionary::{self, DictionaryState};
use crate::rustmix_x4::lua::flashcards_script::{
    self, FlashcardsScreen, LUA_FLASHCARDS_APP_FOLDER, LUA_FLASHCARDS_APP_MARKER,
    LUA_FLASHCARDS_ENTRY_FILE, LUA_FLASHCARDS_MANIFEST_FILE,
};
use crate::rustmix_x4::lua::game_stub_script::{self, LuaGameStubScreen};
use crate::rustmix_x4::lua::grid_games::{
    GridMove, MemoryCardsState, MinesweeperState, SudokuState,
};
use crate::rustmix_x4::lua::tool_stub_script::{self, LuaToolStubScreen};
use crate::rustmix_x4::lua::unit_converter::{self, UnitConverterState};
use crate::rustmix_x4::network::{time_status, wifi_scan};
use crate::rustmix_x4::ui::crossink_internal;
use crate::rustmix_x4::x4_apps::apps::reader_state;
use crate::rustmix_x4::x4_apps::apps::widgets::text_keyboard::{self, TextKeyboardAction};
use crate::rustmix_x4::x4_apps::apps::{App, AppContext, AppId, RECENT_FILE, Transition};
use crate::rustmix_x4::x4_apps::fonts;
use crate::rustmix_x4::x4_apps::fonts::bitmap::BitmapFont;
use crate::rustmix_x4::x4_apps::ui::{
    Alignment, BUTTON_BAR_H, BitmapDynLabel, BitmapLabel, CONTENT_TOP, FULL_CONTENT_W,
    LARGE_MARGIN, Region, SECTION_GAP, TITLE_Y_OFFSET,
};
use crate::rustmix_x4::x4_kernel::app::HomeMenuItem;
use crate::rustmix_x4::x4_kernel::board::action::{Action, ActionEvent};
use crate::rustmix_x4::x4_kernel::board::{SCREEN_H, SCREEN_W};
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;
use crate::rustmix_x4::x4_kernel::kernel::KernelHandle;
use crate::rustmix_x4::x4_kernel::kernel::config::{
    self, SystemSettings, WifiConfig, write_settings_txt,
};

use crate::rustmix_x4::lua::calendar_script::{
    LUA_CALENDAR_AMERICAN_EVENTS_FILE, LUA_CALENDAR_APP_FOLDER, LUA_CALENDAR_APP_MARKER,
    LUA_CALENDAR_ENTRY_FILE, LUA_CALENDAR_EVENTS_FILE, LUA_CALENDAR_HINDU_EVENTS_FILE,
    LUA_CALENDAR_MANIFEST_FILE, LUA_CALENDAR_SD_RUNTIME_MARKER, LuaCalendarScreen,
    build_combined_calendar_sd_runtime,
};
use crate::rustmix_x4::lua::daily_mantra_script::{
    LUA_DAILY_MANTRA_APP_FOLDER, LUA_DAILY_MANTRA_APP_MARKER, LUA_DAILY_MANTRA_ENTRY_FILE,
    LUA_DAILY_MANTRA_MANIFEST_FILE, LUA_DAILY_MANTRA_MANTRAS_FILE,
    LUA_DAILY_MANTRA_SD_RUNTIME_MARKER, LuaDailyMantraScreen,
};
use crate::rustmix_x4::lua::panchang_script::{
    LUA_PANCHANG_APP_FOLDER, LUA_PANCHANG_APP_MARKER, LUA_PANCHANG_DATA_FILE,
    LUA_PANCHANG_ENTRY_FILE, LUA_PANCHANG_MANIFEST_FILE, LUA_PANCHANG_SD_RUNTIME_MARKER,
    LuaPanchangScreen, build_panchang_sd_runtime,
};

pub const HOME_DASHBOARD_MARKER: &str = "x4-biscuit-home-dashboard-active-ok";
pub const HOME_NAV_POLISH_MARKER: &str = "x4-biscuit-home-nav-polish-placeholder-routing-ok";
pub const WIFI_SETTINGS_UI_EDITOR_MARKER: &str = "wifi-settings-shared-keyboard-ok";

pub const SYSTEM_ABOUT_DIAGNOSTICS_CROSSINK_POLISH_MARKER: &str =
    "system-about-diagnostics-crossink-polish-rustmix-ok";

pub const DICTIONARY_FULLSCREEN_KEYBOARD_MARKER: &str = "rustmix-dictionary-fullscreen-keyboard-ok";
pub const CROSSINK_FILES_LIBRARY_VISUAL_PARITY_HOME_MARKER: &str =
    "crossink-files-library-visual-parity-home-ok";
pub const CROSSINK_DICTIONARY_APP_POLISH_MARKER: &str = "crossink-dictionary-app-polish-rustmix-ok";

const DICTIONARY_KEYBOARD_KEY_COUNT: usize = 30;
const DICTIONARY_KEYBOARD_COLS: usize = 6;
const DICTIONARY_KEYBOARD_ROWS: usize = 5;
const DICTIONARY_KEYBOARD_GAP: u16 = 6;
const DICTIONARY_KEYBOARD_STATUS_GAP: u16 = 8;

const HOME_CARD_COUNT: usize = 6;
const HOME_GRID_COLS: usize = 2;
const HOME_GRID_ROWS: usize = 3;
const HOME_GRID_GAP: u16 = 12;
const HOME_CARD_PAD_X: u16 = 10;
const HOME_CARD_PAD_Y: u16 = 8;
const HOME_HEADER_Y: u16 = CONTENT_TOP + 8;
const HOME_HEADER_RULE_GAP: u16 = 6;
const HOME_GRID_TOP_GAP: u16 = 10;
const HOME_FOOTER_RESERVED_H: u16 = BUTTON_BAR_H + 8;
const HOME_FOOTER_GAP: u16 = 10;
const HOME_CARD_TEXT_GAP: u16 = 5;

const TOOLS_CATEGORY_ITEM_COUNT: usize = 3;

const MAX_ITEMS: usize = HOME_CARD_COUNT;
const RECENT_BUF_LEN: usize = 160;
const NETWORK_SSID_BUF_LEN: usize = config::WIFI_SSID_CAP;

const NETWORK_PASS_BUF_LEN: usize = config::WIFI_PASS_CAP;
const WIFI_EDITOR_FIELD_COUNT: u8 = 6;
const WIFI_EDITOR_MODE_ROWS: u8 = 0;
const WIFI_EDITOR_MODE_KEYBOARD_LOWER: u8 = 1;
const WIFI_EDITOR_MODE_KEYBOARD_UPPER: u8 = 2;
const WIFI_EDITOR_MODE_KEYBOARD_SYMBOLS: u8 = 3;
const WIFI_EDITOR_SAVE_MSG: &str = "Scan SSID, enter password, then Save";
const WIFI_SCAN_MAX_RESULTS: usize = 8;
const WIFI_SCAN_SSID_BUF_LEN: usize = config::WIFI_SSID_CAP;
const NETWORK_STATUS_BUF_LEN: usize = 1024;
const NETWORK_STATUS_LINE_GAP: u16 = 12;
const TIME_STATUS_BUF_LEN: usize = 512;
const GEETA_QUOTE_DATA_BUF_LEN: usize = 8192;
const FLASHCARDS_TOPIC_INDEX_BUF_LEN: usize = 2048;
const FLASHCARDS_CARD_DATA_BUF_LEN: usize = 8192;
const FLASHCARDS_X4B_W: usize = 480;
const FLASHCARDS_X4B_H: usize = 800;
const FLASHCARDS_X4B_STRIDE: usize = 60;
const FLASHCARDS_X4B_HEADER_LEN: usize = 12;
const FLASHCARDS_X4B_PAYLOAD_LEN: usize = FLASHCARDS_X4B_STRIDE * FLASHCARDS_X4B_H;
const FLASHCARDS_X4B_FILE_LEN: usize = FLASHCARDS_X4B_HEADER_LEN + FLASHCARDS_X4B_PAYLOAD_LEN;
const CALENDAR_WEEKDAY_LABELS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const CALENDAR_ROWS: usize = 6;
const CALENDAR_COLS: usize = 7;
const CALENDAR_CELL_GAP: u16 = 2;
const CALENDAR_GRID_TOP_GAP: u16 = 12;
const PANCHANG_LINE_GAP: u16 = 10;
const PANCHANG_VALUE_W: u16 = 440;
const PANCHANG_MANTRA_BLOCK_GAP: u16 = 14;
const PANCHANG_MANTRA_FILE: &str = "yearly_hindu_mantras.txt";
const PANCHANG_MANTRA_ALT_FILE: &str = "YEARLY_HINDU_MANTRAS.TXT";
const PANCHANG_MANTRA_SHORT_FILE: &str = "Yearly_h.txt";
const PANCHANG_MANTRA_SHORT_UPPER_FILE: &str = "YEARLY_H.TXT";
const PANCHANG_MANTRA_SHORT_83_FILE: &str = "YEARLY~1.TXT";
const PANCHANG_MANTRA_BUF_LEN: usize = 176;
const PANCHANG_MANTRA_SCAN_BUF_LEN: usize = 1024;
const PANCHANG_MANTRA_LINE_BUF_LEN: usize = 512;

const DAILY_MANTRA_DEFAULT_IMAGE: &str = "/sleep/daily/default.bmp";
const DAILY_MANTRA_WEEKDAY_IMAGES: [&str; 7] = [
    "/sleep/daily/sun.bmp",
    "/sleep/daily/mon.bmp",
    "/sleep/daily/tue.bmp",
    "/sleep/daily/wed.bmp",
    "/sleep/daily/thu.bmp",
    "/sleep/daily/fri.bmp",
    "/sleep/daily/sat.bmp",
];
const DAILY_MANTRA_TITLES: [&str; 7] = [
    "Sunday - Ravivar",
    "Monday - Somvar",
    "Tuesday - Mangalvar",
    "Wednesday - Budhvar",
    "Thursday - Guruvar",
    "Friday - Shukravar",
    "Saturday - Shanivar",
];
const DAILY_MANTRA_DEDICATIONS: [&str; 7] = [
    "Dedicated to Surya Dev",
    "Dedicated to Lord Shiva",
    "Dedicated to Lord Hanuman and Lord Ganesha",
    "Dedicated to Lord Ganesha and Lord Krishna",
    "Dedicated to Lord Vishnu and Brihaspati",
    "Dedicated to Goddess Lakshmi and Devi",
    "Dedicated to Lord Shani",
];
const DAILY_MANTRA_ENGLISH: [&str; 7] = [
    "Om Suryaya Namah",
    "Om Namah Shivaya",
    "Om Hanumate Namah",
    "Om Gam Ganapataye Namah",
    "Om Namo Bhagavate Vasudevaya",
    "Om Shreem Mahalakshmyai Namah",
    "Om Shanaishcharaya Namah",
];

// bookmark list layout (matches Files app)
// Group same-book bookmarks under one title to avoid row clipping.
const BM_ROW_H: u16 = 52;
const BM_ROW_GAP: u16 = 4;
const BM_ROW_STRIDE: u16 = BM_ROW_H + BM_ROW_GAP;
const BM_TITLE_Y: u16 = CONTENT_TOP + TITLE_Y_OFFSET;
const BM_HEADER_LIST_GAP: u16 = SECTION_GAP;
const BM_BOOK_TITLE_GAP: u16 = 6;
const BM_MAX_TITLE_CHARS: usize = 34;
const BM_MAX_MIXED_TITLE_CHARS: usize = 16;
const BM_MAX_ROW_CHARS: usize = 42;
const BM_STATUS_W: u16 = 144;
const BM_STATUS_X: u16 = SCREEN_W - LARGE_MARGIN - BM_STATUS_W;
const READER_HOME_TABS_UNIFIED_MARKER: &str = "crossink-reader-unified-tabs-home-ok";

const CONTENT_REGION: Region = Region::new(0, CONTENT_TOP, SCREEN_W, SCREEN_H - CONTENT_TOP);

fn compute_item_regions(heading_line_h: u16) -> [Region; MAX_ITEMS] {
    let rule_y = HOME_HEADER_Y + heading_line_h + HOME_HEADER_RULE_GAP;
    let grid_y = rule_y + 2 + HOME_GRID_TOP_GAP;
    let footer_y = SCREEN_H.saturating_sub(HOME_FOOTER_RESERVED_H);
    let grid_h = footer_y.saturating_sub(grid_y + HOME_FOOTER_GAP);
    let card_w = (FULL_CONTENT_W.saturating_sub(HOME_GRID_GAP)) / HOME_GRID_COLS as u16;
    let card_h = (grid_h.saturating_sub(HOME_GRID_GAP * (HOME_GRID_ROWS as u16 - 1)))
        / HOME_GRID_ROWS as u16;

    [
        Region::new(LARGE_MARGIN, grid_y, card_w, card_h),
        Region::new(
            LARGE_MARGIN + card_w + HOME_GRID_GAP,
            grid_y,
            card_w,
            card_h,
        ),
        Region::new(
            LARGE_MARGIN,
            grid_y + card_h + HOME_GRID_GAP,
            card_w,
            card_h,
        ),
        Region::new(
            LARGE_MARGIN + card_w + HOME_GRID_GAP,
            grid_y + card_h + HOME_GRID_GAP,
            card_w,
            card_h,
        ),
        Region::new(
            LARGE_MARGIN,
            grid_y + (card_h + HOME_GRID_GAP) * 2,
            card_w,
            card_h,
        ),
        Region::new(
            LARGE_MARGIN + card_w + HOME_GRID_GAP,
            grid_y + (card_h + HOME_GRID_GAP) * 2,
            card_w,
            card_h,
        ),
    ]
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum HomeState {
    Menu,
    ShowApps,
    ShowCategoryItems,
    ShowDailyMantra,
    ShowLuaDailyMantra,
    ShowLuaCalendar,
    ShowLuaPanchang,
    ShowLuaFlashcards,
    ShowCalendar,
    ShowPanchangLite,
    ShowBookmarks,
    ShowNetworkStatus,
    ShowWifiConnect,
    ShowDateTime,
    ShowSleepImageInfo,
    ShowDeviceInfo,
    ShowPlaceholder,
    ShowLuaGameStub,
    ShowLuaToolStub,
    ShowLuaDictionary,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum ReturnTarget {
    Menu,
    CategoryItems,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PanchangMantraScan {
    Missing,
    FileFound,
    Found,
}

enum MenuAction {
    OpenCategory(usize),
}

pub struct HomeApp {
    state: HomeState,
    selected: usize,
    ui_fonts: fonts::UiFonts,
    ui_font_size_idx: u8,
    ui_font_source: u8,
    item_regions: [Region; MAX_ITEMS],
    item_count: usize,
    active_category: usize,
    return_target: ReturnTarget,
    placeholder_title: &'static str,
    placeholder_detail: &'static str,
    lua_game_index: usize,
    lua_game_screen: LuaGameStubScreen,
    needs_load_lua_game_stub: bool,
    lua_tool_index: usize,
    lua_tool_screen: LuaToolStubScreen,
    needs_load_lua_tool_stub: bool,
    dictionary: DictionaryState,
    unit_converter: UnitConverterState,
    sudoku_game: SudokuState,
    mines_game: MinesweeperState,
    memory_game: MemoryCardsState,
    freecell_game: FreeCellState,
    solitaire_game: SolitaireState,
    ludo_game: LudoState,
    snakes_game: SnakesState,

    network_ssid: [u8; NETWORK_SSID_BUF_LEN],
    network_ssid_len: usize,
    network_pass: [u8; NETWORK_PASS_BUF_LEN],
    network_pass_len: usize,
    network_scan_results: [[u8; WIFI_SCAN_SSID_BUF_LEN]; WIFI_SCAN_MAX_RESULTS],
    network_scan_lens: [u8; WIFI_SCAN_MAX_RESULTS],
    network_scan_count: usize,
    network_scan_selected: usize,
    resume_wifi_after_scan: bool,
    network_wifi: WifiConfig,
    network_profile_slot: u8,
    network_default_slot: u8,
    wifi_editor_field: u8,
    wifi_editor_char_idx: u8,
    wifi_editor_mode: u8,
    wifi_editor_save_pending: bool,
    wifi_editor_dirty: bool,
    wifi_editor_message: &'static str,
    network_status_loaded: bool,
    network_settings_found: bool,
    network_wifi_configured: bool,
    network_wifi_password_saved: bool,
    network_sd_ok: bool,
    network_battery_mv: u16,
    network_uptime_secs: u32,
    needs_load_network_status: bool,

    time_cache: time_status::TimeCache,
    time_status_loaded: bool,
    resume_date_time_after_sync: bool,
    time_uptime_secs: u32,
    home_battery_mv: u16,
    needs_load_time_status: bool,
    calendar_month_offset: i16,

    panchang_mantra: [u8; PANCHANG_MANTRA_BUF_LEN],
    panchang_mantra_len: usize,
    panchang_mantra_loaded: bool,
    panchang_mantra_file_found: bool,
    panchang_mantra_found: bool,
    needs_load_panchang_mantra: bool,

    lua_daily_mantra_screen: LuaDailyMantraScreen,
    needs_load_lua_daily_mantra: bool,
    lua_panchang_screen: LuaPanchangScreen,
    needs_load_lua_panchang: bool,
    lua_calendar_screen: LuaCalendarScreen,
    needs_load_lua_calendar: bool,
    flashcards_screen: FlashcardsScreen,
    needs_load_flashcards: bool,
    flashcards_topic_index: usize,
    flashcards_card_index: usize,
    flashcards_topic_open: bool,
    flashcards_show_back: bool,
    flashcards_image_buf: Vec<u8>,
    flashcards_image_ready: bool,

    recent_book: [u8; RECENT_BUF_LEN],
    recent_book_len: usize,
    recent_record: Option<reader_state::RecentBookRecord>,
    needs_load_recent: bool,

    bm_entries: Vec<reader_state::BookmarkIndexRecord>,
    bm_count: usize,
    bm_selected: usize,
    bm_scroll: usize,
    needs_load_bookmarks: bool,
    reader_tab: usize,
    reader_focus_tabs: bool,
}

impl Default for HomeApp {
    fn default() -> Self {
        Self::new()
    }
}

impl HomeApp {
    pub fn new() -> Self {
        let uf = fonts::UiFonts::for_size(0);
        Self {
            state: HomeState::Menu,
            selected: 0,
            ui_fonts: uf,
            ui_font_size_idx: 0,
            ui_font_source: 0,
            item_regions: compute_item_regions(fonts::heading_font(0).line_height),
            item_count: HOME_CARD_COUNT,
            active_category: 0,
            return_target: ReturnTarget::Menu,
            placeholder_title: "Coming soon",
            placeholder_detail: "Placeholder",
            lua_game_index: 0,
            lua_game_screen: LuaGameStubScreen::fallback_for(0),
            needs_load_lua_game_stub: false,
            lua_tool_index: 0,
            lua_tool_screen: LuaToolStubScreen::fallback_for(0),
            needs_load_lua_tool_stub: false,
            dictionary: DictionaryState::default(),
            unit_converter: UnitConverterState::default(),
            sudoku_game: SudokuState::default(),
            mines_game: MinesweeperState::default(),
            memory_game: MemoryCardsState::default(),
            freecell_game: FreeCellState::default(),
            solitaire_game: SolitaireState::default(),
            ludo_game: LudoState::default(),
            snakes_game: SnakesState::default(),
            network_ssid: [0u8; NETWORK_SSID_BUF_LEN],
            network_ssid_len: 0,
            network_pass: [0u8; NETWORK_PASS_BUF_LEN],
            network_pass_len: 0,
            network_scan_results: [[0u8; WIFI_SCAN_SSID_BUF_LEN]; WIFI_SCAN_MAX_RESULTS],
            network_scan_lens: [0u8; WIFI_SCAN_MAX_RESULTS],
            network_scan_count: 0,
            network_scan_selected: 0,
            resume_wifi_after_scan: false,
            network_wifi: WifiConfig::empty(),
            network_profile_slot: 0,
            network_default_slot: 0,
            wifi_editor_field: 0,
            wifi_editor_char_idx: 0,
            wifi_editor_mode: WIFI_EDITOR_MODE_ROWS,
            wifi_editor_save_pending: false,
            wifi_editor_dirty: false,
            wifi_editor_message: WIFI_EDITOR_SAVE_MSG,
            network_status_loaded: false,
            network_settings_found: false,
            network_wifi_configured: false,
            network_wifi_password_saved: false,
            network_sd_ok: false,
            network_battery_mv: 0,
            network_uptime_secs: 0,
            needs_load_network_status: false,
            time_cache: time_status::TimeCache::default(),
            time_status_loaded: false,
            resume_date_time_after_sync: false,
            time_uptime_secs: 0,
            home_battery_mv: 0,
            needs_load_time_status: false,
            calendar_month_offset: 0,

            panchang_mantra: [0u8; PANCHANG_MANTRA_BUF_LEN],
            panchang_mantra_len: 0,
            panchang_mantra_loaded: false,
            panchang_mantra_file_found: false,
            panchang_mantra_found: false,
            needs_load_panchang_mantra: false,
            lua_daily_mantra_screen: LuaDailyMantraScreen::default(),
            needs_load_lua_daily_mantra: false,
            lua_panchang_screen: LuaPanchangScreen::default(),
            needs_load_lua_panchang: false,
            lua_calendar_screen: LuaCalendarScreen::default(),
            needs_load_lua_calendar: false,
            flashcards_screen: FlashcardsScreen::default(),
            needs_load_flashcards: false,
            flashcards_topic_index: 0,
            flashcards_card_index: 0,
            flashcards_topic_open: false,
            flashcards_show_back: false,
            flashcards_image_buf: Vec::new(),
            flashcards_image_ready: false,
            recent_book: [0u8; RECENT_BUF_LEN],
            recent_book_len: 0,
            recent_record: None,
            needs_load_recent: false,
            bm_entries: Vec::new(),
            bm_count: 0,
            bm_selected: 0,
            bm_scroll: 0,
            needs_load_bookmarks: false,
            reader_tab: 3,
            reader_focus_tabs: true,
        }
    }

    pub fn is_game_screen(&self) -> bool {
        matches!(self.state, HomeState::ShowLuaGameStub)
    }

    pub fn set_ui_font_style(&mut self, _source: u8, _idx: u8) {
        self.ui_fonts = fonts::UiFonts::for_size(0);
    }

    pub fn set_ui_font_size(&mut self, _idx: u8) {
        self.ui_fonts = fonts::UiFonts::for_size(0);
    }

    fn refresh_menu_layout(&mut self) {
        self.item_regions = compute_item_regions(self.header_font().line_height);
    }

    // Session state accessors for RTC persistence
    #[inline]
    pub fn state_id(&self) -> u8 {
        match self.state {
            HomeState::Menu => 0,
            HomeState::ShowApps => 1,
            HomeState::ShowDailyMantra => 2,
            HomeState::ShowLuaDailyMantra => 11,
            HomeState::ShowLuaCalendar => 12,
            HomeState::ShowBookmarks => 3,
            HomeState::ShowCategoryItems => 4,
            HomeState::ShowPlaceholder => 5,
            HomeState::ShowNetworkStatus => 6,
            HomeState::ShowWifiConnect => 7,
            HomeState::ShowDateTime => 8,
            HomeState::ShowSleepImageInfo => 13,
            HomeState::ShowDeviceInfo => 14,
            HomeState::ShowCalendar => 9,
            HomeState::ShowPanchangLite => 10,
            HomeState::ShowLuaGameStub => 32,
            HomeState::ShowLuaToolStub => 33,
            HomeState::ShowLuaDictionary => 34,
            HomeState::ShowLuaFlashcards => 35,
            HomeState::ShowLuaPanchang => 10,
        }
    }

    #[inline]
    pub fn selected(&self) -> usize {
        self.selected
    }

    #[inline]
    pub fn bm_selected(&self) -> usize {
        self.bm_selected
    }

    #[inline]
    pub fn bm_scroll(&self) -> usize {
        self.bm_scroll
    }

    // restore home state from RTC session data
    pub fn restore_state(
        &mut self,
        state_id: u8,
        selected: usize,
        bm_selected: usize,
        bm_scroll: usize,
    ) {
        self.state = match state_id {
            1 => HomeState::ShowApps,
            2 => HomeState::ShowDailyMantra,
            11 => HomeState::ShowLuaDailyMantra,
            12 => HomeState::ShowLuaCalendar,
            35 => HomeState::ShowLuaFlashcards,
            3 => HomeState::ShowBookmarks,
            4 => HomeState::ShowCategoryItems,
            5 => HomeState::ShowPlaceholder,
            6 => HomeState::ShowNetworkStatus,
            7 => HomeState::ShowWifiConnect,
            8 => HomeState::ShowDateTime,
            13 => HomeState::ShowSleepImageInfo,
            14 => HomeState::ShowDeviceInfo,
            9 => HomeState::ShowCalendar,
            10 => HomeState::ShowLuaPanchang,
            _ => HomeState::Menu,
        };
        self.selected = selected.min(HOME_CARD_COUNT - 1);
        self.active_category = self
            .active_category
            .min(Self::category_count().saturating_sub(1));
        self.bm_selected = bm_selected;
        self.bm_scroll = bm_scroll;
        if self.state == HomeState::ShowBookmarks {
            self.reader_tab = 3;
            self.reader_focus_tabs = true;
            self.needs_load_bookmarks = true;
        }
        if self.state == HomeState::ShowNetworkStatus || self.state == HomeState::ShowWifiConnect {
            self.network_status_loaded = false;
            self.needs_load_network_status = true;
        }
        if matches!(
            self.state,
            HomeState::ShowCalendar
                | HomeState::ShowPanchangLite
                | HomeState::ShowDailyMantra
                | HomeState::ShowDateTime
                | HomeState::ShowNetworkStatus
        ) {
            self.time_status_loaded = false;
            self.needs_load_time_status = true;
        }
        if self.state == HomeState::ShowLuaDailyMantra {
            self.needs_load_lua_daily_mantra = true;
        }
        if self.state == HomeState::ShowLuaCalendar {
            self.needs_load_lua_calendar = true;
        }
        if self.state == HomeState::ShowLuaPanchang {
            self.time_status_loaded = false;
            self.panchang_mantra_loaded = false;
            self.needs_load_time_status = true;
            self.needs_load_panchang_mantra = true;
            self.needs_load_lua_panchang = true;
        }
        log::info!(
            "home: restore_state state={:?} selected={}",
            self.state,
            selected
        );
    }

    fn clear_recent(&mut self) {
        self.recent_book_len = 0;
        self.recent_record = None;
    }

    fn set_recent_record(&mut self, rec: reader_state::RecentBookRecord) {
        let path = rec.open_path().trim();
        if path.is_empty() {
            self.clear_recent();
            return;
        }

        let bytes = path.as_bytes();
        let n = bytes.len().min(self.recent_book.len());
        self.recent_book[..n].copy_from_slice(&bytes[..n]);
        self.recent_book_len = n;
        self.recent_record = Some(rec);
    }

    fn load_recent_record_from_state(&mut self, k: &mut KernelHandle<'_>) -> bool {
        let mut buf = [0u8; 192];
        let size = match k.read_app_subdir_chunk(
            reader_state::STATE_DIR,
            reader_state::RECENT_RECORD_FILE,
            0,
            &mut buf,
        ) {
            Ok(n) if n > 0 => n,
            _ => return false,
        };

        let text = match core::str::from_utf8(&buf[..size]) {
            Ok(s) => s.trim(),
            Err(_) => return false,
        };

        let Some(rec) = reader_state::RecentBookRecord::decode_line(text) else {
            log::warn!("reader-state: ignored invalid typed recent record");
            return false;
        };

        if rec.source_path.trim().is_empty() {
            return false;
        }

        log::info!(
            "reader-slice: home loaded typed recent book_id={} path={}",
            rec.book_id.as_str(),
            rec.open_path()
        );
        self.set_recent_record(rec);
        true
    }

    fn load_recent_legacy(&mut self, k: &mut KernelHandle<'_>) -> bool {
        let mut buf = [0u8; RECENT_BUF_LEN];
        match k.read_app_data_start(RECENT_FILE, &mut buf) {
            Ok((_, n)) if n > 0 => {
                let text = match core::str::from_utf8(&buf[..n.min(buf.len())]) {
                    Ok(s) => s.trim(),
                    Err(_) => {
                        self.clear_recent();
                        return false;
                    }
                };

                if text.is_empty() || text.contains('|') {
                    // Continue must come from typed state. Legacy fallback is accepted
                    // only as a raw path, never as an encoded recent record
                    // leaked into UI/routing.
                    self.clear_recent();
                    return false;
                }

                let rec = reader_state::RecentBookRecord::from_path(text);
                log::info!(
                    "reader-state: upgraded legacy recent path to typed record book_id={} path={}",
                    rec.book_id.as_str(),
                    rec.source_path
                );
                self.set_recent_record(rec);
                true
            }
            _ => {
                self.clear_recent();
                false
            }
        }
    }

    pub fn load_recent(&mut self, k: &mut KernelHandle<'_>) {
        if !self.load_recent_record_from_state(k) && !self.load_recent_legacy(k) {
            self.clear_recent();
        }
        self.rebuild_item_count();
        self.refresh_menu_layout();
        self.needs_load_recent = false;
    }

    fn clear_network_ssid(&mut self) {
        self.network_ssid_len = 0;
    }

    fn set_network_ssid(&mut self, ssid: &str) {
        let bytes = ssid.as_bytes();
        let n = bytes.len().min(self.network_ssid.len());
        self.network_ssid[..n].copy_from_slice(&bytes[..n]);
        self.network_ssid_len = n;
    }

    fn network_ssid_str(&self) -> &str {
        core::str::from_utf8(&self.network_ssid[..self.network_ssid_len]).unwrap_or("")
    }

    fn clear_network_pass(&mut self) {
        self.network_pass_len = 0;
    }

    fn set_network_pass(&mut self, pass: &str) {
        let bytes = pass.as_bytes();
        let n = bytes.len().min(self.network_pass.len());
        self.network_pass[..n].copy_from_slice(&bytes[..n]);
        self.network_pass_len = n;
    }

    fn network_pass_str(&self) -> &str {
        core::str::from_utf8(&self.network_pass[..self.network_pass_len]).unwrap_or("")
    }

    fn current_wifi_profile_slot(&self) -> usize {
        (self.network_profile_slot as usize).min(config::WIFI_PROFILE_COUNT - 1)
    }

    fn current_wifi_profile_name(&self) -> &str {
        self.network_wifi
            .profile_name(self.current_wifi_profile_slot())
    }

    fn sync_wifi_editor_from_slot(&mut self) {
        let slot = self.current_wifi_profile_slot();

        let mut ssid_buf = [0u8; NETWORK_SSID_BUF_LEN];
        let ssid_len = {
            let bytes = self.network_wifi.profile_ssid(slot).as_bytes();
            let n = bytes.len().min(ssid_buf.len());
            ssid_buf[..n].copy_from_slice(&bytes[..n]);
            n
        };

        let mut pass_buf = [0u8; NETWORK_PASS_BUF_LEN];
        let pass_len = {
            let bytes = self.network_wifi.profile_password(slot).as_bytes();
            let n = bytes.len().min(pass_buf.len());
            pass_buf[..n].copy_from_slice(&bytes[..n]);
            n
        };

        self.network_ssid = ssid_buf;
        self.network_ssid_len = ssid_len;
        self.network_pass = pass_buf;
        self.network_pass_len = pass_len;
        self.network_default_slot = self.network_wifi.default_slot();
    }

    fn sync_wifi_editor_to_slot(&mut self) {
        let slot = self.current_wifi_profile_slot();

        let mut ssid_buf = [0u8; NETWORK_SSID_BUF_LEN];
        let ssid_len = self.network_ssid_len.min(ssid_buf.len());
        ssid_buf[..ssid_len].copy_from_slice(&self.network_ssid[..ssid_len]);

        let mut pass_buf = [0u8; NETWORK_PASS_BUF_LEN];
        let pass_len = self.network_pass_len.min(pass_buf.len());
        pass_buf[..pass_len].copy_from_slice(&self.network_pass[..pass_len]);

        let ssid_text = core::str::from_utf8(&ssid_buf[..ssid_len]).unwrap_or("");
        let pass_text = core::str::from_utf8(&pass_buf[..pass_len]).unwrap_or("");

        self.network_wifi
            .set_profile_credentials_from_str(slot, ssid_text, pass_text);
        self.network_wifi
            .set_default_slot(self.network_default_slot);
    }

    fn cycle_wifi_profile_slot(&mut self, delta: isize) {
        self.sync_wifi_editor_to_slot();
        self.network_profile_slot = (self.network_profile_slot as isize + delta)
            .rem_euclid(config::WIFI_PROFILE_COUNT as isize)
            as u8;
        self.sync_wifi_editor_from_slot();
        self.wifi_editor_message = "Profile changed; edit or set default";
    }

    fn set_current_wifi_profile_as_default(&mut self) {
        self.sync_wifi_editor_to_slot();
        self.network_default_slot = self.network_profile_slot;
        self.network_wifi
            .set_default_slot(self.network_default_slot);
        self.wifi_editor_message = "Default profile selected; Save to persist";
    }

    fn wifi_keyboard_layout(&self) -> u8 {
        match self.wifi_editor_mode {
            WIFI_EDITOR_MODE_KEYBOARD_UPPER => text_keyboard::LAYOUT_UPPER,
            WIFI_EDITOR_MODE_KEYBOARD_SYMBOLS => text_keyboard::LAYOUT_SYMBOLS,
            _ => text_keyboard::LAYOUT_LOWER,
        }
    }

    fn set_wifi_keyboard_layout(&mut self, layout: u8) {
        self.wifi_editor_mode = match text_keyboard::normalize_layout(layout) {
            text_keyboard::LAYOUT_UPPER => WIFI_EDITOR_MODE_KEYBOARD_UPPER,
            text_keyboard::LAYOUT_SYMBOLS => WIFI_EDITOR_MODE_KEYBOARD_SYMBOLS,
            _ => WIFI_EDITOR_MODE_KEYBOARD_LOWER,
        };
    }

    fn is_wifi_text_editing(&self) -> bool {
        self.wifi_editor_mode != WIFI_EDITOR_MODE_ROWS
    }

    fn enter_wifi_text_editor(&mut self) {
        if self.wifi_editor_field == 3 {
            // CrossPoint-style keyboard ownership: the active field buffer
            // is authoritative until Back/Done/Save. Do not allow Home
            // background reloads to pull an older password from SETTINGS.TXT.
            self.needs_load_network_status = false;
            self.network_status_loaded = true;
            self.sync_wifi_editor_to_slot();
            self.set_wifi_keyboard_layout(text_keyboard::LAYOUT_LOWER);
            self.wifi_editor_char_idx = text_keyboard::default_index();
            self.wifi_editor_message = "Keyboard active; Back returns to list";
        }
    }

    fn exit_wifi_text_editor(&mut self) {
        self.wifi_editor_mode = WIFI_EDITOR_MODE_ROWS;
        self.wifi_editor_message = "Select Save to write SETTINGS.TXT";
        self.wifi_editor_dirty = false;
    }

    fn move_wifi_editor_field(&mut self, delta: isize) {
        self.wifi_editor_field = (self.wifi_editor_field as isize + delta)
            .rem_euclid(WIFI_EDITOR_FIELD_COUNT as isize) as u8;
        self.wifi_editor_message = WIFI_EDITOR_SAVE_MSG;
    }

    fn move_wifi_keyboard_horizontal(&mut self, delta: isize) {
        self.wifi_editor_char_idx = text_keyboard::move_horizontal(
            self.wifi_keyboard_layout(),
            self.wifi_editor_char_idx,
            delta,
        );
        self.wifi_editor_message = "OK enters key; Back returns to list";
    }

    fn move_wifi_keyboard_vertical(&mut self, delta: isize) {
        self.wifi_editor_char_idx = text_keyboard::move_vertical(
            self.wifi_keyboard_layout(),
            self.wifi_editor_char_idx,
            delta,
        );
        self.wifi_editor_message = "OK enters key; Back returns to list";
    }

    fn switch_wifi_keyboard_layout(&mut self, layout: u8) {
        let (new_layout, new_idx) = text_keyboard::switch_layout(
            self.wifi_keyboard_layout(),
            layout,
            self.wifi_editor_char_idx,
        );
        self.set_wifi_keyboard_layout(new_layout);
        self.wifi_editor_char_idx = new_idx;
        self.wifi_editor_message = "Keyboard layout changed";
    }

    fn push_wifi_text_byte(&mut self, ch: u8) {
        match self.wifi_editor_field {
            3 if self.network_pass_len < self.network_pass.len() => {
                self.network_pass[self.network_pass_len] = ch;
                self.network_pass_len += 1;
                self.wifi_editor_message = "Password edited; Back then Save";
            }
            3 => {
                self.wifi_editor_message = "Field is full";
            }
            _ => {}
        }
    }

    fn append_wifi_editor_char(&mut self) {
        if !self.is_wifi_text_editing() {
            match self.wifi_editor_field {
                0 => self.cycle_wifi_profile_slot(1),
                1 => {
                    self.resume_wifi_after_scan = true;
                    self.wifi_editor_message = "Scanning SSIDs...";
                }
                2 => self.apply_selected_wifi_scan_result(),
                3 => self.enter_wifi_text_editor(),
                4 => self.set_current_wifi_profile_as_default(),
                _ => {
                    self.sync_wifi_editor_to_slot();
                    self.wifi_editor_save_pending = true;
                    self.wifi_editor_message = "Saving SETTINGS.TXT...";
                }
            }
            return;
        }

        match text_keyboard::activate(self.wifi_keyboard_layout(), self.wifi_editor_char_idx) {
            TextKeyboardAction::Insert(ch) => self.push_wifi_text_byte(ch),
            TextKeyboardAction::Space => self.push_wifi_text_byte(b' '),
            TextKeyboardAction::Delete => self.delete_wifi_editor_char(),
            TextKeyboardAction::Clear => self.clear_wifi_text_buffer(),
            TextKeyboardAction::ToggleCase => {
                if self.wifi_keyboard_layout() == text_keyboard::LAYOUT_UPPER {
                    self.switch_wifi_keyboard_layout(text_keyboard::LAYOUT_LOWER);
                } else {
                    self.switch_wifi_keyboard_layout(text_keyboard::LAYOUT_UPPER);
                }
            }
            TextKeyboardAction::ToggleSymbols => {
                if self.wifi_keyboard_layout() == text_keyboard::LAYOUT_SYMBOLS {
                    self.switch_wifi_keyboard_layout(text_keyboard::LAYOUT_LOWER);
                } else {
                    self.switch_wifi_keyboard_layout(text_keyboard::LAYOUT_SYMBOLS);
                }
            }
            TextKeyboardAction::Done => self.exit_wifi_text_editor(),
            TextKeyboardAction::None => {}
        }
    }

    fn delete_wifi_editor_char(&mut self) {
        if !self.is_wifi_text_editing() {
            self.wifi_editor_message = "Select field first";
            return;
        }

        match self.wifi_editor_field {
            3 if self.network_pass_len > 0 => {
                self.network_pass_len -= 1;
                self.wifi_editor_message = "Deleted password character";
            }
            3 => {
                self.wifi_editor_message = "Nothing to delete";
            }
            _ => {}
        }
    }

    fn clear_wifi_text_buffer(&mut self) {
        match self.wifi_editor_field {
            3 => {
                self.network_pass_len = 0;
                self.wifi_editor_message = "Password cleared";
            }
            _ => {}
        }
    }

    fn clear_wifi_editor_field(&mut self) {
        if self.is_wifi_text_editing() {
            self.clear_wifi_text_buffer();
            return;
        }

        match self.wifi_editor_field {
            0 => self.cycle_wifi_profile_slot(1),
            1 => self.wifi_editor_message = "Select starts scan mode",
            2 => self.apply_selected_wifi_scan_result(),
            3 => self.enter_wifi_text_editor(),
            4 => self.set_current_wifi_profile_as_default(),
            _ => {
                self.wifi_editor_message = "Select Save to write SETTINGS.TXT";
            }
        }
    }

    fn clear_wifi_scan_results(&mut self) {
        self.network_scan_results = [[0u8; WIFI_SCAN_SSID_BUF_LEN]; WIFI_SCAN_MAX_RESULTS];
        self.network_scan_lens = [0u8; WIFI_SCAN_MAX_RESULTS];
        self.network_scan_count = 0;
        self.network_scan_selected = 0;
    }

    fn wifi_scan_result_str(&self, idx: usize) -> &str {
        if idx >= self.network_scan_count {
            return "";
        }
        let n = self.network_scan_lens[idx] as usize;
        core::str::from_utf8(&self.network_scan_results[idx][..n]).unwrap_or("")
    }

    fn wifi_scan_selected_str(&self) -> &str {
        if self.network_scan_count == 0 {
            ""
        } else {
            self.wifi_scan_result_str(self.network_scan_selected.min(self.network_scan_count - 1))
        }
    }

    fn wifi_scan_contains(&self, ssid: &[u8]) -> bool {
        let mut idx = 0usize;
        while idx < self.network_scan_count {
            let n = self.network_scan_lens[idx] as usize;
            if &self.network_scan_results[idx][..n] == ssid {
                return true;
            }
            idx += 1;
        }
        false
    }

    fn load_wifi_scan_results(&mut self, k: &mut KernelHandle<'_>) {
        self.clear_wifi_scan_results();
        let mut buf = [0u8; 768];
        let Ok(n) = k.read_cache_chunk(wifi_scan::WIFI_SCAN_FILE, 0, &mut buf) else {
            return;
        };
        for raw in buf[..n].split(|&b| b == b'\n') {
            let line = Self::trim_ascii(raw);
            if line.is_empty() || line[0] == b'#' || self.wifi_scan_contains(line) {
                continue;
            }
            if self.network_scan_count >= WIFI_SCAN_MAX_RESULTS {
                break;
            }
            let idx = self.network_scan_count;
            let len = line.len().min(WIFI_SCAN_SSID_BUF_LEN);
            self.network_scan_results[idx][..len].copy_from_slice(&line[..len]);
            self.network_scan_lens[idx] = len as u8;
            self.network_scan_count += 1;
        }
        if self.network_scan_count > 0 {
            self.network_scan_selected =
                self.network_scan_selected.min(self.network_scan_count - 1);
        }
    }

    fn cycle_wifi_scan_result(&mut self, delta: isize) {
        if self.network_scan_count == 0 {
            self.wifi_editor_message = "Select Scan SSIDs first";
            return;
        }
        self.network_scan_selected = (self.network_scan_selected as isize + delta)
            .rem_euclid(self.network_scan_count as isize)
            as usize;
        self.wifi_editor_message = "Select applies scanned SSID";
    }

    fn apply_selected_wifi_scan_result(&mut self) {
        if self.network_scan_count == 0 {
            self.wifi_editor_message = "No scan results; select Scan SSIDs";
            return;
        }
        let idx = self.network_scan_selected.min(self.network_scan_count - 1);
        let n = self.network_scan_lens[idx] as usize;
        self.network_ssid = [0u8; NETWORK_SSID_BUF_LEN];
        let len = n.min(self.network_ssid.len());
        self.network_ssid[..len].copy_from_slice(&self.network_scan_results[idx][..len]);
        self.network_ssid_len = len;
        self.sync_wifi_editor_to_slot();
        self.wifi_editor_message = "SSID selected; enter password then Save";
    }

    fn save_wifi_settings(&mut self, k: &mut KernelHandle<'_>) -> bool {
        let mut read_buf = [0u8; NETWORK_STATUS_BUF_LEN];
        let mut settings = SystemSettings::defaults();
        let mut existing_wifi = WifiConfig::empty();

        if let Ok((_size, n)) = k.read_app_data_start(config::SETTINGS_FILE, &mut read_buf) {
            if n > 0 {
                config::parse_settings_txt(&read_buf[..n], &mut settings, &mut existing_wifi);
            }
        }

        settings.sanitize();
        self.sync_wifi_editor_to_slot();
        let wifi = self.network_wifi;

        let mut out = [0u8; NETWORK_STATUS_BUF_LEN];
        let len = write_settings_txt(&settings, &wifi, &mut out);
        match k.write_app_data(config::SETTINGS_FILE, &out[..len]) {
            Ok(_) => {
                self.wifi_editor_message = "Saved to /RUSTMIX/SETTINGS.TXT";
                true
            }
            Err(_) => {
                self.wifi_editor_message = "Save failed";
                false
            }
        }
    }

    fn load_network_status(&mut self, k: &mut KernelHandle<'_>) {
        self.network_sd_ok = k.sd_ok();
        self.network_battery_mv = k.battery_mv();
        self.network_uptime_secs = k.uptime_secs();
        if self.state == HomeState::ShowWifiConnect
            && (self.is_wifi_text_editing() || self.wifi_editor_dirty)
        {
            // Do not reload SETTINGS.TXT over an active/dirty keyboard session.
            self.network_status_loaded = true;
            self.network_wifi
                .set_default_slot(self.network_default_slot);
            self.sync_wifi_editor_to_slot();
            self.network_wifi_configured = self.network_wifi.has_credentials();
            self.network_wifi_password_saved = !self.network_wifi.password().is_empty();
            self.needs_load_network_status = false;
            return;
        }
        self.network_settings_found = false;
        self.network_wifi_configured = false;
        self.network_wifi_password_saved = false;
        let preserve_profile_slot = self
            .network_profile_slot
            .min((config::WIFI_PROFILE_COUNT - 1) as u8);
        let preserve_wifi_screen = self.state == HomeState::ShowWifiConnect;
        self.network_wifi = WifiConfig::empty();
        self.network_default_slot = 0;
        self.network_profile_slot = 0;
        self.clear_network_ssid();
        self.clear_network_pass();

        let mut buf = [0u8; NETWORK_STATUS_BUF_LEN];
        if let Ok((_size, n)) = k.read_app_data_start(config::SETTINGS_FILE, &mut buf) {
            if n > 0 {
                let mut settings = SystemSettings::defaults();
                let mut wifi = WifiConfig::empty();
                config::parse_settings_txt(&buf[..n], &mut settings, &mut wifi);
                self.network_settings_found = true;
                self.network_wifi = wifi;
                self.network_default_slot = wifi.default_slot();
                self.network_profile_slot = if preserve_wifi_screen {
                    preserve_profile_slot
                } else {
                    self.network_default_slot
                };
                self.sync_wifi_editor_from_slot();
                if wifi.has_credentials() {
                    self.network_wifi_configured = true;
                    self.network_wifi_password_saved = !wifi.password().is_empty();
                }
            }
        }

        self.load_wifi_scan_results(k);
        self.network_status_loaded = true;
        self.needs_load_network_status = false;
    }

    fn load_panchang_mantra(&mut self, k: &mut KernelHandle<'_>) {
        self.clear_panchang_mantra();
        self.panchang_mantra_loaded = true;
        self.needs_load_panchang_mantra = false;

        if !self.time_status_loaded {
            return;
        }

        let Some(panchang) = self.time_cache.display_panchang_lite(self.time_uptime_secs) else {
            return;
        };

        let Some(tithi_key) = Self::panchang_tithi_file_key(panchang.tithi) else {
            return;
        };

        let files = [
            PANCHANG_MANTRA_FILE,
            PANCHANG_MANTRA_ALT_FILE,
            PANCHANG_MANTRA_SHORT_FILE,
            PANCHANG_MANTRA_SHORT_UPPER_FILE,
            PANCHANG_MANTRA_SHORT_83_FILE,
        ];
        for name in files {
            match self.scan_panchang_mantra_file(
                k,
                name,
                panchang.month,
                panchang.paksha,
                tithi_key,
            ) {
                PanchangMantraScan::Found => {
                    self.panchang_mantra_file_found = true;
                    self.panchang_mantra_found = true;
                    return;
                }
                PanchangMantraScan::FileFound => {
                    self.panchang_mantra_file_found = true;
                }
                PanchangMantraScan::Missing => {}
            }
        }
    }

    fn scan_panchang_mantra_file(
        &mut self,
        k: &mut KernelHandle<'_>,
        name: &str,
        month: &str,
        paksha: &str,
        tithi_key: &str,
    ) -> PanchangMantraScan {
        let Ok(size) = k.file_size(name) else {
            return PanchangMantraScan::Missing;
        };

        let mut chunk = [0u8; PANCHANG_MANTRA_SCAN_BUF_LEN];
        let mut line = [0u8; PANCHANG_MANTRA_LINE_BUF_LEN];
        let mut line_len = 0usize;
        let mut offset = 0u32;

        while offset < size {
            let Ok(n) = k.read_chunk(name, offset, &mut chunk) else {
                break;
            };
            if n == 0 {
                break;
            }

            for &b in &chunk[..n] {
                if b == b'\n' {
                    if Self::capture_panchang_mantra_line(
                        &line[..line_len],
                        month,
                        paksha,
                        tithi_key,
                        &mut self.panchang_mantra,
                        &mut self.panchang_mantra_len,
                    ) {
                        return PanchangMantraScan::Found;
                    }
                    line_len = 0;
                } else if line_len < line.len() {
                    line[line_len] = b;
                    line_len += 1;
                }
            }

            offset = offset.saturating_add(n as u32);
            if n < chunk.len() {
                break;
            }
        }

        if line_len > 0
            && Self::capture_panchang_mantra_line(
                &line[..line_len],
                month,
                paksha,
                tithi_key,
                &mut self.panchang_mantra,
                &mut self.panchang_mantra_len,
            )
        {
            return PanchangMantraScan::Found;
        }

        PanchangMantraScan::FileFound
    }

    fn capture_panchang_mantra_line(
        line: &[u8],
        month: &str,
        paksha: &str,
        tithi_key: &str,
        out: &mut [u8; PANCHANG_MANTRA_BUF_LEN],
        out_len: &mut usize,
    ) -> bool {
        let Some(month_field) = Self::panchang_field(line, 0) else {
            return false;
        };
        let Some(paksha_field) = Self::panchang_field(line, 1) else {
            return false;
        };
        let Some(tithi_field) = Self::panchang_field(line, 2) else {
            return false;
        };
        let Some(english_field) = Self::panchang_field(line, 5) else {
            return false;
        };

        if !Self::panchang_month_matches(month_field, month) {
            return false;
        }
        if !Self::panchang_paksha_matches(paksha_field, paksha) {
            return false;
        }
        if !Self::panchang_tithi_matches(tithi_field, tithi_key) {
            return false;
        }

        *out_len = Self::copy_panchang_ascii(english_field, out);
        *out_len > 0
    }

    fn panchang_field(line: &[u8], index: usize) -> Option<&[u8]> {
        let mut start = 0usize;
        let mut field = 0usize;

        for (pos, &b) in line.iter().enumerate() {
            if b == b'|' {
                if field == index {
                    return Some(Self::trim_ascii(&line[start..pos]));
                }
                field += 1;
                start = pos + 1;
            }
        }

        if field == index {
            return Some(Self::trim_ascii(&line[start..]));
        }

        None
    }

    fn trim_ascii(mut input: &[u8]) -> &[u8] {
        while let Some((&first, rest)) = input.split_first() {
            if first == b' ' || first == b'\t' || first == b'\r' {
                input = rest;
            } else {
                break;
            }
        }

        while let Some((&last, rest)) = input.split_last() {
            if last == b' ' || last == b'\t' || last == b'\r' {
                input = rest;
            } else {
                break;
            }
        }

        input
    }

    fn panchang_month_matches(field: &[u8], month: &str) -> bool {
        Self::ascii_eq_ignore_case(field, month.as_bytes())
            || match month {
                "Vaishakha" => Self::ascii_eq_ignore_case(field, b"Vaisakha"),
                "Jyeshtha" => Self::ascii_eq_ignore_case(field, b"Jyaistha"),
                "Ashwin" => Self::ascii_eq_ignore_case(field, b"Ashvina"),
                _ => false,
            }
    }

    fn panchang_paksha_matches(field: &[u8], paksha: &str) -> bool {
        let trimmed = Self::trim_ascii(field);
        let paksha = paksha.as_bytes();
        trimmed.len() >= paksha.len()
            && Self::ascii_eq_ignore_case(&trimmed[..paksha.len()], paksha)
    }

    fn panchang_tithi_matches(field: &[u8], key: &str) -> bool {
        let value = if let Some(colon) = field.iter().position(|&b| b == b':') {
            Self::trim_ascii(&field[colon + 1..])
        } else {
            Self::trim_ascii(field)
        };
        Self::ascii_eq_ignore_case(value, key.as_bytes())
    }

    fn ascii_eq_ignore_case(left: &[u8], right: &[u8]) -> bool {
        if left.len() != right.len() {
            return false;
        }

        left.iter()
            .zip(right.iter())
            .all(|(&a, &b)| Self::ascii_lower(a) == Self::ascii_lower(b))
    }

    fn ascii_lower(b: u8) -> u8 {
        if b.is_ascii_uppercase() { b + 32 } else { b }
    }

    fn panchang_tithi_file_key(tithi: &str) -> Option<&'static str> {
        match tithi {
            "Pratipada" => Some("1"),
            "Dwitiya" => Some("2"),
            "Tritiya" => Some("3"),
            "Chaturthi" => Some("4"),
            "Panchami" => Some("5"),
            "Shashthi" => Some("6"),
            "Saptami" => Some("7"),
            "Ashtami" => Some("8"),
            "Navami" => Some("9"),
            "Dashami" => Some("10"),
            "Ekadashi" => Some("11"),
            "Dwadashi" => Some("12"),
            "Trayodashi" => Some("13"),
            "Chaturdashi" => Some("14"),
            "Purnima" => Some("Purnima"),
            "Amavasya" => Some("Amavasya"),
            _ => None,
        }
    }

    fn copy_panchang_ascii(src: &[u8], out: &mut [u8; PANCHANG_MANTRA_BUF_LEN]) -> usize {
        let src = Self::trim_ascii(src);
        let mut len = 0usize;

        for &b in src {
            if len >= out.len().saturating_sub(1) {
                break;
            }

            if b == b'\r' || b == b'\n' {
                break;
            }

            if b.is_ascii() && !b.is_ascii_control() {
                out[len] = b;
                len += 1;
            }
        }

        len
    }

    fn clear_panchang_mantra(&mut self) {
        self.panchang_mantra_len = 0;
        self.panchang_mantra_found = false;
        self.panchang_mantra_file_found = false;
        self.panchang_mantra.fill(0);
    }

    fn panchang_mantra_status(&self) -> &str {
        if self.panchang_mantra_found {
            return self.panchang_mantra_str().unwrap_or("--");
        }

        if !self.panchang_mantra_loaded {
            "Loading mantra from SD"
        } else if !self.panchang_mantra_file_found {
            "Mantra file not found: try Yearly_h.txt"
        } else {
            "No matching mantra in file"
        }
    }

    fn panchang_mantra_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.panchang_mantra[..self.panchang_mantra_len])
            .ok()
            .filter(|s| !s.trim().is_empty())
    }

    fn lua_daily_mantra_day_from_time(&self) -> Option<&'static str> {
        match self
            .time_cache
            .display_weekday_index(self.time_uptime_secs)?
        {
            0 => Some("Sunday"),
            1 => Some("Monday"),
            2 => Some("Tuesday"),
            3 => Some("Wednesday"),
            4 => Some("Thursday"),
            5 => Some("Friday"),
            6 => Some("Saturday"),
            _ => None,
        }
    }

    fn lua_daily_mantra_status_label(&self) -> &'static str {
        if self.lua_daily_mantra_screen.source.is_sd_loaded() {
            self.time_cache.freshness(self.time_uptime_secs).as_str()
        } else {
            self.lua_daily_mantra_screen.source.label()
        }
    }

    fn daily_quote_date_index(&self) -> Option<usize> {
        let date = self.time_cache.display_date(self.time_uptime_secs)?;
        let mut ordinal = 0usize;
        let mut month = 1u8;
        while month < date.month {
            ordinal += usize::from(time_status::calendar_days_in_month(date.year, month));
            month = month.saturating_add(1);
        }
        ordinal += usize::from(date.day.saturating_sub(1));
        Some(ordinal)
    }

    fn daily_quote_date_label(&self) -> Option<String> {
        let mut label = String::new();
        self.time_cache
            .write_date_value(self.time_uptime_secs, &mut label)
            .ok()?;
        if label.as_str() == "--" {
            None
        } else {
            Some(label)
        }
    }

    fn load_lua_daily_mantra(&mut self, k: &mut KernelHandle<'_>) {
        let selected_day = self.lua_daily_mantra_day_from_time();
        let mut manifest_buf = [0u8; 1024];
        let mut script_buf = [0u8; 4096];
        let mut mantras_buf = [0u8; 2048];

        self.lua_daily_mantra_screen = match k.read_lua_app_file_start(
            LUA_DAILY_MANTRA_APP_FOLDER,
            LUA_DAILY_MANTRA_MANIFEST_FILE,
            &mut manifest_buf,
        ) {
            Ok((_size, manifest_n)) if manifest_n > 0 => {
                let manifest = match core::str::from_utf8(&manifest_buf[..manifest_n]) {
                    Ok(text) => text,
                    Err(_) => {
                        self.needs_load_lua_daily_mantra = false;
                        self.lua_daily_mantra_screen =
                            LuaDailyMantraScreen::invalid_manifest_utf8();
                        log::info!(
                            "lua-app marker={} app=daily_mantra source={}",
                            LUA_DAILY_MANTRA_APP_MARKER,
                            self.lua_daily_mantra_screen.source.label()
                        );
                        return;
                    }
                };

                match k.read_lua_app_file_start(
                    LUA_DAILY_MANTRA_APP_FOLDER,
                    LUA_DAILY_MANTRA_ENTRY_FILE,
                    &mut script_buf,
                ) {
                    Ok((_size, script_n)) if script_n > 0 => {
                        let script = match core::str::from_utf8(&script_buf[..script_n]) {
                            Ok(text) => text,
                            Err(_) => {
                                self.needs_load_lua_daily_mantra = false;
                                self.lua_daily_mantra_screen =
                                    LuaDailyMantraScreen::invalid_script_utf8();
                                log::info!(
                                    "lua-app marker={} app=daily_mantra source={}",
                                    LUA_DAILY_MANTRA_APP_MARKER,
                                    self.lua_daily_mantra_screen.source.label()
                                );
                                return;
                            }
                        };

                        match k.read_lua_app_file_start(
                            LUA_DAILY_MANTRA_APP_FOLDER,
                            LUA_DAILY_MANTRA_MANTRAS_FILE,
                            &mut mantras_buf,
                        ) {
                            Ok((_size, mantras_n)) if mantras_n > 0 => {
                                match core::str::from_utf8(&mantras_buf[..mantras_n]) {
                                    Ok(mantras) => {
                                        #[cfg(feature = "lua-vm")]
                                        {
                                            crate::rustmix_x4::lua::daily_mantra_vm_bridge::build_daily_mantra_vm_sd_runtime_for_day(
                                                    manifest, script, mantras, selected_day,
                                                )
                                        }
                                        #[cfg(not(feature = "lua-vm"))]
                                        {
                                            crate::rustmix_x4::lua::daily_mantra_script::build_daily_mantra_sd_runtime_for_day(
                                                    manifest, script, mantras, selected_day,
                                                )
                                        }
                                    }
                                    Err(_) => LuaDailyMantraScreen::invalid_mantras_utf8(),
                                }
                            }
                            _ => LuaDailyMantraScreen::missing_mantras(),
                        }
                    }
                    _ => LuaDailyMantraScreen::missing_entry(),
                }
            }
            _ => LuaDailyMantraScreen::missing_manifest(),
        };
        self.needs_load_lua_daily_mantra = false;
        log::info!(
            "lua-app marker={} sd_runtime_marker={} app=daily_mantra source={}",
            LUA_DAILY_MANTRA_APP_MARKER,
            LUA_DAILY_MANTRA_SD_RUNTIME_MARKER,
            self.lua_daily_mantra_screen.source.label()
        );
    }

    fn load_lua_panchang(&mut self, k: &mut KernelHandle<'_>) {
        let mut manifest_buf = [0u8; 1024];
        let mut script_buf = [0u8; 4096];
        let mut data_buf = [0u8; 2048];

        self.lua_panchang_screen = match k.read_lua_app_file_start(
            LUA_PANCHANG_APP_FOLDER,
            LUA_PANCHANG_MANIFEST_FILE,
            &mut manifest_buf,
        ) {
            Ok((_size, manifest_n)) if manifest_n > 0 => {
                let manifest = match core::str::from_utf8(&manifest_buf[..manifest_n]) {
                    Ok(text) => text,
                    Err(_) => {
                        self.lua_panchang_screen = LuaPanchangScreen::invalid_manifest_utf8();
                        self.needs_load_lua_panchang = false;
                        return;
                    }
                };

                match k.read_lua_app_file_start(
                    LUA_PANCHANG_APP_FOLDER,
                    LUA_PANCHANG_ENTRY_FILE,
                    &mut script_buf,
                ) {
                    Ok((_size, script_n)) if script_n > 0 => {
                        let script = match core::str::from_utf8(&script_buf[..script_n]) {
                            Ok(text) => text,
                            Err(_) => {
                                self.lua_panchang_screen = LuaPanchangScreen::invalid_script_utf8();
                                self.needs_load_lua_panchang = false;
                                return;
                            }
                        };

                        match k.read_lua_panchang_y2026_start(&mut data_buf) {
                            Ok((_size, data_n)) if data_n > 0 => {
                                match core::str::from_utf8(&data_buf[..data_n]) {
                                    Ok(data) => build_panchang_sd_runtime(manifest, script, data),
                                    Err(_) => LuaPanchangScreen::invalid_data_utf8(),
                                }
                            }
                            _ => {
                                // Compatibility fallback for emergency/manual recovery: if the
                                // nested DATA folder cannot be opened, allow a flat app-root copy
                                // at `/RUSTMIX/APPS/PANCHANG/Y2026.TXT`. The canonical contract
                                // remains `/RUSTMIX/APPS/PANCHANG/DATA/Y2026.TXT`.
                                match k.read_lua_app_file_start(
                                    LUA_PANCHANG_APP_FOLDER,
                                    LUA_PANCHANG_DATA_FILE,
                                    &mut data_buf,
                                ) {
                                    Ok((_size, data_n)) if data_n > 0 => {
                                        match core::str::from_utf8(&data_buf[..data_n]) {
                                            Ok(data) => {
                                                build_panchang_sd_runtime(manifest, script, data)
                                            }
                                            Err(_) => LuaPanchangScreen::invalid_data_utf8(),
                                        }
                                    }
                                    _ => LuaPanchangScreen::missing_data(),
                                }
                            }
                        }
                    }
                    _ => LuaPanchangScreen::missing_entry(),
                }
            }
            _ => LuaPanchangScreen::missing_manifest(),
        };
        self.needs_load_lua_panchang = false;
        self.apply_lua_panchang_lite_context();
        log::info!(
            "lua-app marker={} sd_runtime_marker={} app=panchang source={}",
            LUA_PANCHANG_APP_MARKER,
            LUA_PANCHANG_SD_RUNTIME_MARKER,
            self.lua_panchang_screen.source.label()
        );
    }

    fn apply_lua_panchang_lite_context(&mut self) {
        self.lua_panchang_screen.title.set("Panchang");

        if !self.time_status_loaded {
            self.lua_panchang_screen
                .subtitle
                .set("Date & Time cache loading");
            self.lua_panchang_screen
                .line1
                .set("Reading cached Date & Time...");
            self.lua_panchang_screen
                .line2
                .set("Panchang opens after cache read");
            self.lua_panchang_screen
                .line3
                .set("Back returns to Productivity");
            self.lua_panchang_screen
                .footer
                .set("Lua Panchang + cached Date & Time");
            return;
        }

        let Some(panchang) = self.time_cache.display_panchang_lite(self.time_uptime_secs) else {
            self.lua_panchang_screen
                .subtitle
                .set("Date & Time not synced");
            self.lua_panchang_screen.line1.set("Sync Date & Time first");
            self.lua_panchang_screen
                .line2
                .set("Panchang works offline after one successful sync");
            self.lua_panchang_screen
                .line3
                .set("Back returns to Productivity");
            self.lua_panchang_screen
                .footer
                .set("No network API used by Panchang");
            return;
        };

        let source = self.lua_panchang_screen.source.label();

        self.lua_panchang_screen.subtitle.set(panchang.weekday);
        self.lua_panchang_screen.subtitle.push_str(" - ");
        self.lua_panchang_screen
            .subtitle
            .push_str(panchang.location);

        self.lua_panchang_screen.line1.set("Tithi: ");
        self.lua_panchang_screen.line1.push_str(panchang.tithi);

        self.lua_panchang_screen.line2.set("Paksha: ");
        self.lua_panchang_screen.line2.push_str(panchang.paksha);
        self.lua_panchang_screen.line2.push_str("  Month: ");
        self.lua_panchang_screen.line2.push_str(panchang.month);

        let mantra = String::from(self.panchang_mantra_status());
        if mantra.is_empty() || mantra == "--" {
            self.lua_panchang_screen.line3.set(panchang.note);
        } else {
            self.lua_panchang_screen.line3.set("Mantra: ");
            self.lua_panchang_screen.line3.push_str(mantra.as_str());
        }

        self.lua_panchang_screen.footer.set("Source: ");
        self.lua_panchang_screen.footer.push_str(source);
        self.lua_panchang_screen
            .footer
            .push_str(" + cached Date & Time");
    }

    fn load_lua_calendar(&mut self, k: &mut KernelHandle<'_>) {
        let mut manifest_buf = [0u8; 1024];
        let mut script_buf = [0u8; 4096];
        let mut data_buf = [0u8; 4096];
        let mut combined_events = String::new();

        self.lua_calendar_screen = match k.read_lua_app_file_start(
            LUA_CALENDAR_APP_FOLDER,
            LUA_CALENDAR_MANIFEST_FILE,
            &mut manifest_buf,
        ) {
            Ok((_size, manifest_n)) if manifest_n > 0 => {
                let manifest = match core::str::from_utf8(&manifest_buf[..manifest_n]) {
                    Ok(text) => text,
                    Err(_) => {
                        self.needs_load_lua_calendar = false;
                        self.lua_calendar_screen = LuaCalendarScreen::invalid_manifest_utf8();
                        log::info!(
                            "lua-app marker={} app=calendar source={}",
                            LUA_CALENDAR_APP_MARKER,
                            self.lua_calendar_screen.source.label()
                        );
                        return;
                    }
                };

                match k.read_lua_app_file_start(
                    LUA_CALENDAR_APP_FOLDER,
                    LUA_CALENDAR_ENTRY_FILE,
                    &mut script_buf,
                ) {
                    Ok((_size, script_n)) if script_n > 0 => {
                        let script = match core::str::from_utf8(&script_buf[..script_n]) {
                            Ok(text) => text,
                            Err(_) => {
                                self.needs_load_lua_calendar = false;
                                self.lua_calendar_screen = LuaCalendarScreen::invalid_script_utf8();
                                log::info!(
                                    "lua-app marker={} app=calendar source={}",
                                    LUA_CALENDAR_APP_MARKER,
                                    self.lua_calendar_screen.source.label()
                                );
                                return;
                            }
                        };

                        for name in [
                            LUA_CALENDAR_EVENTS_FILE,
                            LUA_CALENDAR_AMERICAN_EVENTS_FILE,
                            LUA_CALENDAR_HINDU_EVENTS_FILE,
                        ] {
                            match k.read_lua_app_file_start(
                                LUA_CALENDAR_APP_FOLDER,
                                name,
                                &mut data_buf,
                            ) {
                                Ok((_size, data_n)) if data_n > 0 => {
                                    match core::str::from_utf8(&data_buf[..data_n]) {
                                        Ok(text) => {
                                            if !combined_events.is_empty() {
                                                combined_events.push('\n');
                                            }
                                            combined_events.push_str(text);
                                        }
                                        Err(_) => {
                                            self.needs_load_lua_calendar = false;
                                            self.lua_calendar_screen =
                                                LuaCalendarScreen::invalid_events_utf8();
                                            log::info!(
                                                "lua-app marker={} app=calendar source={}",
                                                LUA_CALENDAR_APP_MARKER,
                                                self.lua_calendar_screen.source.label()
                                            );
                                            return;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }

                        if combined_events.is_empty() {
                            LuaCalendarScreen::missing_events()
                        } else {
                            let today = if self.time_status_loaded {
                                self.time_cache
                                    .display_date(self.time_uptime_secs)
                                    .map(|date| (date.year, date.month, date.day))
                            } else {
                                None
                            };
                            build_combined_calendar_sd_runtime(
                                manifest,
                                script,
                                combined_events.as_str(),
                                today,
                            )
                        }
                    }
                    _ => LuaCalendarScreen::missing_entry(),
                }
            }
            _ => LuaCalendarScreen::missing_manifest(),
        };
        self.needs_load_lua_calendar = false;
        log::info!(
            "lua-app marker={} sd_runtime_marker={} app=calendar source={}",
            LUA_CALENDAR_APP_MARKER,
            LUA_CALENDAR_SD_RUNTIME_MARKER,
            self.lua_calendar_screen.source.label()
        );
    }

    fn load_time_status(&mut self, k: &mut KernelHandle<'_>) {
        self.time_uptime_secs = k.uptime_secs();
        self.home_battery_mv = k.battery_mv();

        let mut buf = [0u8; TIME_STATUS_BUF_LEN];
        self.time_cache = match k.read_app_data_start(time_status::TIME_FILE, &mut buf) {
            Ok((_size, n)) if n > 0 => time_status::parse_time_txt(&buf[..n]),
            _ => time_status::TimeCache::default(),
        };
        self.time_status_loaded = true;
        self.needs_load_time_status = false;
    }

    fn rebuild_item_count(&mut self) {
        self.item_count = HOME_CARD_COUNT;
        if self.selected >= self.item_count {
            self.selected = 0;
        }
    }

    fn has_recent(&self) -> bool {
        self.recent_record.is_some() && self.recent_book_len > 0
    }

    pub fn recent_book_bytes(&self) -> Option<&[u8]> {
        if self.has_recent() {
            Some(&self.recent_book[..self.recent_book_len])
        } else {
            None
        }
    }

    pub fn recent_book_str(&self) -> Option<&str> {
        self.recent_book_bytes()
            .and_then(|bytes| core::str::from_utf8(bytes).ok())
            .filter(|s| !s.trim().is_empty())
    }

    pub fn recent_record(&self) -> Option<reader_state::RecentBookRecord> {
        self.recent_record.clone()
    }

    pub fn recent_book_id(&self) -> Option<String> {
        self.recent_record().map(|rec| rec.book_id.0)
    }

    pub fn recent_source_path(&self) -> Option<String> {
        self.recent_record().map(|rec| rec.source_path)
    }

    fn basename(path: &str) -> &str {
        path.rsplit('/').next().unwrap_or(path)
    }

    pub fn shell_menu_items(&self) -> Vec<HomeMenuItem> {
        let mut items = Vec::with_capacity(self.item_count);
        items.push(HomeMenuItem::ContinueReading);
        items.push(HomeMenuItem::FileBrowser);
        items.push(HomeMenuItem::Bookmarks);
        items.push(HomeMenuItem::Settings);
        items.push(HomeMenuItem::Sync);
        items.push(HomeMenuItem::Upload);
        items
    }

    fn item_action(&self, idx: usize) -> MenuAction {
        MenuAction::OpenCategory(idx.min(Self::category_count().saturating_sub(1)))
    }

    fn set_selection(&mut self, selected: usize, ctx: &mut AppContext) {
        let count = self.item_count;
        if count == 0 {
            return;
        }
        let new = selected.min(count - 1);
        if new != self.selected {
            ctx.mark_dirty(self.item_regions[self.selected]);
            self.selected = new;
            ctx.mark_dirty(self.item_regions[self.selected]);
        }
    }

    fn move_selection_row(&mut self, delta: isize, ctx: &mut AppContext) {
        let col = self.selected % HOME_GRID_COLS;
        let row = self.selected / HOME_GRID_COLS;
        let next_row = if delta > 0 {
            (row + 1).min(HOME_GRID_ROWS - 1)
        } else {
            row.saturating_sub(1)
        };
        self.set_selection(next_row * HOME_GRID_COLS + col, ctx);
    }

    fn move_selection_col(&mut self, delta: isize, ctx: &mut AppContext) {
        let col = self.selected % HOME_GRID_COLS;
        let row = self.selected / HOME_GRID_COLS;
        let next_col = if delta > 0 {
            (col + 1).min(HOME_GRID_COLS - 1)
        } else {
            col.saturating_sub(1)
        };
        self.set_selection(row * HOME_GRID_COLS + next_col, ctx);
    }

    #[inline]
    fn header_font(&self) -> &'static BitmapFont {
        fonts::heading_font(0)
    }

    #[inline]
    fn card_title_font(&self) -> &'static BitmapFont {
        fonts::body_font(1)
    }

    #[inline]
    fn card_meta_font(&self) -> &'static BitmapFont {
        fonts::chrome_font()
    }

    fn bm_group_title(&self) -> Option<&str> {
        let first = self.bm_entries.first()?;
        let first_id = first.book_id.as_str();

        if self
            .bm_entries
            .iter()
            .all(|entry| entry.book_id.as_str() == first_id)
        {
            let title = first.display_title.trim();
            if !title.is_empty() {
                Some(title)
            } else {
                Some(Self::basename(&first.source_path))
            }
        } else {
            None
        }
    }

    fn ellipsize_ascii(s: &str, max_chars: usize) -> String {
        let trimmed = s.trim();
        let mut out = String::new();
        let mut chars = trimmed.chars();

        for _ in 0..max_chars {
            match chars.next() {
                Some(ch) => out.push(ch),
                None => return out,
            }
        }

        if chars.next().is_some() && max_chars > 3 {
            for _ in 0..3 {
                out.pop();
            }
            out.push_str("...");
        }

        out
    }

    fn bm_detail_label(entry: &reader_state::BookmarkIndexRecord) -> String {
        let detail = entry.label.trim();
        if !detail.is_empty() {
            return Self::ellipsize_ascii(detail, BM_MAX_ROW_CHARS);
        }

        let mut out = String::new();
        let _ = write!(out, "Ch {}", u32::from(entry.chapter) + 1);
        if entry.byte_offset > 0 {
            let _ = write!(out, " · Off {}", entry.byte_offset);
        }
        out
    }

    fn bm_mixed_book_label(entry: &reader_state::BookmarkIndexRecord) -> String {
        let title = if !entry.display_title.trim().is_empty() {
            entry.display_title.trim()
        } else {
            Self::basename(&entry.source_path)
        };

        let mut out = Self::ellipsize_ascii(title, BM_MAX_MIXED_TITLE_CHARS);
        out.push_str(" · ");
        out.push_str(&Self::bm_detail_label(entry));
        Self::ellipsize_ascii(&out, BM_MAX_ROW_CHARS)
    }

    fn bm_subtitle_region(&self) -> Region {
        crossink_internal::reader_list_row_region_from(
            crossink_internal::READER_LIST_TOP_WITH_TABS,
            0,
        )
    }

    fn bm_list_y(&self) -> u16 {
        if self.bm_group_title().is_some() {
            crossink_internal::READER_LIST_TOP_WITH_TABS + crossink_internal::READER_ROW_STRIDE
        } else {
            crossink_internal::READER_LIST_TOP_WITH_TABS
        }
    }

    fn bm_visible_lines(&self) -> usize {
        crossink_internal::reader_visible_rows_from(self.bm_list_y())
            .max(1)
            .min(64)
    }

    fn bm_row_region(&self, i: usize) -> Region {
        crossink_internal::reader_list_row_region_from(self.bm_list_y(), i)
    }

    fn bm_list_region(&self) -> Region {
        let vis = self.bm_visible_lines();
        crossink_internal::reader_list_region_from(
            crossink_internal::READER_LIST_TOP_WITH_TABS,
            vis + 1,
        )
    }

    fn bm_status_region(&self) -> Region {
        crossink_internal::header_status_region()
    }

    fn reader_tab_has_home_rows(&self) -> bool {
        match self.reader_tab {
            0 => self.recent_record.is_some(),
            3 => self.bm_count > 0,
            _ => false,
        }
    }

    fn push_files_reader_tab(tab: usize, ctx: &mut AppContext) -> Transition {
        let msg = match tab {
            1 => "reader-tab:1",
            2 => "reader-tab:2",
            _ => "reader-tab:2",
        };
        ctx.set_message(msg.as_bytes());
        Transition::Push(AppId::Files)
    }
}

impl App<AppId> for HomeApp {
    fn on_enter(&mut self, ctx: &mut AppContext, _k: &mut KernelHandle<'_>) {
        ctx.clear_message();
        self.state = HomeState::Menu;
        self.selected = 0;
        self.needs_load_recent = true;
        self.needs_load_bookmarks = true;
        self.needs_load_time_status = true;
        ctx.mark_dirty(CONTENT_REGION);
    }

    fn on_resume(&mut self, ctx: &mut AppContext, _k: &mut KernelHandle<'_>) {
        if self.resume_wifi_after_scan {
            self.resume_wifi_after_scan = false;
            self.state = HomeState::ShowWifiConnect;
            self.return_target = ReturnTarget::CategoryItems;
            self.network_status_loaded = false;
            self.needs_load_network_status = true;
            self.wifi_editor_field = 2;
            self.wifi_editor_message = "Scan complete; choose SSID then enter password";
        } else if self.resume_date_time_after_sync {
            self.resume_date_time_after_sync = false;
            self.state = HomeState::ShowDateTime;
            self.return_target = ReturnTarget::CategoryItems;
        } else {
            self.state = HomeState::Menu;
            self.selected = 0;
        }
        self.needs_load_recent = true;
        self.needs_load_bookmarks = true;
        self.needs_load_time_status = true;
        ctx.mark_dirty(CONTENT_REGION);
    }

    async fn background(&mut self, ctx: &mut AppContext, k: &mut KernelHandle<'_>) {
        if self.state == HomeState::ShowWifiConnect && self.is_wifi_text_editing() {
            // CrossPoint-style keyboard session active: no Home background SD
            // reloads and no background-requested redraws while navigating the
            // on-screen keyboard. The in-memory SSID/password buffers remain
            // authoritative until the user exits or saves.
            self.needs_load_network_status = false;
            self.network_status_loaded = true;
            self.sync_wifi_editor_to_slot();
            let _ = ctx.take_redraw();
            return;
        }

        if self.needs_load_lua_game_stub {
            self.load_lua_game_stub(k);
            self.needs_load_lua_game_stub = false;
            if self.state == HomeState::ShowLuaGameStub {
                ctx.request_full_redraw();
            }
        }

        if self.needs_load_lua_tool_stub {
            self.load_lua_tool_stub(k);
            self.needs_load_lua_tool_stub = false;
            if matches!(
                self.state,
                HomeState::ShowLuaToolStub | HomeState::ShowLuaDictionary
            ) {
                ctx.request_full_redraw();
            }
        }

        if self.needs_load_flashcards {
            self.load_lua_flashcards(k);
            self.needs_load_flashcards = false;
            if self.state == HomeState::ShowLuaFlashcards {
                ctx.request_full_redraw();
            }
        }

        if self.needs_load_recent {
            let old_count = self.item_count;
            if !self.load_recent_record_from_state(k) && !self.load_recent_legacy(k) {
                self.clear_recent();
            }
            self.rebuild_item_count();
            self.needs_load_recent = false;
            self.refresh_menu_layout();
            if self.item_count != old_count {
                ctx.request_full_redraw();
            } else if self.state == HomeState::Menu {
                ctx.mark_dirty(self.item_regions[0]);
            }
        }

        if self.needs_load_network_status {
            self.load_network_status(k);
            if self.state == HomeState::ShowNetworkStatus
                || self.state == HomeState::ShowWifiConnect
            {
                ctx.request_full_redraw();
            }
        }

        if self.wifi_editor_save_pending {
            self.wifi_editor_save_pending = false;
            self.wifi_editor_dirty = false;
            let _ = self.save_wifi_settings(k);
            self.load_network_status(k);
            if self.state == HomeState::ShowWifiConnect
                || self.state == HomeState::ShowNetworkStatus
            {
                ctx.request_full_redraw();
            }
        }

        if self.needs_load_time_status {
            self.load_time_status(k);
            if matches!(
                self.state,
                HomeState::Menu
                    | HomeState::ShowApps
                    | HomeState::ShowDailyMantra
                    | HomeState::ShowLuaDailyMantra
                    | HomeState::ShowCalendar
                    | HomeState::ShowLuaCalendar
                    | HomeState::ShowPanchangLite
                    | HomeState::ShowLuaPanchang
                    | HomeState::ShowLuaToolStub
                    | HomeState::ShowNetworkStatus
                    | HomeState::ShowDateTime
            ) {
                ctx.request_full_redraw();
            }
        }

        if self.needs_load_panchang_mantra {
            self.load_panchang_mantra(k);
            if matches!(
                self.state,
                HomeState::ShowPanchangLite | HomeState::ShowLuaPanchang
            ) {
                ctx.request_full_redraw();
            }
        }

        if self.needs_load_lua_daily_mantra {
            self.load_lua_daily_mantra(k);
            if self.state == HomeState::ShowLuaDailyMantra {
                ctx.request_full_redraw();
            }
        }

        if self.needs_load_lua_calendar {
            self.load_lua_calendar(k);
            if self.state == HomeState::ShowLuaCalendar {
                ctx.request_full_redraw();
            }
        }

        if self.needs_load_lua_panchang {
            self.load_lua_panchang(k);
            if self.state == HomeState::ShowLuaPanchang {
                ctx.request_full_redraw();
            }
        }

        if self.needs_load_bookmarks {
            let mut buf = [0u8; 4096];
            self.bm_entries.clear();

            if let Ok(n) = k.read_app_subdir_chunk(
                reader_state::STATE_DIR,
                reader_state::BOOKMARKS_INDEX_FILE,
                0,
                &mut buf,
            ) {
                if n > 0 {
                    if let Ok(payload) = core::str::from_utf8(&buf[..n]) {
                        self.bm_entries = reader_state::decode_bookmarks_index(payload);
                    }
                }
            }

            self.bm_count = self.bm_entries.len();
            log::info!(
                "bookmark-index: home loaded {} item(s) from global bookmark index",
                self.bm_count
            );
            if self.bm_count == 0 {
                self.bm_selected = 0;
                self.bm_scroll = 0;
            } else {
                if self.bm_selected >= self.bm_count {
                    self.bm_selected = self.bm_count - 1;
                }
                let vis = self.bm_visible_lines();
                if self.bm_scroll + vis <= self.bm_selected {
                    self.bm_scroll = self.bm_selected.saturating_sub(vis.saturating_sub(1));
                }
            }

            self.needs_load_bookmarks = false;
            if self.state == HomeState::ShowBookmarks {
                ctx.mark_dirty(self.bm_list_region());
                ctx.mark_dirty(self.bm_status_region());
            } else if self.state == HomeState::Menu {
                ctx.mark_dirty(self.item_regions[3]);
            }
        }
    }

    fn on_event(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match self.state {
            HomeState::Menu => self.on_event_menu(event, ctx),
            HomeState::ShowApps => self.on_event_apps(event, ctx),
            HomeState::ShowCategoryItems => self.on_event_category_items(event, ctx),
            HomeState::ShowDailyMantra => self.on_event_daily_mantra(event, ctx),
            HomeState::ShowLuaDailyMantra => self.on_event_lua_daily_mantra(event, ctx),
            HomeState::ShowLuaPanchang => self.on_event_lua_panchang(event, ctx),
            HomeState::ShowLuaCalendar => self.on_event_lua_calendar(event, ctx),
            HomeState::ShowLuaFlashcards => self.on_event_lua_flashcards(event, ctx),
            HomeState::ShowCalendar => self.on_event_calendar(event, ctx),
            HomeState::ShowPanchangLite => self.on_event_panchang_lite(event, ctx),
            HomeState::ShowBookmarks => self.on_event_bookmarks(event, ctx),
            HomeState::ShowNetworkStatus => self.on_event_network_status(event, ctx),
            HomeState::ShowWifiConnect => self.on_event_wifi_connect(event, ctx),
            HomeState::ShowDateTime => self.on_event_date_time(event, ctx),
            HomeState::ShowSleepImageInfo | HomeState::ShowDeviceInfo => {
                self.on_event_system_info(event, ctx)
            }
            HomeState::ShowPlaceholder => self.on_event_placeholder(event, ctx),
            HomeState::ShowLuaGameStub => self.on_event_lua_game_stub(event, ctx),
            HomeState::ShowLuaToolStub => self.on_event_lua_tool_stub(event, ctx),
            HomeState::ShowLuaDictionary => self.on_event_lua_tool_stub(event, ctx),
        }
    }

    fn draw(&self, strip: &mut StripBuffer) {
        match self.state {
            HomeState::Menu => self.draw_menu(strip),
            HomeState::ShowApps => self.draw_apps(strip),
            HomeState::ShowCategoryItems => self.draw_category_items(strip),
            HomeState::ShowDailyMantra => self.draw_daily_mantra(strip),
            HomeState::ShowLuaDailyMantra => self.draw_lua_daily_mantra(strip),
            HomeState::ShowLuaPanchang => self.draw_lua_panchang(strip),
            HomeState::ShowLuaCalendar => self.draw_lua_calendar(strip),
            HomeState::ShowLuaFlashcards => self.draw_lua_flashcards(strip),
            HomeState::ShowCalendar => self.draw_calendar(strip),
            HomeState::ShowPanchangLite => self.draw_panchang_lite(strip),
            HomeState::ShowBookmarks => self.draw_bookmarks(strip),
            HomeState::ShowNetworkStatus => self.draw_network_status(strip),
            HomeState::ShowWifiConnect => self.draw_wifi_connect(strip),
            HomeState::ShowDateTime => self.draw_date_time(strip),
            HomeState::ShowSleepImageInfo => self.draw_sleep_image_info(strip),
            HomeState::ShowDeviceInfo => self.draw_device_info(strip),
            HomeState::ShowPlaceholder => self.draw_placeholder(strip),
            HomeState::ShowLuaGameStub => self.draw_lua_game_stub(strip),
            HomeState::ShowLuaToolStub => self.draw_lua_tool_stub(strip),
            HomeState::ShowLuaDictionary => self.draw_dictionary_tool(strip),
        }
    }
}

impl HomeApp {
    fn on_event_menu(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next) => {
                self.move_selection_row(1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev) => {
                self.move_selection_row(-1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::NextJump) | ActionEvent::Repeat(Action::NextJump) => {
                self.move_selection_col(1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::PrevJump) | ActionEvent::Repeat(Action::PrevJump) => {
                self.move_selection_col(-1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Select) => match self.item_action(self.selected) {
                MenuAction::OpenCategory(category) => {
                    self.open_category(category, ctx);
                    Transition::None
                }
            },
            _ => Transition::None,
        }
    }

    fn open_reader_entry(&mut self, ctx: &mut AppContext) -> Transition {
        if let Some(rec) = self.recent_record() {
            log::info!(
                "reader-slice: continue from typed recent book_id={} path={}",
                rec.book_id.as_str(),
                rec.open_path()
            );
            ctx.set_message(rec.open_path().as_bytes());
            Transition::Push(AppId::Reader)
        } else {
            log::info!("home-dashboard: reader app selected without recent; opening library");
            ctx.clear_message();
            Transition::Push(AppId::Files)
        }
    }

    fn open_category(&mut self, category: usize, ctx: &mut AppContext) {
        self.active_category = category.min(Self::category_count().saturating_sub(1));
        self.selected = 0;
        self.state = HomeState::ShowCategoryItems;
        ctx.request_full_redraw();
    }

    fn open_daily_mantra(&mut self, ctx: &mut AppContext) {
        self.return_target = ReturnTarget::CategoryItems;
        self.time_status_loaded = false;
        self.needs_load_time_status = true;
        self.state = HomeState::ShowDailyMantra;
        ctx.request_full_redraw();
    }

    fn open_lua_panchang(&mut self, ctx: &mut AppContext) {
        self.return_target = ReturnTarget::CategoryItems;
        self.time_status_loaded = false;
        self.panchang_mantra_loaded = false;
        self.needs_load_time_status = true;
        self.needs_load_panchang_mantra = true;
        self.needs_load_lua_panchang = true;
        self.state = HomeState::ShowLuaPanchang;
        ctx.request_full_redraw();
    }

    fn open_lua_daily_mantra(&mut self, ctx: &mut AppContext) {
        self.return_target = ReturnTarget::CategoryItems;
        self.time_status_loaded = false;
        self.needs_load_time_status = true;
        self.needs_load_lua_daily_mantra = true;
        self.state = HomeState::ShowLuaDailyMantra;
        ctx.request_full_redraw();
    }

    fn open_lua_calendar(&mut self, ctx: &mut AppContext) {
        self.return_target = ReturnTarget::CategoryItems;
        self.calendar_month_offset = 0;
        self.time_status_loaded = false;
        self.needs_load_time_status = true;
        self.needs_load_lua_calendar = true;
        self.state = HomeState::ShowLuaCalendar;
        ctx.request_full_redraw();
    }

    fn open_panchang_lite(&mut self, ctx: &mut AppContext) {
        self.return_target = ReturnTarget::CategoryItems;
        self.time_status_loaded = false;
        self.panchang_mantra_loaded = false;
        self.needs_load_time_status = true;
        self.needs_load_panchang_mantra = true;
        self.state = HomeState::ShowPanchangLite;
        ctx.request_full_redraw();
    }

    fn open_calendar(&mut self, ctx: &mut AppContext) {
        self.open_lua_calendar(ctx);
    }

    fn open_lua_flashcards(&mut self, ctx: &mut AppContext) {
        self.return_target = ReturnTarget::CategoryItems;
        self.flashcards_topic_open = false;
        self.flashcards_show_back = false;
        self.flashcards_image_buf.clear();
        self.flashcards_image_ready = false;
        self.needs_load_flashcards = true;
        self.flashcards_screen = FlashcardsScreen::fallback();
        self.state = HomeState::ShowLuaFlashcards;
        ctx.request_full_redraw();
    }

    fn load_lua_flashcards(&mut self, k: &mut KernelHandle<'_>) {
        self.flashcards_image_buf.clear();
        self.flashcards_image_ready = false;
        let mut manifest_buf = [0u8; 768];
        let manifest_len = match k.read_lua_app_file_start(
            LUA_FLASHCARDS_APP_FOLDER,
            LUA_FLASHCARDS_MANIFEST_FILE,
            &mut manifest_buf,
        ) {
            Ok((_size, len)) => len,
            Err(_) => {
                self.flashcards_screen = FlashcardsScreen::diagnostic(
                    flashcards_script::FlashcardsSource::MissingManifest,
                    "Flashcards APP.TOM is missing",
                    "Upload /RUSTMIX/APPS/FLASHCRD/APP.TOM",
                );
                return;
            }
        };
        let manifest = match core::str::from_utf8(&manifest_buf[..manifest_len]) {
            Ok(value) => value,
            Err(_) => {
                self.flashcards_screen = FlashcardsScreen::diagnostic(
                    flashcards_script::FlashcardsSource::ManifestInvalidUtf8,
                    "APP.TOM is not valid UTF-8",
                    "Re-upload the Flashcards manifest",
                );
                return;
            }
        };

        let mut script_buf = [0u8; 2048];
        let script_len = match k.read_lua_app_file_start(
            LUA_FLASHCARDS_APP_FOLDER,
            LUA_FLASHCARDS_ENTRY_FILE,
            &mut script_buf,
        ) {
            Ok((_size, len)) => len,
            Err(_) => {
                self.flashcards_screen = FlashcardsScreen::diagnostic(
                    flashcards_script::FlashcardsSource::MissingEntry,
                    "Flashcards MAIN.LUA is missing",
                    "Upload /RUSTMIX/APPS/FLASHCRD/MAIN.LUA",
                );
                return;
            }
        };
        let script = match core::str::from_utf8(&script_buf[..script_len]) {
            Ok(value) => value,
            Err(_) => {
                self.flashcards_screen = FlashcardsScreen::diagnostic(
                    flashcards_script::FlashcardsSource::ScriptInvalidUtf8,
                    "MAIN.LUA is not valid UTF-8",
                    "Re-upload the Flashcards script",
                );
                return;
            }
        };

        let mut topic_index_buf = [0u8; FLASHCARDS_TOPIC_INDEX_BUF_LEN];
        let topic_index_len = match k.read_lua_flashcards_topic_index_start(&mut topic_index_buf) {
            Ok((_size, len)) => len,
            Err(_) => {
                let default_index = flashcards_script::FLASHCARDS_DEFAULT_TOPIC_INDEX.as_bytes();
                let mut probe = [0u8; 1];
                let has_seed_topic = k
                    .read_lua_flashcards_topic_cards_start(
                        flashcards_script::FLASHCARDS_DEFAULT_TEXT_TOPIC_FOLDER,
                        &mut probe,
                    )
                    .is_ok();
                if !has_seed_topic || default_index.len() > topic_index_buf.len() {
                    self.flashcards_screen = FlashcardsScreen::diagnostic(
                        flashcards_script::FlashcardsSource::MissingTopicIndex,
                        "TOPICS/INDEX.TXT is missing",
                        "Add topic rows such as TEXTDEMO|Text Flashcards|TEXT",
                    );
                    return;
                }
                topic_index_buf[..default_index.len()].copy_from_slice(default_index);
                default_index.len()
            }
        };
        let topic_index = match core::str::from_utf8(&topic_index_buf[..topic_index_len]) {
            Ok(value) => value,
            Err(_) => {
                self.flashcards_screen = FlashcardsScreen::diagnostic(
                    flashcards_script::FlashcardsSource::TopicIndexInvalidUtf8,
                    "TOPICS/INDEX.TXT is not valid UTF-8",
                    "Re-upload the topic index",
                );
                return;
            }
        };

        let topic_count = flashcards_script::topic_count(topic_index);
        if topic_count == 0 {
            self.flashcards_screen = FlashcardsScreen::diagnostic(
                flashcards_script::FlashcardsSource::EmptyTopicIndex,
                "No topics found in TOPICS/INDEX.TXT",
                "Add TEXTDEMO, IMGDEMO, or your own topic rows",
            );
            return;
        }
        self.flashcards_topic_index = self
            .flashcards_topic_index
            .min(topic_count.saturating_sub(1));

        if !self.flashcards_topic_open {
            self.flashcards_screen = flashcards_script::build_flashcards_topic_list_screen(
                manifest,
                script,
                topic_index,
                self.flashcards_topic_index,
            );
            log::info!(
                "lua-app marker={} app=flashcards mode=topic-list source={}",
                LUA_FLASHCARDS_APP_MARKER,
                self.flashcards_screen.source.label()
            );
            return;
        }

        let Some(topic_folder) =
            flashcards_script::topic_folder_at(topic_index, self.flashcards_topic_index)
        else {
            self.flashcards_screen = FlashcardsScreen::diagnostic(
                flashcards_script::FlashcardsSource::EmptyTopicIndex,
                "Selected topic is invalid",
                "Check TOPICS/INDEX.TXT folder names",
            );
            return;
        };

        let mut cards_buf = alloc::vec::Vec::new();
        cards_buf.resize(FLASHCARDS_CARD_DATA_BUF_LEN, 0);
        let cards_len = match k.read_lua_flashcards_topic_cards_start(topic_folder, &mut cards_buf)
        {
            Ok((_size, len)) => len,
            Err(_) => {
                self.flashcards_screen = FlashcardsScreen::diagnostic(
                    flashcards_script::FlashcardsSource::MissingTopicCards,
                    "Selected topic CARDS.TXT is missing",
                    "Expected TOPICS/<TOPIC>/CARDS.TXT",
                );
                return;
            }
        };
        let cards = match core::str::from_utf8(&cards_buf[..cards_len]) {
            Ok(value) => value,
            Err(_) => {
                self.flashcards_screen = FlashcardsScreen::diagnostic(
                    flashcards_script::FlashcardsSource::TopicCardsInvalidUtf8,
                    "CARDS.TXT is not valid UTF-8",
                    "Re-upload the selected topic cards",
                );
                return;
            }
        };

        let card_count = flashcards_script::card_count(cards);
        if card_count > 0 {
            self.flashcards_card_index =
                self.flashcards_card_index.min(card_count.saturating_sub(1));
        }

        self.flashcards_screen = flashcards_script::build_flashcards_screen(
            manifest,
            script,
            topic_index,
            cards,
            self.flashcards_topic_index,
            self.flashcards_card_index,
            self.flashcards_show_back,
        );
        if self.flashcards_screen.is_image_card() {
            let image_ref = String::from(self.flashcards_screen.image_file());
            self.load_lua_flashcards_image(k, topic_folder, image_ref.as_str());
        }
        log::info!(
            "lua-app marker={} app=flashcards source={}",
            LUA_FLASHCARDS_APP_MARKER,
            self.flashcards_screen.source.label()
        );
    }

    fn load_lua_flashcards_image(
        &mut self,
        k: &mut KernelHandle<'_>,
        topic_folder: &str,
        image_ref: &str,
    ) {
        let mut image = Vec::new();
        image.resize(FLASHCARDS_X4B_FILE_LEN, 0);
        match k.read_lua_flashcards_topic_image_start(topic_folder, image_ref, &mut image) {
            Ok((size, len))
                if size as usize == FLASHCARDS_X4B_FILE_LEN
                    && len == FLASHCARDS_X4B_FILE_LEN
                    && Self::valid_flashcards_x4b(image.as_slice()) =>
            {
                self.flashcards_image_buf = image;
                self.flashcards_image_ready = true;
            }
            _ => {
                self.flashcards_image_buf.clear();
                self.flashcards_image_ready = false;
                self.flashcards_screen = FlashcardsScreen::diagnostic(
                    flashcards_script::FlashcardsSource::MissingTopicImage,
                    "Selected image card is missing or invalid",
                    "Upload X4B images under TOPICS/<TOPIC>/IMG",
                );
            }
        }
    }

    fn valid_flashcards_x4b(data: &[u8]) -> bool {
        data.len() == FLASHCARDS_X4B_FILE_LEN
            && &data[0..4] == b"X4B1"
            && u16::from_le_bytes([data[4], data[5]]) as usize == FLASHCARDS_X4B_W
            && u16::from_le_bytes([data[6], data[7]]) as usize == FLASHCARDS_X4B_H
            && u16::from_le_bytes([data[8], data[9]]) as usize == FLASHCARDS_X4B_STRIDE
    }

    fn open_bookmarks(&mut self, return_target: ReturnTarget, ctx: &mut AppContext) {
        self.return_target = return_target;
        self.bm_selected = 0;
        self.bm_scroll = 0;
        self.reader_tab = 3;
        self.reader_focus_tabs = true;
        self.needs_load_bookmarks = true;
        self.state = HomeState::ShowBookmarks;
        ctx.request_full_redraw();
    }

    fn open_wifi_connect(&mut self, ctx: &mut AppContext) {
        self.return_target = ReturnTarget::CategoryItems;
        self.network_status_loaded = false;
        self.needs_load_network_status = true;
        self.wifi_editor_field = 0;
        self.wifi_editor_char_idx = 0;
        self.wifi_editor_mode = WIFI_EDITOR_MODE_ROWS;
        self.wifi_editor_save_pending = false;
        self.wifi_editor_dirty = false;
        self.wifi_editor_message = WIFI_EDITOR_SAVE_MSG;
        self.state = HomeState::ShowWifiConnect;
        ctx.request_full_redraw();
    }

    fn open_network_status(&mut self, ctx: &mut AppContext) {
        self.return_target = ReturnTarget::CategoryItems;
        self.network_status_loaded = false;
        self.needs_load_network_status = true;
        self.time_status_loaded = false;
        self.needs_load_time_status = true;
        self.state = HomeState::ShowNetworkStatus;
        ctx.request_full_redraw();
    }

    fn open_date_time(&mut self, ctx: &mut AppContext) {
        self.return_target = ReturnTarget::CategoryItems;
        self.time_status_loaded = false;
        self.needs_load_time_status = true;
        self.state = HomeState::ShowDateTime;
        ctx.request_full_redraw();
    }

    fn open_sleep_image_info(&mut self, ctx: &mut AppContext) {
        self.return_target = ReturnTarget::CategoryItems;
        self.state = HomeState::ShowSleepImageInfo;
        ctx.request_full_redraw();
    }

    fn open_device_info(&mut self, ctx: &mut AppContext) {
        self.return_target = ReturnTarget::CategoryItems;
        self.network_status_loaded = false;
        self.needs_load_network_status = true;
        self.time_status_loaded = false;
        self.needs_load_time_status = true;
        self.state = HomeState::ShowDeviceInfo;
        ctx.request_full_redraw();
    }
    fn open_lua_tool_stub(&mut self, idx: usize, ctx: &mut AppContext) {
        self.lua_tool_index = idx.min(tool_stub_script::lua_tool_count().saturating_sub(1));
        if self.lua_tool_index == 0 {
            self.dictionary.reset();
        }
        if self.lua_tool_index == 1 {
            self.unit_converter.reset();
        }
        if self.lua_tool_index == 2 {
            self.time_status_loaded = false;
            self.needs_load_time_status = true;
        }
        self.lua_tool_screen = LuaToolStubScreen::fallback_for(self.lua_tool_index);
        self.needs_load_lua_tool_stub = true;
        self.return_target = ReturnTarget::CategoryItems;
        self.state = HomeState::ShowLuaToolStub;
        ctx.request_full_redraw();
    }
    fn load_dictionary_fallback_json(&mut self, k: &mut KernelHandle<'_>) {
        let mut data_buf = alloc::vec::Vec::new();
        data_buf.resize(dictionary::MAX_DICTIONARY_SHARD_BYTES, 0);
        match k.read_lua_dictionary_fallback_json_start(&mut data_buf) {
            Ok((size, len)) => {
                if size as usize > dictionary::MAX_DICTIONARY_SHARD_BYTES {
                    self.dictionary
                        .mark_shard_too_large(dictionary::LUA_DICTIONARY_FALLBACK_JSON_FILE, size);
                    self.lua_tool_screen
                        .line2
                        .set("DICT.JSN too large; use INDEX.TXT shards");
                    return;
                }
                match core::str::from_utf8(&data_buf[..len]) {
                    Ok(data) => {
                        match self.dictionary.load_dictionary_json_with_source(data, true) {
                            Ok(()) => {
                                self.lua_tool_screen.footer.set("Loaded fallback DICT.JSN");
                            }
                            Err(_) => {
                                self.dictionary.load_default();
                                self.lua_tool_screen
                                    .footer
                                    .set("Using built-in dictionary fallback");
                            }
                        }
                    }
                    Err(_) => {
                        self.dictionary.load_default();
                        self.lua_tool_screen
                            .line2
                            .set("DICT.JSN invalid UTF-8; using built-in fallback");
                    }
                }
            }
            Err(_) => {
                self.dictionary.load_default();
                self.lua_tool_screen
                    .line2
                    .set("Missing INDEX.TXT/DICT.JSN; using built-in fallback");
            }
        }
    }

    fn load_lua_dictionary_shard_from_index(&mut self, k: &mut KernelHandle<'_>) {
        let mut index_chunk = [0u8; dictionary::DICTIONARY_INDEX_SCAN_BUF_BYTES];
        let mut resolver = dictionary::DictionaryIndexResolver::new(
            self.dictionary.query_text(),
            self.dictionary.search_prefix(),
        );

        let (index_size, first_len) = match k.read_lua_dictionary_index_start(&mut index_chunk) {
            Ok((size, len)) => (size, len),
            Err(_) => {
                self.load_dictionary_fallback_json(k);
                return;
            }
        };

        if index_size as usize > dictionary::MAX_DICTIONARY_INDEX_BYTES {
            self.dictionary
                .mark_parse_failed("INDEX.TXT too large; rebuild dictionary pack");
            self.lua_tool_screen
                .line2
                .set("INDEX.TXT too large; rebuild dictionary pack");
            return;
        }

        resolver.push_bytes(&index_chunk[..first_len]);
        let mut offset = first_len as u32;
        while offset < index_size {
            match k.read_lua_dictionary_index_chunk(offset, &mut index_chunk) {
                Ok(0) => break,
                Ok(len) => {
                    resolver.push_bytes(&index_chunk[..len]);
                    offset = offset.saturating_add(len as u32);
                }
                Err(_) => {
                    self.dictionary.mark_parse_failed("INDEX.TXT read failed");
                    self.lua_tool_screen.line2.set("INDEX.TXT read failed");
                    return;
                }
            }
        }

        let (saw_valid_index_line, resolved_shard_path) = resolver.finish_with_status();
        if !saw_valid_index_line {
            self.dictionary
                .mark_parse_failed("INDEX.TXT has no valid shard rows");
            self.lua_tool_screen
                .line2
                .set("INDEX.TXT has no valid shard rows");
            return;
        }

        let Some(shard_path) = resolved_shard_path else {
            let search_label = self.dictionary.search_label();
            self.dictionary.mark_missing_shard(search_label.as_str());
            self.lua_tool_screen
                .line2
                .set("INDEX.TXT missing matching prefix shard");
            return;
        };

        let mut data_buf = alloc::vec::Vec::new();
        data_buf.resize(dictionary::MAX_DICTIONARY_SHARD_BYTES, 0);
        match k.read_lua_dictionary_shard_start(shard_path.as_str(), &mut data_buf) {
            Ok((size, data_len)) => {
                if size as usize > dictionary::MAX_DICTIONARY_SHARD_BYTES {
                    self.dictionary
                        .mark_shard_too_large(shard_path.as_str(), size);
                    self.lua_tool_screen
                        .line2
                        .set("Dictionary shard too large; split smaller");
                    return;
                }
                match core::str::from_utf8(&data_buf[..data_len]) {
                    Ok(data) => {
                        match self.dictionary.load_shard_json_with_label(
                            self.dictionary.desired_shard_letter(),
                            data,
                            shard_path.as_str(),
                        ) {
                            Ok(()) => {
                                self.lua_tool_screen
                                    .footer
                                    .set("Loaded dictionary prefix shard from DATA");
                            }
                            Err(_) => {
                                self.load_dictionary_fallback_json(k);
                            }
                        }
                    }
                    Err(_) => {
                        self.dictionary.mark_parse_failed("shard invalid UTF-8");
                        self.lua_tool_screen
                            .line2
                            .set("Dictionary shard invalid UTF-8");
                    }
                }
            }
            Err(_) => {
                self.dictionary.mark_missing_shard(shard_path.as_str());
                self.lua_tool_screen
                    .line2
                    .set("Dictionary shard missing in DATA");
            }
        }
    }

    fn load_lua_tool_stub(&mut self, k: &mut KernelHandle<'_>) {
        let app = tool_stub_script::lua_tool_app(self.lua_tool_index);
        let mut manifest_buf = [0u8; 768];
        let manifest_len =
            match k.read_lua_game_app_file_start(app.folder, "APP.TOM", &mut manifest_buf) {
                Ok((_size, len)) => len,
                Err(_) => {
                    self.lua_tool_screen = LuaToolStubScreen::diagnostic(
                        self.lua_tool_index,
                        tool_stub_script::LuaToolStubSource::MissingManifest,
                        "Lua tool APP.TOM is missing",
                        "Upload APP.TOM with Wi-Fi Transfer",
                    );
                    return;
                }
            };
        let manifest = match core::str::from_utf8(&manifest_buf[..manifest_len]) {
            Ok(value) => value,
            Err(_) => {
                self.lua_tool_screen = LuaToolStubScreen::diagnostic(
                    self.lua_tool_index,
                    tool_stub_script::LuaToolStubSource::ManifestInvalidUtf8,
                    "APP.TOM is not valid UTF-8",
                    "Re-upload the manifest file",
                );
                return;
            }
        };

        let mut script_buf = [0u8; 2048];
        let script_len =
            match k.read_lua_game_app_file_start(app.folder, "MAIN.LUA", &mut script_buf) {
                Ok((_size, len)) => len,
                Err(_) => {
                    self.lua_tool_screen = LuaToolStubScreen::diagnostic(
                        self.lua_tool_index,
                        tool_stub_script::LuaToolStubSource::MissingEntry,
                        "Lua tool MAIN.LUA is missing",
                        "Upload MAIN.LUA with Wi-Fi Transfer",
                    );
                    return;
                }
            };
        let script = match core::str::from_utf8(&script_buf[..script_len]) {
            Ok(value) => value,
            Err(_) => {
                self.lua_tool_screen = LuaToolStubScreen::diagnostic(
                    self.lua_tool_index,
                    tool_stub_script::LuaToolStubSource::ScriptInvalidUtf8,
                    "MAIN.LUA is not valid UTF-8",
                    "Re-upload the script file",
                );
                return;
            }
        };

        self.lua_tool_screen =
            tool_stub_script::build_tool_stub_runtime(self.lua_tool_index, manifest, script);

        if self.lua_tool_index == 0 {
            self.load_lua_dictionary_shard_from_index(k);
        } else if self.lua_tool_index == 1 {
            let mut data_buf = [0u8; 2048];
            match k.read_lua_game_app_file_start(app.folder, app.data_file, &mut data_buf) {
                Ok((_size, len)) => match core::str::from_utf8(&data_buf[..len]) {
                    Ok(data) => self.unit_converter.load_units(data),
                    Err(_) => self.unit_converter.load_default(),
                },
                Err(_) => self.unit_converter.load_default(),
            }
        } else if self.lua_tool_index == 2 {
            if !self.time_status_loaded {
                self.load_time_status(k);
            }
            let date_index = self.daily_quote_date_index();
            let date_label = self.daily_quote_date_label();
            let mut data_buf = alloc::vec::Vec::new();
            data_buf.resize(GEETA_QUOTE_DATA_BUF_LEN, 0);
            match k.read_lua_game_app_file_start(app.folder, app.data_file, &mut data_buf) {
                Ok((_size, len)) => match core::str::from_utf8(&data_buf[..len]) {
                    Ok(quotes) => {
                        self.lua_tool_screen =
                            tool_stub_script::build_daily_bhagvat_geeta_quote_runtime(
                                self.lua_tool_index,
                                manifest,
                                script,
                                quotes,
                                date_index,
                                date_label.as_deref(),
                            );
                    }
                    Err(_) => {
                        self.lua_tool_screen = LuaToolStubScreen::diagnostic(
                            self.lua_tool_index,
                            tool_stub_script::LuaToolStubSource::ScriptInvalidUtf8,
                            "QUOTES.TXT is not valid UTF-8",
                            "Re-upload quote data",
                        );
                    }
                },
                Err(_) => {
                    self.lua_tool_screen = LuaToolStubScreen::diagnostic(
                        self.lua_tool_index,
                        tool_stub_script::LuaToolStubSource::MissingEntry,
                        "Missing QUOTES.TXT",
                        "Upload quote data with Wi-Fi Transfer",
                    );
                }
            }
        }
    }
    fn on_event_lua_tool_stub(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                self.return_to_target(ctx)
            }
            ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next)
                if self.lua_tool_index == 0 =>
            {
                self.dictionary.move_keyboard_down();
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev)
                if self.lua_tool_index == 0 =>
            {
                self.dictionary.move_keyboard_up();
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::NextJump) | ActionEvent::Repeat(Action::NextJump)
                if self.lua_tool_index == 0 =>
            {
                self.dictionary.move_keyboard_right();
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::PrevJump) | ActionEvent::Repeat(Action::PrevJump)
                if self.lua_tool_index == 0 =>
            {
                self.dictionary.move_keyboard_left();
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::Select) if self.lua_tool_index == 0 => {
                let selected_key = self
                    .dictionary
                    .keyboard_key(self.dictionary.keyboard_cursor());
                let before_query = String::from(self.dictionary.query_text());
                let before_letter = self.dictionary.desired_shard_letter();
                self.dictionary.select_keyboard_key();
                let after_query = self.dictionary.query_text();
                let after_letter = self.dictionary.desired_shard_letter();
                if selected_key == "GO"
                    || selected_key == "*"
                    || after_query != before_query.as_str()
                    || after_letter != before_letter
                {
                    self.needs_load_lua_tool_stub = true;
                }
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next)
                if self.lua_tool_index == 1 =>
            {
                self.unit_converter.next_conversion();
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev)
                if self.lua_tool_index == 1 =>
            {
                self.unit_converter.prev_conversion();
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::NextJump) | ActionEvent::Repeat(Action::NextJump)
                if self.lua_tool_index == 1 =>
            {
                self.unit_converter.next_amount();
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::PrevJump) | ActionEvent::Repeat(Action::PrevJump)
                if self.lua_tool_index == 1 =>
            {
                self.unit_converter.prev_amount();
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::Select) if self.lua_tool_index == 1 => {
                self.unit_converter.next_amount();
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::Select) => self.return_to_target(ctx),
            _ => Transition::None,
        }
    }

    fn open_lua_game_stub(&mut self, idx: usize, ctx: &mut AppContext) {
        self.lua_game_index = idx.min(game_stub_script::lua_game_count().saturating_sub(1));
        self.lua_game_screen = LuaGameStubScreen::fallback_for(self.lua_game_index);
        self.needs_load_lua_game_stub = true;
        self.return_target = ReturnTarget::CategoryItems;
        self.state = HomeState::ShowLuaGameStub;
        ctx.request_full_redraw();
    }

    fn load_lua_game_stub(&mut self, k: &mut KernelHandle<'_>) {
        let app = game_stub_script::lua_game_app(self.lua_game_index);
        let mut manifest_buf = [0u8; 512];
        let manifest_len =
            match k.read_lua_game_app_file_start(app.folder, "APP.TOM", &mut manifest_buf) {
                Ok((_size, n)) => n,
                Err(_) => {
                    self.lua_game_screen = LuaGameStubScreen::diagnostic(
                        self.lua_game_index,
                        game_stub_script::LuaGameStubSource::MissingManifest,
                        "Missing APP.TOM",
                        "Upload the app folder with Wi-Fi Transfer",
                    );
                    return;
                }
            };

        let manifest = match core::str::from_utf8(&manifest_buf[..manifest_len]) {
            Ok(text) => text,
            Err(_) => {
                self.lua_game_screen = LuaGameStubScreen::diagnostic(
                    self.lua_game_index,
                    game_stub_script::LuaGameStubSource::ManifestInvalidUtf8,
                    "APP.TOM is not valid UTF-8",
                    "Replace APP.TOM and reopen the app",
                );
                return;
            }
        };

        let mut script_buf = [0u8; 1024];
        let script_len =
            match k.read_lua_game_app_file_start(app.folder, "MAIN.LUA", &mut script_buf) {
                Ok((_size, n)) => n,
                Err(_) => {
                    self.lua_game_screen = LuaGameStubScreen::diagnostic(
                        self.lua_game_index,
                        game_stub_script::LuaGameStubSource::MissingEntry,
                        "Missing MAIN.LUA",
                        "Upload MAIN.LUA and reopen the app",
                    );
                    return;
                }
            };

        let script = match core::str::from_utf8(&script_buf[..script_len]) {
            Ok(text) => text,
            Err(_) => {
                self.lua_game_screen = LuaGameStubScreen::diagnostic(
                    self.lua_game_index,
                    game_stub_script::LuaGameStubSource::ScriptInvalidUtf8,
                    "MAIN.LUA is not valid UTF-8",
                    "Replace MAIN.LUA and reopen the app",
                );
                return;
            }
        };

        self.lua_game_screen =
            game_stub_script::build_game_stub_runtime(self.lua_game_index, manifest, script);

        let mut data_buf = [0u8; 1024];
        if let Ok((_size, data_len)) =
            k.read_lua_game_app_file_start(app.folder, app.data_file, &mut data_buf)
        {
            if let Ok(data) = core::str::from_utf8(&data_buf[..data_len]) {
                self.hydrate_lua_grid_game_data(self.lua_game_index, data);
            }
        }
    }

    fn hydrate_lua_grid_game_data(&mut self, index: usize, data: &str) {
        match index {
            0 => self.sudoku_game.load_puzzle_line(data),
            1 => self.mines_game.load_board_line(data),
            2 => self.freecell_game.load_cards_line(data),
            3 => self.memory_game.load_cards_line(data),
            4 => self.solitaire_game.load_cards_line(data),
            5 => self.ludo_game.load_config_line(data),
            6 => self.snakes_game.load_board_line(data),
            _ => {}
        }
    }

    fn move_lua_grid_cursor(&mut self, direction: GridMove, ctx: &mut AppContext) {
        match self.lua_game_index {
            0 => self.sudoku_game.move_cursor(direction),
            1 => self.mines_game.move_cursor(direction),
            2 => self.freecell_game.move_cursor(direction),
            3 => self.memory_game.move_cursor(direction),
            4 => self.solitaire_game.move_cursor(direction),
            5 => self.ludo_game.move_cursor(direction),
            6 => self.snakes_game.move_cursor(direction),
            _ => {}
        }
        ctx.request_full_redraw();
    }

    fn select_lua_grid_cell(&mut self, ctx: &mut AppContext) {
        match self.lua_game_index {
            0 => self.sudoku_game.select(),
            1 => self.mines_game.select(),
            2 => self.freecell_game.select(),
            3 => self.memory_game.select(),
            4 => self.solitaire_game.select(),
            5 => self.ludo_game.select(),
            6 => self.snakes_game.select(),
            _ => {}
        }
        ctx.request_full_redraw();
    }

    const fn is_playable_lua_grid_game(index: usize) -> bool {
        index <= 6
    }

    fn on_event_lua_game_stub(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                self.return_to_target(ctx)
            }
            ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next)
                if Self::is_playable_lua_grid_game(self.lua_game_index) =>
            {
                self.move_lua_grid_cursor(GridMove::Down, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev)
                if Self::is_playable_lua_grid_game(self.lua_game_index) =>
            {
                self.move_lua_grid_cursor(GridMove::Up, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::NextJump) | ActionEvent::Repeat(Action::NextJump)
                if Self::is_playable_lua_grid_game(self.lua_game_index) =>
            {
                self.move_lua_grid_cursor(GridMove::Right, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::PrevJump) | ActionEvent::Repeat(Action::PrevJump)
                if Self::is_playable_lua_grid_game(self.lua_game_index) =>
            {
                self.move_lua_grid_cursor(GridMove::Left, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Select)
                if Self::is_playable_lua_grid_game(self.lua_game_index) =>
            {
                self.select_lua_grid_cell(ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Select) => self.return_to_target(ctx),
            _ => Transition::None,
        }
    }

    fn open_placeholder(
        &mut self,
        title: &'static str,
        detail: &'static str,
        return_target: ReturnTarget,
        ctx: &mut AppContext,
    ) {
        self.placeholder_title = title;
        self.placeholder_detail = detail;
        self.return_target = return_target;
        self.state = HomeState::ShowPlaceholder;
        ctx.request_full_redraw();
    }

    fn return_to_target(&mut self, ctx: &mut AppContext) -> Transition {
        match self.return_target {
            ReturnTarget::Menu => {
                self.state = HomeState::Menu;
                self.selected = 0;
            }
            ReturnTarget::CategoryItems => {
                self.state = HomeState::ShowCategoryItems;
                self.selected = 0;
            }
        }
        ctx.request_full_redraw();
        Transition::None
    }

    fn on_event_apps(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next) => {
                self.move_selection_row(1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev) => {
                self.move_selection_row(-1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::NextJump) | ActionEvent::Repeat(Action::NextJump) => {
                self.move_selection_col(1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::PrevJump) | ActionEvent::Repeat(Action::PrevJump) => {
                self.move_selection_col(-1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                self.selected = 0;
                self.state = HomeState::Menu;
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::Select) => {
                self.open_category(self.selected, ctx);
                Transition::None
            }
            _ => Transition::None,
        }
    }

    fn on_event_category_items(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next) => {
                let count = Self::category_item_count(self.active_category);
                if count > 0 {
                    let previous = self.selected.min(count.saturating_sub(1));
                    let next = (previous + 1).min(count.saturating_sub(1));
                    self.selected = next;
                    self.request_apps_list_selection_redraw(ctx, previous, next);
                }
                Transition::None
            }
            ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev) => {
                let count = Self::category_item_count(self.active_category);
                if count > 0 {
                    let previous = self.selected.min(count.saturating_sub(1));
                    let next = previous.saturating_sub(1);
                    self.selected = next;
                    self.request_apps_list_selection_redraw(ctx, previous, next);
                }
                Transition::None
            }
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                self.selected = self.active_category;
                self.state = HomeState::Menu;
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::Select) | ActionEvent::Press(Action::NextJump) => {
                self.open_category_item(ctx)
            }
            _ => Transition::None,
        }
    }

    fn open_category_item(&mut self, ctx: &mut AppContext) -> Transition {
        match (self.active_category, self.selected) {
            // Network
            (0, 0) => Transition::Push(AppId::BiscuitWifi),
            (0, 1) => Transition::Push(AppId::Upload),
            (0, _) => {
                self.open_network_status(ctx);
                Transition::None
            }

            // Productivity
            (1, 0) => {
                self.open_calendar(ctx);
                Transition::None
            }
            (1, _) => {
                self.open_lua_flashcards(ctx);
                Transition::None
            }

            // Games
            (2, _) => {
                self.open_lua_game_stub(self.selected, ctx);
                Transition::None
            }

            // Reader
            (3, 0) => self.open_reader_entry(ctx),
            (3, 1) => Transition::Push(AppId::Files),
            (3, _) => {
                self.open_bookmarks(ReturnTarget::CategoryItems, ctx);
                Transition::None
            }

            // System
            (4, 0) => Transition::Push(AppId::Settings),
            (4, 1) => {
                self.open_date_time(ctx);
                Transition::None
            }
            (4, 2) => {
                self.open_sleep_image_info(ctx);
                Transition::None
            }
            (4, _) => {
                self.open_device_info(ctx);
                Transition::None
            }

            // Tools
            (5, 0) => Transition::Push(AppId::Files),
            (5, 1) => {
                self.open_lua_tool_stub(0, ctx);
                Transition::None
            }
            (5, 2) => {
                self.open_lua_tool_stub(1, ctx);
                Transition::None
            }
            (5, _) => {
                self.open_placeholder(
                    "Placeholder",
                    "This app slot is not assigned.",
                    ReturnTarget::CategoryItems,
                    ctx,
                );
                Transition::None
            }
            _ => Transition::None,
        }
    }

    fn on_event_daily_mantra(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back)
            | ActionEvent::LongPress(Action::Back)
            | ActionEvent::Press(Action::Select) => self.return_to_target(ctx),
            _ => Transition::None,
        }
    }

    fn on_event_lua_daily_mantra(
        &mut self,
        event: ActionEvent,
        ctx: &mut AppContext,
    ) -> Transition {
        match event {
            ActionEvent::Press(Action::Back)
            | ActionEvent::LongPress(Action::Back)
            | ActionEvent::Press(Action::Select) => self.return_to_target(ctx),
            _ => Transition::None,
        }
    }

    fn on_event_lua_panchang(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back)
            | ActionEvent::LongPress(Action::Back)
            | ActionEvent::Press(Action::Select) => self.return_to_target(ctx),
            _ => Transition::None,
        }
    }

    fn on_event_panchang_lite(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back)
            | ActionEvent::LongPress(Action::Back)
            | ActionEvent::Press(Action::Select) => self.return_to_target(ctx),
            _ => Transition::None,
        }
    }

    fn on_event_lua_flashcards(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                if self.flashcards_topic_open {
                    self.flashcards_topic_open = false;
                    self.flashcards_show_back = false;
                    self.flashcards_image_buf.clear();
                    self.flashcards_image_ready = false;
                    self.needs_load_flashcards = true;
                    ctx.request_full_redraw();
                    Transition::None
                } else {
                    self.return_to_target(ctx)
                }
            }
            ActionEvent::Press(Action::Next)
            | ActionEvent::Repeat(Action::Next)
            | ActionEvent::Press(Action::NextJump)
            | ActionEvent::Repeat(Action::NextJump) => {
                if self.flashcards_topic_open {
                    let count = self.flashcards_screen.card_count.max(1);
                    self.flashcards_card_index = (self.flashcards_card_index + 1) % count;
                    self.flashcards_show_back = false;
                } else {
                    let count = self.flashcards_screen.topic_count.max(1);
                    self.flashcards_topic_index = (self.flashcards_topic_index + 1) % count;
                    self.flashcards_card_index = 0;
                    self.flashcards_show_back = false;
                }
                self.needs_load_flashcards = true;
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::Prev)
            | ActionEvent::Repeat(Action::Prev)
            | ActionEvent::Press(Action::PrevJump)
            | ActionEvent::Repeat(Action::PrevJump) => {
                if self.flashcards_topic_open {
                    let count = self.flashcards_screen.card_count.max(1);
                    self.flashcards_card_index = if self.flashcards_card_index == 0 {
                        count.saturating_sub(1)
                    } else {
                        self.flashcards_card_index.saturating_sub(1)
                    };
                    self.flashcards_show_back = false;
                } else {
                    let count = self.flashcards_screen.topic_count.max(1);
                    self.flashcards_topic_index = if self.flashcards_topic_index == 0 {
                        count.saturating_sub(1)
                    } else {
                        self.flashcards_topic_index.saturating_sub(1)
                    };
                    self.flashcards_card_index = 0;
                    self.flashcards_show_back = false;
                }
                self.needs_load_flashcards = true;
                ctx.request_full_redraw();
                Transition::None
            }
            ActionEvent::Press(Action::Select) => {
                if self.flashcards_topic_open {
                    self.flashcards_show_back = !self.flashcards_show_back;
                } else {
                    self.flashcards_topic_open = true;
                    self.flashcards_card_index = 0;
                    self.flashcards_show_back = false;
                }
                self.needs_load_flashcards = true;
                ctx.request_full_redraw();
                Transition::None
            }
            _ => Transition::None,
        }
    }

    fn on_event_lua_calendar(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                self.return_to_target(ctx)
            }
            ActionEvent::Press(Action::Prev)
            | ActionEvent::Repeat(Action::Prev)
            | ActionEvent::Press(Action::PrevJump)
            | ActionEvent::Repeat(Action::PrevJump) => {
                self.move_calendar_month(-1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Next)
            | ActionEvent::Repeat(Action::Next)
            | ActionEvent::Press(Action::NextJump)
            | ActionEvent::Repeat(Action::NextJump) => {
                self.move_calendar_month(1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Select) => {
                if self.calendar_month_offset != 0 {
                    self.calendar_month_offset = 0;
                    ctx.request_full_redraw();
                }
                Transition::None
            }
            _ => Transition::None,
        }
    }

    fn on_event_calendar(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                self.return_to_target(ctx)
            }
            ActionEvent::Press(Action::Prev)
            | ActionEvent::Repeat(Action::Prev)
            | ActionEvent::Press(Action::PrevJump)
            | ActionEvent::Repeat(Action::PrevJump) => {
                self.move_calendar_month(-1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Next)
            | ActionEvent::Repeat(Action::Next)
            | ActionEvent::Press(Action::NextJump)
            | ActionEvent::Repeat(Action::NextJump) => {
                self.move_calendar_month(1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Select) => {
                if self.calendar_month_offset != 0 {
                    self.calendar_month_offset = 0;
                    ctx.request_full_redraw();
                }
                Transition::None
            }
            _ => Transition::None,
        }
    }

    fn move_calendar_month(&mut self, delta: i16, ctx: &mut AppContext) {
        self.calendar_month_offset = self
            .calendar_month_offset
            .saturating_add(delta)
            .clamp(-120, 120);
        ctx.request_full_redraw();
    }

    fn on_event_system_info(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                self.return_to_target(ctx)
            }
            ActionEvent::Press(Action::Select) => {
                if self.state == HomeState::ShowDeviceInfo {
                    self.network_status_loaded = false;
                    self.needs_load_network_status = true;
                    self.time_status_loaded = false;
                    self.needs_load_time_status = true;
                }
                ctx.request_full_redraw();
                Transition::None
            }
            _ => Transition::None,
        }
    }
    fn on_event_placeholder(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back)
            | ActionEvent::LongPress(Action::Back)
            | ActionEvent::Press(Action::Select) => self.return_to_target(ctx),
            _ => Transition::None,
        }
    }

    fn on_event_network_status(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                self.return_to_target(ctx)
            }
            ActionEvent::Press(Action::Select) => {
                self.network_status_loaded = false;
                self.needs_load_network_status = true;
                ctx.request_full_redraw();
                Transition::None
            }
            _ => Transition::None,
        }
    }

    fn suppress_wifi_keyboard_background_and_redraw(&mut self, ctx: &mut AppContext) {
        // CrossPoint-style keyboard session: while the keyboard is active,
        // the edit buffer is owned by the keyboard/session, not by SETTINGS.TXT
        // reloads or Home background refresh.  Navigation changes only RAM
        // cursor state, so drop any stale redraw queued before this navigation
        // event can flush to e-paper.
        if self.state == HomeState::ShowWifiConnect && self.is_wifi_text_editing() {
            self.needs_load_network_status = false;
            self.network_status_loaded = true;
            self.sync_wifi_editor_to_slot();
            let _ = ctx.take_redraw();
        }
    }

    fn activate_wifi_editor_row(&mut self, ctx: &mut AppContext) -> Transition {
        if self.wifi_editor_field == 1 {
            self.resume_wifi_after_scan = true;
            return Transition::Push(AppId::BiscuitWifi);
        }

        self.append_wifi_editor_char();
        ctx.request_full_redraw();
        Transition::None
    }

    fn activate_wifi_keyboard_key(&mut self, ctx: &mut AppContext) -> Transition {
        self.append_wifi_editor_char();
        if self.is_wifi_text_editing() {
            ctx.mark_dirty(CONTENT_REGION);
        } else {
            ctx.request_full_redraw();
        }
        Transition::None
    }

    fn on_event_wifi_connect(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                if self.is_wifi_text_editing() {
                    self.exit_wifi_text_editor();
                    ctx.request_full_redraw();
                    Transition::None
                } else {
                    self.return_to_target(ctx)
                }
            }
            ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next) => {
                if self.is_wifi_text_editing() {
                    self.move_wifi_keyboard_vertical(1);
                    self.suppress_wifi_keyboard_background_and_redraw(ctx);
                } else {
                    self.move_wifi_editor_field(1);
                    ctx.request_full_redraw();
                }
                Transition::None
            }
            ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev) => {
                if self.is_wifi_text_editing() {
                    self.move_wifi_keyboard_vertical(-1);
                    self.suppress_wifi_keyboard_background_and_redraw(ctx);
                } else {
                    self.move_wifi_editor_field(-1);
                    ctx.request_full_redraw();
                }
                Transition::None
            }
            ActionEvent::Press(Action::Select) => {
                if self.is_wifi_text_editing() {
                    self.activate_wifi_keyboard_key(ctx)
                } else {
                    self.activate_wifi_editor_row(ctx)
                }
            }
            ActionEvent::Press(Action::NextJump) => {
                if self.is_wifi_text_editing() {
                    // In normal button layout NextJump is the physical Right key, so keep
                    // it as keyboard navigation. In swapped layout the physical Confirm/OK
                    // button may also arrive as NextJump; callers can still activate with
                    // the semantic Select path, and row-mode activation below handles OK
                    // aliases before the keyboard opens.
                    self.move_wifi_keyboard_horizontal(1);
                    self.suppress_wifi_keyboard_background_and_redraw(ctx);
                    Transition::None
                } else if self.wifi_editor_field == 2 {
                    self.cycle_wifi_scan_result(1);
                    ctx.request_full_redraw();
                    Transition::None
                } else {
                    // OK/Confirm can map to NextJump when swap_buttons is enabled. Treat a
                    // press on rows as Activate so Wi-Fi Networks, Password, Default, and
                    // Save still work. Repeat remains navigation/no-op to avoid repeated Save.
                    self.activate_wifi_editor_row(ctx)
                }
            }
            ActionEvent::Repeat(Action::NextJump) => {
                if self.is_wifi_text_editing() {
                    self.move_wifi_keyboard_horizontal(1);
                    self.suppress_wifi_keyboard_background_and_redraw(ctx);
                } else if self.wifi_editor_field == 2 {
                    self.cycle_wifi_scan_result(1);
                    ctx.request_full_redraw();
                }
                Transition::None
            }
            ActionEvent::Press(Action::PrevJump) | ActionEvent::Repeat(Action::PrevJump) => {
                if self.is_wifi_text_editing() {
                    self.move_wifi_keyboard_horizontal(-1);
                    self.suppress_wifi_keyboard_background_and_redraw(ctx);
                } else if self.wifi_editor_field == 2 {
                    self.cycle_wifi_scan_result(-1);
                    ctx.request_full_redraw();
                } else {
                    self.move_wifi_editor_field(-1);
                    ctx.request_full_redraw();
                }
                Transition::None
            }
            ActionEvent::LongPress(Action::Select) => {
                self.delete_wifi_editor_char();
                self.suppress_wifi_keyboard_background_and_redraw(ctx);
                if self.is_wifi_text_editing() {
                    ctx.mark_dirty(CONTENT_REGION);
                } else {
                    ctx.request_full_redraw();
                }
                Transition::None
            }
            ActionEvent::LongPress(Action::NextJump) | ActionEvent::LongPress(Action::PrevJump) => {
                self.clear_wifi_editor_field();
                self.suppress_wifi_keyboard_background_and_redraw(ctx);
                if self.is_wifi_text_editing() {
                    ctx.mark_dirty(CONTENT_REGION);
                } else {
                    ctx.request_full_redraw();
                }
                Transition::None
            }
            _ => Transition::None,
        }
    }

    fn on_event_date_time(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                self.return_to_target(ctx)
            }
            ActionEvent::Press(Action::Select) => {
                self.resume_date_time_after_sync = true;
                Transition::Push(AppId::TimeSync)
            }
            _ => Transition::None,
        }
    }

    fn switch_reader_tab_from_bookmarks(
        &mut self,
        delta: isize,
        ctx: &mut AppContext,
    ) -> Transition {
        let len = crossink_internal::READER_TABS.len() as isize;
        if len == 0 {
            return Transition::None;
        }
        self.reader_tab = (self.reader_tab as isize + delta).rem_euclid(len) as usize;
        self.reader_focus_tabs = true;
        ctx.request_full_redraw();

        match self.reader_tab {
            1 | 2 => Self::push_files_reader_tab(self.reader_tab, ctx),
            _ => Transition::None,
        }
    }

    fn on_event_bookmarks(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                if self.reader_focus_tabs {
                    self.return_to_target(ctx)
                } else {
                    self.reader_focus_tabs = true;
                    ctx.request_full_redraw();
                    Transition::None
                }
            }

            ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next) => {
                if self.reader_focus_tabs {
                    if self.reader_tab_has_home_rows() {
                        self.reader_focus_tabs = false;
                    }
                    ctx.request_full_redraw();
                    return Transition::None;
                }
                if self.reader_tab == 3 && self.bm_count > 0 {
                    let old = self.bm_selected;
                    let vis = self.bm_visible_lines();
                    if self.bm_selected + 1 < self.bm_count {
                        self.bm_selected += 1;
                        if self.bm_selected >= self.bm_scroll + vis {
                            self.bm_scroll = self.bm_selected + 1 - vis;
                            ctx.mark_dirty(self.bm_list_region());
                        } else {
                            ctx.mark_dirty(self.bm_row_region(old - self.bm_scroll));
                            ctx.mark_dirty(self.bm_row_region(self.bm_selected - self.bm_scroll));
                        }
                    } else {
                        self.bm_selected = 0;
                        self.bm_scroll = 0;
                        ctx.mark_dirty(self.bm_list_region());
                    }
                    ctx.mark_dirty(self.bm_status_region());
                }
                Transition::None
            }

            ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev) => {
                if self.reader_focus_tabs {
                    if self.reader_tab_has_home_rows() {
                        self.reader_focus_tabs = false;
                    }
                    ctx.request_full_redraw();
                    return Transition::None;
                }
                if self.reader_tab == 3 && self.bm_count > 0 {
                    let old = self.bm_selected;
                    let vis = self.bm_visible_lines();
                    if self.bm_selected > 0 {
                        self.bm_selected -= 1;
                        if self.bm_selected < self.bm_scroll {
                            self.bm_scroll = self.bm_selected;
                            ctx.mark_dirty(self.bm_list_region());
                        } else {
                            ctx.mark_dirty(self.bm_row_region(old - self.bm_scroll));
                            ctx.mark_dirty(self.bm_row_region(self.bm_selected - self.bm_scroll));
                        }
                    } else {
                        self.bm_selected = self.bm_count - 1;
                        if self.bm_selected >= vis {
                            self.bm_scroll = self.bm_selected + 1 - vis;
                        }
                        ctx.mark_dirty(self.bm_list_region());
                    }
                    ctx.mark_dirty(self.bm_status_region());
                }
                Transition::None
            }

            ActionEvent::Press(Action::NextJump) | ActionEvent::Repeat(Action::NextJump) => {
                self.switch_reader_tab_from_bookmarks(1, ctx)
            }

            ActionEvent::Press(Action::PrevJump) | ActionEvent::Repeat(Action::PrevJump) => {
                self.switch_reader_tab_from_bookmarks(-1, ctx)
            }

            ActionEvent::Press(Action::Select) => {
                if self.reader_focus_tabs {
                    return self.switch_reader_tab_from_bookmarks(1, ctx);
                }
                if self.reader_tab == 0 {
                    if let Some(record) = &self.recent_record {
                        ctx.set_message(record.open_path().as_bytes());
                        return Transition::Push(AppId::Reader);
                    }
                    return Transition::None;
                }
                if matches!(self.reader_tab, 1 | 2) {
                    return Self::push_files_reader_tab(self.reader_tab, ctx);
                }
                if self.reader_tab != 3 {
                    return Transition::None;
                }
                if self.bm_count > 0 && self.bm_selected < self.bm_count {
                    let slot = &self.bm_entries[self.bm_selected];
                    log::info!(
                        "bookmark-index: home bookmark selected idx={} ch={} off={} label={}",
                        self.bm_selected,
                        slot.chapter,
                        slot.byte_offset,
                        slot.display_label()
                    );
                    let jump = slot.jump_message();
                    ctx.set_message(jump.as_bytes());
                    self.state = HomeState::Menu;
                    Transition::Push(AppId::Reader)
                } else {
                    Transition::None
                }
            }

            _ => Transition::None,
        }
    }
}

impl HomeApp {
    fn draw_home_header_status(&self, strip: &mut StripBuffer, y: u16) {
        let meta_font = self.card_meta_font();
        let battery_w: u16 = 64;
        let time_w: u16 = 190;
        let battery_x = SCREEN_W
            .saturating_sub(LARGE_MARGIN)
            .saturating_sub(battery_w);
        let time_x = battery_x.saturating_sub(time_w + 8);

        let mut time = BitmapDynLabel::<80>::new(
            Region::new(time_x, y, time_w, meta_font.line_height),
            meta_font,
        )
        .alignment(Alignment::CenterRight);
        let _ = self
            .time_cache
            .write_home_time(self.time_uptime_secs, &mut time);
        time.draw(strip).unwrap();

        let pct = time_status::battery_percent_value(self.home_battery_mv);
        let icon_w: u16 = 22;
        let icon_h: u16 = 10;
        let icon_x = SCREEN_W
            .saturating_sub(LARGE_MARGIN)
            .saturating_sub(icon_w + 4);
        let icon_y = y + meta_font.line_height.saturating_sub(icon_h) / 2;
        let pct_w: u16 = battery_w.saturating_sub(icon_w + 8);
        let mut pct_label = BitmapDynLabel::<12>::new(
            Region::new(battery_x, y, pct_w, meta_font.line_height),
            meta_font,
        )
        .alignment(Alignment::CenterRight);
        if let Some(pct) = pct {
            let _ = write!(pct_label, "{}%", pct);
        } else {
            let _ = write!(pct_label, "--");
        }
        pct_label.draw(strip).unwrap();

        Region::new(icon_x, icon_y, icon_w, icon_h)
            .to_rect()
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(strip)
            .unwrap();
        Region::new(icon_x + icon_w, icon_y + 3, 3, icon_h.saturating_sub(6))
            .to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(strip)
            .unwrap();

        if let Some(pct) = pct {
            let inner_w = icon_w.saturating_sub(4);
            let fill_w = ((u32::from(inner_w) * u32::from(pct)) / 100) as u16;
            if fill_w > 0 {
                Region::new(icon_x + 2, icon_y + 2, fill_w, icon_h.saturating_sub(4))
                    .to_rect()
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(strip)
                    .unwrap();
            }
        }
    }

    fn draw_menu(&self, strip: &mut StripBuffer) {
        let header_font = self.header_font();
        let title_region = Region::new(
            LARGE_MARGIN,
            HOME_HEADER_Y,
            FULL_CONTENT_W.saturating_sub(152),
            header_font.line_height,
        );
        BitmapLabel::new(title_region, "Rustmix", header_font)
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();

        self.draw_home_header_status(strip, HOME_HEADER_Y);

        let rule = Region::new(
            LARGE_MARGIN,
            HOME_HEADER_Y + header_font.line_height + HOME_HEADER_RULE_GAP,
            FULL_CONTENT_W,
            2,
        );
        rule.to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(strip)
            .unwrap();

        for i in 0..HOME_CARD_COUNT {
            self.draw_home_card(strip, i);
        }
    }

    fn draw_home_card(&self, strip: &mut StripBuffer, idx: usize) {
        let region = self.item_regions[idx];
        let selected = idx == self.selected;

        let fill = if selected {
            BinaryColor::On
        } else {
            BinaryColor::Off
        };
        region
            .to_rect()
            .into_styled(PrimitiveStyle::with_fill(fill))
            .draw(strip)
            .unwrap();

        let stroke = if selected {
            BinaryColor::Off
        } else {
            BinaryColor::On
        };
        region
            .to_rect()
            .into_styled(PrimitiveStyle::with_stroke(stroke, 2))
            .draw(strip)
            .unwrap();

        let title_font = self.card_title_font();
        let meta_font = self.card_meta_font();
        let text_x = region.x + HOME_CARD_PAD_X;
        let text_w = region.w.saturating_sub(HOME_CARD_PAD_X * 2);
        let title_y = region.y + HOME_CARD_PAD_Y;
        let subtitle_y = title_y + title_font.line_height + HOME_CARD_TEXT_GAP;
        let detail_y = subtitle_y + meta_font.line_height + HOME_CARD_TEXT_GAP;

        BitmapLabel::new(
            Region::new(text_x, title_y, text_w, title_font.line_height),
            self.card_title(idx),
            title_font,
        )
        .alignment(Alignment::CenterLeft)
        .inverted(selected)
        .draw(strip)
        .unwrap();

        BitmapLabel::new(
            Region::new(text_x, subtitle_y, text_w, meta_font.line_height),
            self.card_subtitle(idx),
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .inverted(selected)
        .draw(strip)
        .unwrap();

        let detail = self.card_detail(idx);
        if !detail.is_empty() {
            let detail_region = Region::new(text_x, detail_y, text_w, meta_font.line_height);
            BitmapLabel::new(detail_region, detail.as_str(), meta_font)
                .alignment(Alignment::CenterLeft)
                .inverted(selected)
                .draw(strip)
                .unwrap();
        }
    }

    fn card_title(&self, idx: usize) -> &'static str {
        Self::category_title(idx)
    }

    fn card_subtitle(&self, idx: usize) -> &'static str {
        Self::category_detail(idx)
    }

    fn card_detail(&self, _idx: usize) -> String {
        String::new()
    }

    const fn category_count() -> usize {
        6
    }

    const fn category_title(idx: usize) -> &'static str {
        match idx {
            0 => "Network",
            1 => "Productivity",
            2 => "Games",
            3 => "Reader",
            4 => "System",
            5 => "Tools",
            _ => "",
        }
    }

    const fn category_detail(idx: usize) -> &'static str {
        match idx {
            0 => "Connect & diagnose",
            1 => "Calendar & study apps",
            2 => "Puzzles & board games",
            3 => "Books & bookmarks",
            4 => "Device & settings",
            5 => "Apps & helpers",
            _ => "",
        }
    }

    const fn category_item_count(category: usize) -> usize {
        match category {
            0 => 3,
            1 => 2,
            2 => 7,
            3 => 3,
            4 => 4,
            5 => TOOLS_CATEGORY_ITEM_COUNT,

            _ => 0,
        }
    }

    const fn category_item_title(category: usize, idx: usize) -> &'static str {
        match (category, idx) {
            (0, 0) => "Wi-Fi Networks",
            (0, 1) => "Wi-Fi Transfer",
            (0, 2) => "Network Status",
            (1, 0) => "Calendar",
            (1, 1) => "Flashcards",
            (2, 0) => "Sudoku",
            (2, 1) => "Minesweeper",
            (2, 2) => "FreeCell",
            (2, 3) => "Memory Cards",
            (2, 4) => "Solitaire",
            (2, 5) => "Ludo",
            (2, 6) => "Snakes and Ladder",
            (3, 0) => "Continue Reading",
            (3, 1) => "Library",
            (3, 2) => "Bookmarks",
            (4, 0) => "Settings",
            (4, 1) => "Date & Time",
            (4, 2) => "Sleep Images",
            (4, 3) => "Device Info",
            (5, 0) => "File Browser",
            (5, 1) => "Dictionary",
            (5, 2) => "Unit Converter",
            _ => "",
        }
    }

    const fn category_item_detail(category: usize, idx: usize) -> &'static str {
        match (category, idx) {
            (0, 0) => "Scan SSID + password",
            (0, 1) => "Transfer + time sync",
            (0, 2) => "Wi-Fi, SD and runtime",
            (1, 0) => "Month grid + SD events",
            (1, 1) => "SD topics + cards",
            (2, 0) => "Grid puzzle",
            (2, 1) => "Mine puzzle",
            (2, 2) => "Card puzzle",
            (2, 3) => "Memory puzzle",
            (2, 4) => "Card solitaire",
            (2, 5) => "Board race",
            (2, 6) => "Board ladder",
            (3, 0) => "Resume last book",
            (3, 1) => "Open file library",
            (3, 2) => "Open bookmark list",
            (4, 0) => "Preferences",
            (4, 1) => "Clock and sync",
            (4, 2) => "Random /RUSTMIX/SLEEP",
            (4, 3) => "Device and runtime",
            (5, 0) => "Open file library",
            (5, 1) => "Offline word lookup",
            (5, 2) => "Offline units helper",
            _ => "",
        }
    }

    fn draw_screen_header(&self, strip: &mut StripBuffer, title: &str, status: &str) {
        crossink_internal::draw_header(strip, title, status);
    }

    fn app_row_region(&self, idx: usize) -> Region {
        crossink_internal::list_row_region_from(crossink_internal::LIST_TOP_NO_TABS, idx)
    }

    fn merged_redraw_region(first: Region, second: Region) -> Region {
        let x0 = first.x.min(second.x);
        let y0 = first.y.min(second.y);
        let x1 = first
            .x
            .saturating_add(first.w)
            .max(second.x.saturating_add(second.w));
        let y1 = first
            .y
            .saturating_add(first.h)
            .max(second.y.saturating_add(second.h));
        Region::new(x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0))
    }

    fn request_apps_list_selection_redraw(
        &self,
        ctx: &mut AppContext,
        previous: usize,
        next: usize,
    ) {
        if previous == next {
            return;
        }

        let previous_region = self.app_row_region(previous);
        let next_region = self.app_row_region(next);
        ctx.request_partial_redraw(Self::merged_redraw_region(previous_region, next_region));
    }

    fn draw_apps(&self, strip: &mut StripBuffer) {
        self.draw_screen_header(strip, "Rustmix", "");
        self.draw_home_header_status(strip, BM_TITLE_Y);

        for idx in 0..Self::category_count() {
            self.draw_category_card(strip, idx);
        }
    }

    fn draw_category_card(&self, strip: &mut StripBuffer, idx: usize) {
        let region = self.item_regions[idx];
        let selected = idx == self.selected.min(Self::category_count().saturating_sub(1));

        let fill = if selected {
            BinaryColor::On
        } else {
            BinaryColor::Off
        };
        region
            .to_rect()
            .into_styled(PrimitiveStyle::with_fill(fill))
            .draw(strip)
            .unwrap();

        let stroke = if selected {
            BinaryColor::Off
        } else {
            BinaryColor::On
        };
        region
            .to_rect()
            .into_styled(PrimitiveStyle::with_stroke(stroke, 2))
            .draw(strip)
            .unwrap();

        let title_font = self.card_title_font();
        let meta_font = self.card_meta_font();
        let text_x = region.x + HOME_CARD_PAD_X;
        let text_w = region.w.saturating_sub(HOME_CARD_PAD_X * 2);
        let title_y = region.y + HOME_CARD_PAD_Y;
        let subtitle_y = title_y + title_font.line_height + HOME_CARD_TEXT_GAP;

        BitmapLabel::new(
            Region::new(text_x, title_y, text_w, title_font.line_height),
            Self::category_title(idx),
            title_font,
        )
        .alignment(Alignment::CenterLeft)
        .inverted(selected)
        .draw(strip)
        .unwrap();

        BitmapLabel::new(
            Region::new(text_x, subtitle_y, text_w, meta_font.line_height),
            Self::category_detail(idx),
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .inverted(selected)
        .draw(strip)
        .unwrap();
    }

    fn draw_category_items(&self, strip: &mut StripBuffer) {
        let count = Self::category_item_count(self.active_category);
        let mut status = BitmapDynLabel::<20>::new(
            crossink_internal::header_status_region(),
            fonts::chrome_font(),
        )
        .alignment(Alignment::CenterRight);
        let _ = time_status::battery_label(self.home_battery_mv, &mut status);
        self.draw_screen_header(strip, Self::category_title(self.active_category), "");
        status.draw(strip).unwrap();

        for idx in 0..count {
            self.draw_category_item_row(strip, idx);
        }
    }

    fn draw_category_item_row(&self, strip: &mut StripBuffer, idx: usize) {
        let selected = idx
            == self
                .selected
                .min(Self::category_item_count(self.active_category).saturating_sub(1));
        let row = self.app_row_region(idx);
        crossink_internal::draw_list_item(
            strip,
            row,
            Self::category_item_title(self.active_category, idx),
            Self::category_item_detail(self.active_category, idx),
            selected,
        );
    }

    fn draw_panchang_lite(&self, strip: &mut StripBuffer) {
        let status = if !self.time_status_loaded {
            "Loading"
        } else {
            self.time_cache.freshness(self.time_uptime_secs).as_str()
        };
        self.draw_screen_header(strip, "Panchang", status);

        let title_font = self.card_title_font();
        let meta_font = self.card_meta_font();
        let x = LARGE_MARGIN;
        let w = FULL_CONTENT_W;
        let mut y = CONTENT_TOP + 52;
        let bottom_y = SCREEN_H.saturating_sub(BUTTON_BAR_H + 14);

        if !self.time_status_loaded {
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                "Reading cached Date & Time...",
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            return;
        }

        let Some(panchang) = self.time_cache.display_panchang_lite(self.time_uptime_secs) else {
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                "Sync Date & Time first",
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            y += title_font.line_height + PANCHANG_LINE_GAP;
            BitmapLabel::new(
                Region::new(x, y, w, meta_font.line_height),
                "Panchang works offline after one successful sync",
                meta_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            return;
        };

        self.draw_panchang_line(strip, y, "Weekday", panchang.weekday, meta_font);
        y += meta_font.line_height + PANCHANG_LINE_GAP;
        self.draw_panchang_line(strip, y, "Tithi", panchang.tithi, meta_font);
        y += meta_font.line_height + PANCHANG_LINE_GAP;
        self.draw_panchang_line(strip, y, "Paksha", panchang.paksha, meta_font);
        y += meta_font.line_height + PANCHANG_LINE_GAP;
        self.draw_panchang_line(strip, y, "Month", panchang.month, meta_font);
        y += meta_font.line_height + PANCHANG_MANTRA_BLOCK_GAP;

        if y + title_font.line_height + meta_font.line_height <= bottom_y {
            y = self.draw_panchang_mantra_block(
                strip,
                y,
                self.panchang_mantra_status(),
                title_font,
                meta_font,
            );
            y += PANCHANG_MANTRA_BLOCK_GAP;
        }

        if y + meta_font.line_height <= bottom_y && !panchang.note.is_empty() {
            BitmapLabel::new(
                Region::new(x, y, w, meta_font.line_height),
                panchang.note,
                meta_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
        }
    }

    fn draw_panchang_mantra_block(
        &self,
        strip: &mut StripBuffer,
        mut y: u16,
        mantra: &str,
        mantra_font: &'static BitmapFont,
        label_font: &'static BitmapFont,
    ) -> u16 {
        let x = LARGE_MARGIN;
        let w = FULL_CONTENT_W;

        BitmapLabel::new(
            Region::new(x, y, w, label_font.line_height),
            "Day Mantra",
            label_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();

        y += label_font.line_height + 6;

        let mut current =
            BitmapDynLabel::<112>::new(Region::new(x, y, w, mantra_font.line_height), mantra_font)
                .alignment(Alignment::CenterLeft);

        let mut current_len = 0usize;
        let mut has_words = false;

        for word in mantra.split_whitespace() {
            let extra = if current_len == 0 { 0 } else { 1 };

            if current_len > 0 && current_len + word.len() + extra > 48 {
                current.draw(strip).unwrap();
                y += mantra_font.line_height + 4;

                current = BitmapDynLabel::<112>::new(
                    Region::new(x, y, w, mantra_font.line_height),
                    mantra_font,
                )
                .alignment(Alignment::CenterLeft);

                current_len = 0;
            }

            if current_len > 0 {
                let _ = write!(current, " ");
                current_len += 1;
            }

            let _ = write!(current, "{}", word);
            current_len += word.len();
            has_words = true;
        }

        if has_words && current_len > 0 {
            current.draw(strip).unwrap();
            y += mantra_font.line_height;
        }

        y
    }

    fn draw_panchang_line(
        &self,
        strip: &mut StripBuffer,
        y: u16,
        label: &str,
        value: &str,
        font: &'static BitmapFont,
    ) {
        let x = LARGE_MARGIN;
        let mut line =
            BitmapDynLabel::<96>::new(Region::new(x, y, PANCHANG_VALUE_W, font.line_height), font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(line, "{}: {}", label, value);
        line.draw(strip).unwrap();
    }

    fn draw_calendar(&self, strip: &mut StripBuffer) {
        let status = if !self.time_status_loaded {
            "Loading"
        } else {
            self.time_cache.freshness(self.time_uptime_secs).as_str()
        };
        self.draw_screen_header(strip, "Calendar", status);

        let title_font = self.card_title_font();
        let meta_font = self.card_meta_font();
        let x = LARGE_MARGIN;
        let w = FULL_CONTENT_W;
        let mut y = BM_TITLE_Y + self.ui_fonts.heading.line_height + CALENDAR_GRID_TOP_GAP;

        if !self.time_status_loaded {
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                "Reading cached Date & Time...",
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            return;
        }

        let Some(today) = self.time_cache.display_date(self.time_uptime_secs) else {
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                "Sync Date & Time first",
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            y += title_font.line_height + NETWORK_STATUS_LINE_GAP;
            BitmapLabel::new(
                Region::new(x, y, w, meta_font.line_height),
                "Calendar works offline after one successful sync",
                meta_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            return;
        };

        let (year, month) = Self::calendar_month_from_offset(today, self.calendar_month_offset);
        let mut month_title =
            BitmapDynLabel::<40>::new(Region::new(x, y, w, title_font.line_height), title_font)
                .alignment(Alignment::Center);
        let _ = write!(
            month_title,
            "{} {}",
            time_status::calendar_month_name(month),
            year
        );
        month_title.draw(strip).unwrap();
        y += title_font.line_height + 10;

        let cell_w = (w.saturating_sub(CALENDAR_CELL_GAP * (CALENDAR_COLS as u16 - 1)))
            / CALENDAR_COLS as u16;
        let weekday_h = meta_font.line_height + 4;

        for (idx, label) in CALENDAR_WEEKDAY_LABELS.iter().enumerate() {
            let cell_x = x + (idx as u16).saturating_mul(cell_w + CALENDAR_CELL_GAP);
            BitmapLabel::new(
                Region::new(cell_x, y, cell_w, meta_font.line_height),
                label,
                meta_font,
            )
            .alignment(Alignment::Center)
            .draw(strip)
            .unwrap();
        }
        y += weekday_h;

        let first_weekday = time_status::calendar_weekday_for_date(year, month, 1);
        let days_in_month = time_status::calendar_days_in_month(year, month);
        let cell_h = 36;

        for row in 0..CALENDAR_ROWS {
            for col in 0..CALENDAR_COLS {
                let cell_x = x + (col as u16).saturating_mul(cell_w + CALENDAR_CELL_GAP);
                let cell_y = y + (row as u16).saturating_mul(cell_h + CALENDAR_CELL_GAP);
                let region = Region::new(cell_x, cell_y, cell_w, cell_h);
                let day = (row * CALENDAR_COLS + col) as i16 + 1 - i16::from(first_weekday);

                if day < 1 || day > i16::from(days_in_month) {
                    continue;
                }

                let day = day as u8;
                let is_today = year == today.year && month == today.month && day == today.day;
                self.draw_calendar_day(strip, region, day, is_today, meta_font);
            }
        }

        let footer_y = y + (CALENDAR_ROWS as u16).saturating_mul(cell_h + CALENDAR_CELL_GAP) + 8;
        let footer = if self.calendar_month_offset == 0 {
            "Prev/Next month"
        } else {
            "Select today / Prev / Next"
        };
        BitmapLabel::new(
            Region::new(x, footer_y, w, meta_font.line_height),
            footer,
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
    }

    fn draw_calendar_day(
        &self,
        strip: &mut StripBuffer,
        region: Region,
        day: u8,
        is_today: bool,
        font: &'static BitmapFont,
    ) {
        let fill = if is_today {
            BinaryColor::On
        } else {
            BinaryColor::Off
        };

        region
            .to_rect()
            .into_styled(PrimitiveStyle::with_fill(fill))
            .draw(strip)
            .unwrap();

        region
            .to_rect()
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(strip)
            .unwrap();

        let mut label = BitmapDynLabel::<4>::new(
            Region::new(region.x, region.y + 9, region.w, font.line_height),
            font,
        )
        .alignment(Alignment::Center)
        .inverted(is_today);

        let _ = write!(label, "{}", day);
        label.draw(strip).unwrap();
    }

    fn calendar_month_from_offset(today: time_status::CalendarDate, offset: i16) -> (i32, u8) {
        let base = today
            .year
            .saturating_mul(12)
            .saturating_add(i32::from(today.month))
            .saturating_sub(1);
        let shifted = base.saturating_add(i32::from(offset));
        let year = shifted.div_euclid(12);
        let month = shifted.rem_euclid(12) + 1;

        (year, month as u8)
    }

    fn lua_app_content_top(&self) -> u16 {
        crossink_internal::HEADER_RULE_Y + 16
    }

    fn lua_app_content_bottom(&self) -> u16 {
        SCREEN_H.saturating_sub(BUTTON_BAR_H + 14)
    }

    fn draw_wrapped_lua_line(
        &self,
        strip: &mut StripBuffer,
        text: &str,
        font: &'static BitmapFont,
        x: u16,
        w: u16,
        y: &mut u16,
        bottom_y: u16,
        line_gap: u16,
        semibold: bool,
    ) {
        let text = text.trim();
        if text.is_empty() || *y + font.line_height > bottom_y {
            return;
        }

        let mut line = String::new();
        for word in text.split_whitespace() {
            let candidate_w = if line.is_empty() {
                font.measure_str(word)
            } else {
                font.measure_str(line.as_str())
                    .saturating_add(font.advance(' ') as u16)
                    .saturating_add(font.measure_str(word))
            };

            if !line.is_empty() && candidate_w > w {
                if *y + font.line_height > bottom_y {
                    return;
                }
                let mut label =
                    BitmapLabel::new(Region::new(x, *y, w, font.line_height), line.as_str(), font)
                        .alignment(Alignment::CenterLeft);
                label = if semibold {
                    label.semibold()
                } else {
                    label.medium()
                };
                label.draw(strip).unwrap();
                *y = (*y).saturating_add(font.line_height + line_gap);
                line.clear();
            }

            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }

        if !line.is_empty() && *y + font.line_height <= bottom_y {
            let mut label =
                BitmapLabel::new(Region::new(x, *y, w, font.line_height), line.as_str(), font)
                    .alignment(Alignment::CenterLeft);
            label = if semibold {
                label.semibold()
            } else {
                label.medium()
            };
            label.draw(strip).unwrap();
            *y = (*y).saturating_add(font.line_height + line_gap);
        }
    }

    fn draw_lua_app_text_screen(
        &self,
        strip: &mut StripBuffer,
        title: &str,
        status: &str,
        subtitle: &str,
        line1: &str,
        line2: &str,
        line3: &str,
        footer: &str,
    ) {
        self.draw_screen_header(strip, title, status);

        let heading_font = fonts::ui_list_section_font_fixed();
        let text_font = fonts::ui_list_font_fixed();
        let x = crossink_internal::CONTENT_X;
        let w = crossink_internal::CONTENT_W;
        let mut y = self.lua_app_content_top();
        let bottom_y = self.lua_app_content_bottom();

        self.draw_wrapped_lua_line(
            strip,
            subtitle,
            heading_font,
            x,
            w,
            &mut y,
            bottom_y,
            8,
            true,
        );

        for line in [line1, line2, line3] {
            self.draw_wrapped_lua_line(strip, line, text_font, x, w, &mut y, bottom_y, 7, false);
        }

        if !footer.starts_with("Loaded from") && !footer.starts_with("Source:") {
            self.draw_wrapped_lua_line(strip, footer, text_font, x, w, &mut y, bottom_y, 7, false);
        }
    }

    fn draw_daily_mantra(&self, strip: &mut StripBuffer) {
        let status = if !self.time_status_loaded {
            "Loading"
        } else {
            self.time_cache.freshness(self.time_uptime_secs).as_str()
        };
        self.draw_screen_header(strip, "Daily Mantra", status);

        let title_font = self.card_title_font();
        let meta_font = self.card_meta_font();
        let x = LARGE_MARGIN;
        let w = FULL_CONTENT_W;
        let mut y = CONTENT_TOP + 52;

        if !self.time_status_loaded {
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                "Reading Date & Time...",
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            return;
        }

        if let Some(weekday) = self.time_cache.display_weekday_index(self.time_uptime_secs) {
            let idx = weekday as usize;
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                DAILY_MANTRA_TITLES[idx],
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            y += title_font.line_height + NETWORK_STATUS_LINE_GAP;

            BitmapLabel::new(
                Region::new(x, y, w, meta_font.line_height),
                DAILY_MANTRA_DEDICATIONS[idx],
                meta_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

            BitmapLabel::new(
                Region::new(x, y, w, meta_font.line_height),
                DAILY_MANTRA_ENGLISH[idx],
                meta_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

            let mut date_line =
                BitmapDynLabel::<80>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                    .alignment(Alignment::CenterLeft);
            let _ = write!(date_line, "Date: ");
            let _ = self
                .time_cache
                .write_date_value(self.time_uptime_secs, &mut date_line);
            date_line.draw(strip).unwrap();
        } else {
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                "Date unavailable",
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            y += title_font.line_height + NETWORK_STATUS_LINE_GAP;

            BitmapLabel::new(
                Region::new(x, y, w, meta_font.line_height),
                "Sync Date & Time to select today's mantra",
                meta_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
        }
    }

    fn draw_lua_daily_mantra(&self, strip: &mut StripBuffer) {
        self.draw_lua_app_text_screen(
            strip,
            self.lua_daily_mantra_screen.title(),
            self.lua_daily_mantra_status_label(),
            self.lua_daily_mantra_screen.subtitle(),
            self.lua_daily_mantra_screen.line1(),
            self.lua_daily_mantra_screen.line2(),
            self.lua_daily_mantra_screen.line3(),
            self.lua_daily_mantra_screen.footer(),
        );
    }

    fn draw_lua_panchang(&self, strip: &mut StripBuffer) {
        let status = if self.lua_panchang_screen.source.is_sd_loaded() {
            self.time_cache.freshness(self.time_uptime_secs).as_str()
        } else {
            self.lua_panchang_screen.source.label()
        };

        self.draw_lua_app_text_screen(
            strip,
            self.lua_panchang_screen.title(),
            status,
            self.lua_panchang_screen.subtitle(),
            self.lua_panchang_screen.line1(),
            self.lua_panchang_screen.line2(),
            self.lua_panchang_screen.line3(),
            self.lua_panchang_screen.footer(),
        );
    }

    fn draw_lua_calendar(&self, strip: &mut StripBuffer) {
        let status = if !self.time_status_loaded {
            "Loading"
        } else {
            self.time_cache.freshness(self.time_uptime_secs).as_str()
        };
        self.draw_screen_header(strip, "Calendar", status);

        let title_font = self.card_title_font();
        let meta_font = self.card_meta_font();
        let x = LARGE_MARGIN;
        let w = FULL_CONTENT_W;
        let mut y = CONTENT_TOP + 44;
        let bottom_y = SCREEN_H.saturating_sub(BUTTON_BAR_H + 14);

        if !self.time_status_loaded {
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                "Reading cached Date & Time...",
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            return;
        }

        let Some(today) = self.time_cache.display_date(self.time_uptime_secs) else {
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                "Sync Date & Time first",
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            y += title_font.line_height + NETWORK_STATUS_LINE_GAP;
            BitmapLabel::new(
                Region::new(x, y, w, meta_font.line_height),
                "Calendar works offline after one successful sync",
                meta_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            return;
        };

        let (year, month) = Self::calendar_month_from_offset(today, self.calendar_month_offset);
        let mut month_title =
            BitmapDynLabel::<40>::new(Region::new(x, y, w, title_font.line_height), title_font)
                .alignment(Alignment::Center);
        let _ = write!(
            month_title,
            "{} {}",
            time_status::calendar_month_name(month),
            year
        );
        month_title.draw(strip).unwrap();
        y += title_font.line_height + 7;

        let cell_w = (w.saturating_sub(CALENDAR_CELL_GAP * (CALENDAR_COLS as u16 - 1)))
            / CALENDAR_COLS as u16;
        let weekday_h = meta_font.line_height + 2;

        for (idx, label) in CALENDAR_WEEKDAY_LABELS.iter().enumerate() {
            let cell_x = x + (idx as u16).saturating_mul(cell_w + CALENDAR_CELL_GAP);
            BitmapLabel::new(
                Region::new(cell_x, y, cell_w, meta_font.line_height),
                label,
                meta_font,
            )
            .alignment(Alignment::Center)
            .draw(strip)
            .unwrap();
        }
        y += weekday_h;

        let first_weekday = time_status::calendar_weekday_for_date(year, month, 1);
        let days_in_month = time_status::calendar_days_in_month(year, month);
        let cell_h = 24u16;

        for row in 0..CALENDAR_ROWS {
            for col in 0..CALENDAR_COLS {
                let cell_x = x + (col as u16).saturating_mul(cell_w + CALENDAR_CELL_GAP);
                let cell_y = y + (row as u16).saturating_mul(cell_h + CALENDAR_CELL_GAP);
                let region = Region::new(cell_x, cell_y, cell_w, cell_h);
                let day = (row * CALENDAR_COLS + col) as i16 + 1 - i16::from(first_weekday);

                if day < 1 || day > i16::from(days_in_month) {
                    continue;
                }

                let day = day as u8;
                let is_today = year == today.year && month == today.month && day == today.day;
                let fill = if is_today {
                    BinaryColor::On
                } else {
                    BinaryColor::Off
                };
                region
                    .to_rect()
                    .into_styled(PrimitiveStyle::with_fill(fill))
                    .draw(strip)
                    .unwrap();
                region
                    .to_rect()
                    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                    .draw(strip)
                    .unwrap();

                let mut label = BitmapDynLabel::<4>::new(
                    Region::new(region.x, region.y + 4, region.w, meta_font.line_height),
                    meta_font,
                )
                .alignment(Alignment::Center)
                .inverted(is_today);
                let _ = write!(label, "{}", day);
                label.draw(strip).unwrap();
            }
        }

        y += (CALENDAR_ROWS as u16).saturating_mul(cell_h + CALENDAR_CELL_GAP) + 8;

        if y + title_font.line_height <= bottom_y {
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                "Events",
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            y += title_font.line_height + 6;
        }

        for line in [
            self.lua_calendar_screen.line1(),
            self.lua_calendar_screen.line2(),
            self.lua_calendar_screen.line3(),
        ] {
            if line.is_empty()
                || line.starts_with("Back exits safely")
                || y + meta_font.line_height > bottom_y
            {
                continue;
            }
            BitmapLabel::new(Region::new(x, y, w, meta_font.line_height), line, meta_font)
                .alignment(Alignment::CenterLeft)
                .draw(strip)
                .unwrap();
            y += meta_font.line_height + 6;
        }
    }

    fn draw_wifi_connect(&self, strip: &mut StripBuffer) {
        let status = if !self.network_status_loaded {
            "Loading"
        } else if self.network_wifi_configured && self.network_wifi_password_saved {
            "Ready"
        } else {
            "Setup"
        };
        self.draw_screen_header(strip, "Wi-Fi Networks", status);

        let title_font = self.card_title_font();
        let meta_font = self.card_meta_font();
        let x = LARGE_MARGIN;
        let w = FULL_CONTENT_W;
        let mut y = BM_TITLE_Y + self.ui_fonts.heading.line_height + 24;

        if !self.network_status_loaded {
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                "Reading _x4/SETTINGS.TXT...",
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            return;
        }

        if self.is_wifi_text_editing() {
            let field_name = "Password";
            let mut edit_title =
                BitmapDynLabel::<96>::new(Region::new(x, y, w, title_font.line_height), title_font)
                    .alignment(Alignment::CenterLeft);
            let _ = write!(
                edit_title,
                "Edit {} · {} keyboard",
                field_name,
                text_keyboard::layout_name(self.wifi_keyboard_layout())
            );
            edit_title.draw(strip).unwrap();
            y += title_font.line_height + NETWORK_STATUS_LINE_GAP;

            let mut value_line =
                BitmapDynLabel::<128>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                    .alignment(Alignment::CenterLeft);
            if self.network_pass_len == 0 {
                let _ = write!(value_line, "Password: not set");
            } else {
                let shown = self.network_pass_len.min(28);
                let _ = write!(value_line, "Password: ");
                for _ in 0..shown {
                    let _ = write!(value_line, "*");
                }
                if self.network_pass_len > shown {
                    let _ = write!(value_line, "...");
                }
                let _ = write!(value_line, " ({} chars)", self.network_pass_len);
            }
            value_line.draw(strip).unwrap();
            y += meta_font.line_height + 8;

            let keyboard_h = text_keyboard::TEXT_KEYBOARD_LARGE_HEIGHT;
            text_keyboard::draw(
                strip,
                Region::new(x, y, w, keyboard_h),
                self.wifi_keyboard_layout(),
                self.wifi_editor_char_idx,
                title_font,
                meta_font,
            );
            y += keyboard_h + NETWORK_STATUS_LINE_GAP;

            BitmapLabel::new(
                Region::new(x, y, w, meta_font.line_height),
                "Vol up/down rows · Left/Right keys · OK select",
                meta_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

            BitmapLabel::new(
                Region::new(x, y, w, meta_font.line_height),
                "Hold OK delete · done/back returns · Save persists",
                meta_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

            let mut msg_line =
                BitmapDynLabel::<96>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                    .alignment(Alignment::CenterLeft);
            let _ = write!(msg_line, "{}", self.wifi_editor_message);
            msg_line.draw(strip).unwrap();
            return;
        }

        BitmapLabel::new(
            Region::new(x, y, w, meta_font.line_height),
            "Scan SSID, enter password, choose default, then Save.",
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let profile_prefix = if self.wifi_editor_field == 0 {
            ">"
        } else {
            " "
        };
        let default_mark = if self.network_profile_slot == self.network_default_slot {
            "*"
        } else {
            " "
        };
        let mut profile_line =
            BitmapDynLabel::<96>::new(Region::new(x, y, w, title_font.line_height), title_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(
            profile_line,
            "{} Profile {} {}",
            profile_prefix,
            default_mark,
            self.current_wifi_profile_name()
        );
        profile_line.draw(strip).unwrap();
        y += title_font.line_height + NETWORK_STATUS_LINE_GAP;

        let scan_prefix = if self.wifi_editor_field == 1 {
            ">"
        } else {
            " "
        };
        let mut scan_line =
            BitmapDynLabel::<96>::new(Region::new(x, y, w, title_font.line_height), title_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(scan_line, "{} Scan SSIDs", scan_prefix);
        scan_line.draw(strip).unwrap();
        y += title_font.line_height + NETWORK_STATUS_LINE_GAP;

        let scanned = if self.network_scan_count > 0 {
            Self::ellipsize_ascii(self.wifi_scan_selected_str(), 28)
        } else if self.network_ssid_len > 0 {
            Self::ellipsize_ascii(self.network_ssid_str(), 28)
        } else {
            String::from("scan first")
        };
        let ssid_prefix = if self.wifi_editor_field == 2 {
            ">"
        } else {
            " "
        };
        let mut ssid_line =
            BitmapDynLabel::<96>::new(Region::new(x, y, w, title_font.line_height), title_font)
                .alignment(Alignment::CenterLeft);
        if self.network_scan_count > 0 {
            let _ = write!(
                ssid_line,
                "{} SSID       {} ({}/{})",
                ssid_prefix,
                scanned.as_str(),
                self.network_scan_selected + 1,
                self.network_scan_count
            );
        } else {
            let _ = write!(ssid_line, "{} SSID       {}", ssid_prefix, scanned.as_str());
        }
        ssid_line.draw(strip).unwrap();
        y += title_font.line_height + NETWORK_STATUS_LINE_GAP;

        let pass_prefix = if self.wifi_editor_field == 3 {
            ">"
        } else {
            " "
        };
        let mut pass_line =
            BitmapDynLabel::<96>::new(Region::new(x, y, w, title_font.line_height), title_font)
                .alignment(Alignment::CenterLeft);
        if self.network_pass_len == 0 {
            let _ = write!(pass_line, "{} Password   not set", pass_prefix);
        } else {
            let _ = write!(
                pass_line,
                "{} Password   saved ({} chars)",
                pass_prefix, self.network_pass_len
            );
        }
        pass_line.draw(strip).unwrap();
        y += title_font.line_height + NETWORK_STATUS_LINE_GAP;

        let default_prefix = if self.wifi_editor_field == 4 {
            ">"
        } else {
            " "
        };
        let mut default_line =
            BitmapDynLabel::<96>::new(Region::new(x, y, w, title_font.line_height), title_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(
            default_line,
            "{} Set as default   {}",
            default_prefix,
            if self.network_profile_slot == self.network_default_slot {
                "yes"
            } else {
                "no"
            }
        );
        default_line.draw(strip).unwrap();
        y += title_font.line_height + NETWORK_STATUS_LINE_GAP;

        let save_prefix = if self.wifi_editor_field == 5 {
            ">"
        } else {
            " "
        };
        let mut save_line =
            BitmapDynLabel::<96>::new(Region::new(x, y, w, title_font.line_height), title_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(save_line, "{} Save to SETTINGS.TXT", save_prefix);
        save_line.draw(strip).unwrap();
        y += title_font.line_height + NETWORK_STATUS_LINE_GAP + 4;

        BitmapLabel::new(
            Region::new(x, y, w, meta_font.line_height),
            "Select scans/applies/edits · Left/Right cycles SSIDs",
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        BitmapLabel::new(
            Region::new(x, y, w, meta_font.line_height),
            "Larger keyboard is used for password entry",
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let mut msg_line =
            BitmapDynLabel::<96>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(msg_line, "{}", self.wifi_editor_message);
        msg_line.draw(strip).unwrap();
    }

    fn draw_network_status(&self, strip: &mut StripBuffer) {
        let status = if self.network_status_loaded {
            "Ready"
        } else {
            "Loading"
        };
        self.draw_screen_header(strip, "Network Status", status);

        let title_font = self.card_title_font();
        let meta_font = self.card_meta_font();
        let x = LARGE_MARGIN;
        let w = FULL_CONTENT_W;
        let mut y = BM_TITLE_Y + self.ui_fonts.heading.line_height + 28;

        if !self.network_status_loaded {
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                "Reading local network settings...",
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            return;
        }

        let mut wifi =
            BitmapDynLabel::<64>::new(Region::new(x, y, w, title_font.line_height), title_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(
            wifi,
            "Wi-Fi: {}",
            if self.network_wifi_configured {
                "Configured"
            } else {
                "Not configured"
            }
        );
        wifi.draw(strip).unwrap();
        y += title_font.line_height + NETWORK_STATUS_LINE_GAP;

        let ssid = if self.network_wifi_configured {
            Self::ellipsize_ascii(self.network_ssid_str(), 28)
        } else {
            String::from("-")
        };
        let mut ssid_line =
            BitmapDynLabel::<64>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(
            ssid_line,
            "Default: {} · SSID: {}",
            self.network_wifi
                .profile_name(self.network_default_slot as usize),
            ssid.as_str()
        );
        ssid_line.draw(strip).unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let mut settings_line =
            BitmapDynLabel::<64>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(
            settings_line,
            "Settings: {}",
            if self.network_settings_found {
                "_x4/SETTINGS.TXT"
            } else {
                "not found"
            }
        );
        settings_line.draw(strip).unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        BitmapLabel::new(
            Region::new(x, y, w, meta_font.line_height),
            "Radio: idle until Wi-Fi Transfer",
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        BitmapLabel::new(
            Region::new(x, y, w, meta_font.line_height),
            "IP: shown when transfer starts",
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let mut time_line =
            BitmapDynLabel::<80>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(time_line, "Time Sync: ");
        let _ = self
            .time_cache
            .write_sync_summary(self.time_uptime_secs, &mut time_line);
        time_line.draw(strip).unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let mut sd_line =
            BitmapDynLabel::<64>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(
            sd_line,
            "SD Card: {}",
            if self.network_sd_ok { "OK" } else { "Missing" }
        );
        sd_line.draw(strip).unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let mut power_line =
            BitmapDynLabel::<64>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(power_line, "Battery: {} mV", self.network_battery_mv);
        power_line.draw(strip).unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let mut uptime_line =
            BitmapDynLabel::<64>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(
            uptime_line,
            "Uptime: {}s    Select refresh",
            self.network_uptime_secs
        );
        uptime_line.draw(strip).unwrap();
    }

    fn draw_date_time(&self, strip: &mut StripBuffer) {
        let status = if !self.time_status_loaded {
            "Loading"
        } else {
            self.time_cache.freshness(self.time_uptime_secs).as_str()
        };
        self.draw_screen_header(strip, "Date & Time", status);

        let title_font = self.card_title_font();
        let meta_font = self.card_meta_font();
        let x = LARGE_MARGIN;
        let w = FULL_CONTENT_W;
        let mut y = BM_TITLE_Y + self.ui_fonts.heading.line_height + 28;

        if !self.time_status_loaded {
            BitmapLabel::new(
                Region::new(x, y, w, title_font.line_height),
                "Reading cached time...",
                title_font,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            return;
        }

        let mut time_line =
            BitmapDynLabel::<64>::new(Region::new(x, y, w, title_font.line_height), title_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(time_line, "Time: ");
        let _ = self
            .time_cache
            .write_time_value(self.time_uptime_secs, &mut time_line);
        time_line.draw(strip).unwrap();
        y += title_font.line_height + NETWORK_STATUS_LINE_GAP;

        let mut date_line =
            BitmapDynLabel::<80>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(date_line, "Date: ");
        let _ = self
            .time_cache
            .write_date_value(self.time_uptime_secs, &mut date_line);
        date_line.draw(strip).unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let mut tz_line =
            BitmapDynLabel::<80>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(tz_line, "Timezone: {}", time_status::TIMEZONE_ID);
        tz_line.draw(strip).unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let mut sync_line =
            BitmapDynLabel::<80>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(sync_line, "Sync: ");
        let _ = self
            .time_cache
            .write_sync_summary(self.time_uptime_secs, &mut sync_line);
        sync_line.draw(strip).unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let mut clock_line =
            BitmapDynLabel::<80>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = self
            .time_cache
            .write_clock_detail(self.time_uptime_secs, &mut clock_line);
        clock_line.draw(strip).unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let mut last_line =
            BitmapDynLabel::<80>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(last_line, "Last sync: ");
        let _ = self.time_cache.write_last_sync(&mut last_line);
        last_line.draw(strip).unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let mut result_line =
            BitmapDynLabel::<80>::new(Region::new(x, y, w, meta_font.line_height), meta_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(result_line, "Last result: ");
        if self.time_cache.last_sync_unix.is_none() && self.time_cache.last_sync_error.is_empty() {
            let _ = write!(result_line, "Never");
        } else if self.time_cache.last_sync_ok {
            let _ = write!(result_line, "OK");
        } else if self.time_cache.last_sync_error.is_empty() {
            let _ = write!(result_line, "Failed");
        } else {
            let _ = write!(result_line, "{}", self.time_cache.last_sync_error.as_str());
        }
        result_line.draw(strip).unwrap();
        y += meta_font.line_height + NETWORK_STATUS_LINE_GAP;

        let prompt = match self.time_cache.freshness(self.time_uptime_secs) {
            time_status::ClockFreshness::Live => "Select safely resyncs    Back returns",
            time_status::ClockFreshness::Cached => "Cached; Select safely retries",
            time_status::ClockFreshness::Unsynced => "Select syncs now    Back returns",
        };
        BitmapLabel::new(
            Region::new(x, y, w, meta_font.line_height),
            prompt,
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
    }

    fn draw_lua_grid_cell(
        &self,
        strip: &mut StripBuffer,
        region: Region,
        ch: char,
        selected: bool,
        font: &'static BitmapFont,
    ) {
        if selected {
            region
                .to_rect()
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(strip)
                .unwrap();
        }
        region
            .to_rect()
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(strip)
            .unwrap();

        let mut label = BitmapDynLabel::<4>::new(region, font).alignment(Alignment::Center);
        let _ = write!(label, "{}", ch);
        label.inverted(selected).draw(strip).unwrap();
    }

    fn draw_sudoku_game(&self, strip: &mut StripBuffer) {
        self.draw_screen_header(strip, "Sudoku", self.lua_game_screen.source.label());
        let cell_font = fonts::chrome_font();
        let meta_font = self.card_meta_font();
        let cell_w: u16 = 46;
        let cell_h: u16 = 46;
        let grid_w = cell_w * 9;
        let grid_h = cell_h * 9;
        let grid_x = SCREEN_W.saturating_sub(grid_w) / 2;
        let grid_y = crossink_internal::HEADER_RULE_Y + 18;
        Region::new(grid_x, grid_y, grid_w, grid_h)
            .to_rect()
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
            .draw(strip)
            .unwrap();

        for row in 0..9usize {
            for col in 0..9usize {
                let idx = (row * 9 + col) as u8;
                let cell = Region::new(
                    grid_x + (col as u16).saturating_mul(cell_w),
                    grid_y + (row as u16).saturating_mul(cell_h),
                    cell_w,
                    cell_h,
                );
                self.draw_lua_grid_cell(
                    strip,
                    cell,
                    self.sudoku_game.cell_char(row, col),
                    idx == self.sudoku_game.cursor(),
                    cell_font,
                );
            }
        }

        for block_col in 1..3u16 {
            let x = grid_x + block_col * cell_w * 3;
            Region::new(x.saturating_sub(1), grid_y, 2, grid_h)
                .to_rect()
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(strip)
                .unwrap();
        }
        for block_row in 1..3u16 {
            let y = grid_y + block_row * cell_h * 3;
            Region::new(grid_x, y.saturating_sub(1), grid_w, 2)
                .to_rect()
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(strip)
                .unwrap();
        }

        BitmapLabel::new(
            Region::new(
                LARGE_MARGIN,
                SCREEN_H.saturating_sub(BUTTON_BAR_H + meta_font.line_height + 18),
                FULL_CONTENT_W,
                meta_font.line_height,
            ),
            "Arrows move. OK changes cell.",
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
    }

    fn draw_mines_game(&self, strip: &mut StripBuffer) {
        let status = if self.mines_game.lost() {
            "Boom"
        } else {
            self.lua_game_screen.source.label()
        };
        self.draw_screen_header(strip, "Minesweeper", status);
        let cell_font = fonts::chrome_font();
        let meta_font = self.card_meta_font();
        let cell_w: u16 = 50;
        let cell_h: u16 = 50;
        let grid_w = cell_w * 8;
        let grid_h = cell_h * 8;
        let grid_x = SCREEN_W.saturating_sub(grid_w) / 2;
        let grid_y = crossink_internal::HEADER_RULE_Y + 18;
        Region::new(grid_x, grid_y, grid_w, grid_h)
            .to_rect()
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
            .draw(strip)
            .unwrap();

        for row in 0..8usize {
            for col in 0..8usize {
                let idx = (row * 8 + col) as u8;
                let cell = Region::new(
                    grid_x + (col as u16).saturating_mul(cell_w),
                    grid_y + (row as u16).saturating_mul(cell_h),
                    cell_w,
                    cell_h,
                );
                self.draw_lua_grid_cell(
                    strip,
                    cell,
                    self.mines_game.cell_char(row, col),
                    idx == self.mines_game.cursor(),
                    cell_font,
                );
            }
        }

        let footer = if self.mines_game.lost() {
            "Mine hit. Reopen from Games to reset."
        } else {
            "Arrows move. OK reveals."
        };
        BitmapLabel::new(
            Region::new(
                LARGE_MARGIN,
                SCREEN_H.saturating_sub(BUTTON_BAR_H + meta_font.line_height + 18),
                FULL_CONTENT_W,
                meta_font.line_height,
            ),
            footer,
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
    }

    fn draw_memory_game(&self, strip: &mut StripBuffer) {
        self.draw_screen_header(strip, "Memory Cards", self.lua_game_screen.source.label());
        let cell_font = self.ui_fonts.body;
        let meta_font = self.card_meta_font();
        let cell_w: u16 = 86;
        let cell_h: u16 = 74;
        let grid_w = cell_w * 4;
        let grid_h = cell_h * 4;
        let grid_x = SCREEN_W.saturating_sub(grid_w) / 2;
        let grid_y = crossink_internal::HEADER_RULE_Y + 24;
        Region::new(grid_x, grid_y, grid_w, grid_h)
            .to_rect()
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
            .draw(strip)
            .unwrap();

        for row in 0..4usize {
            for col in 0..4usize {
                let idx = (row * 4 + col) as u8;
                let cell = Region::new(
                    grid_x + (col as u16).saturating_mul(cell_w),
                    grid_y + (row as u16).saturating_mul(cell_h),
                    cell_w,
                    cell_h,
                );
                self.draw_lua_grid_cell(
                    strip,
                    cell,
                    self.memory_game.cell_char(row, col),
                    idx == self.memory_game.cursor(),
                    cell_font,
                );
            }
        }

        let mut matched = BitmapDynLabel::<64>::new(
            Region::new(
                LARGE_MARGIN,
                grid_y + grid_h + 14,
                FULL_CONTENT_W,
                meta_font.line_height,
            ),
            meta_font,
        )
        .alignment(Alignment::CenterLeft);
        let _ = write!(matched, "Matched: {}/16", self.memory_game.matched_count());
        matched.draw(strip).unwrap();

        BitmapLabel::new(
            Region::new(
                LARGE_MARGIN,
                SCREEN_H.saturating_sub(BUTTON_BAR_H + meta_font.line_height + 18),
                FULL_CONTENT_W,
                meta_font.line_height,
            ),
            "Arrows move. OK flips.",
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
    }

    fn draw_freecell_game(&self, strip: &mut StripBuffer) {
        let status = if self.freecell_game.has_picked() {
            "Picked"
        } else {
            "SD Lua"
        };
        self.draw_screen_header(strip, "FreeCell", status);

        let cell_font = fonts::chrome_font();
        let meta_font = self.card_meta_font();
        let grid_x = LARGE_MARGIN;
        let grid_y = crossink_internal::HEADER_RULE_Y + 18;
        let footer_y = SCREEN_H.saturating_sub(BUTTON_BAR_H + meta_font.line_height + 20);
        let grid_w = FULL_CONTENT_W.saturating_sub(2);
        let available_h = footer_y.saturating_sub(grid_y).saturating_sub(18);
        let cell_w = grid_w / 8;
        let mut cell_h = available_h / 7;
        if cell_h > 62 {
            cell_h = 62;
        }
        if cell_h < 38 {
            cell_h = 38;
        }
        let grid_h = cell_h * 7;

        Region::new(grid_x, grid_y, grid_w, grid_h)
            .to_rect()
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
            .draw(strip)
            .unwrap();

        for row in 0..7usize {
            for col in 0..8usize {
                let cell = Region::new(
                    grid_x + (col as u16).saturating_mul(cell_w),
                    grid_y + (row as u16).saturating_mul(cell_h),
                    cell_w,
                    cell_h,
                );
                self.draw_lua_grid_cell(
                    strip,
                    cell,
                    self.freecell_game.cell_char(row, col),
                    self.freecell_game.is_highlighted(row, col),
                    cell_font,
                );
            }
        }

        let mut footer = BitmapDynLabel::<96>::new(
            Region::new(
                LARGE_MARGIN,
                footer_y,
                FULL_CONTENT_W,
                meta_font.line_height,
            ),
            meta_font,
        )
        .alignment(Alignment::CenterLeft);
        let _ = write!(
            footer,
            "FreeCell: OK pick/drop. Moves: {}",
            self.freecell_game.moves()
        );
        footer.draw(strip).unwrap();
    }

    fn draw_solitaire_game(&self, strip: &mut StripBuffer) {
        let status = if self.solitaire_game.has_picked() {
            "Picked"
        } else {
            "SD Lua"
        };
        self.draw_screen_header(strip, "Solitaire", status);

        let cell_font = fonts::chrome_font();
        let meta_font = self.card_meta_font();
        let grid_x = LARGE_MARGIN;
        let grid_y = crossink_internal::HEADER_RULE_Y + 18;
        let footer_y = SCREEN_H.saturating_sub(BUTTON_BAR_H + meta_font.line_height + 20);
        let grid_w = FULL_CONTENT_W.saturating_sub(2);
        let available_h = footer_y.saturating_sub(grid_y).saturating_sub(18);
        let cell_w = grid_w / 7;
        let mut cell_h = available_h / 7;
        if cell_h > 62 {
            cell_h = 62;
        }
        if cell_h < 38 {
            cell_h = 38;
        }
        let grid_h = cell_h * 7;

        Region::new(grid_x, grid_y, grid_w, grid_h)
            .to_rect()
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
            .draw(strip)
            .unwrap();

        for row in 0..7usize {
            for col in 0..7usize {
                let cell = Region::new(
                    grid_x + (col as u16).saturating_mul(cell_w),
                    grid_y + (row as u16).saturating_mul(cell_h),
                    cell_w,
                    cell_h,
                );
                self.draw_lua_grid_cell(
                    strip,
                    cell,
                    self.solitaire_game.cell_char(row, col),
                    self.solitaire_game.is_highlighted(row, col),
                    cell_font,
                );
            }
        }

        let mut footer = BitmapDynLabel::<96>::new(
            Region::new(
                LARGE_MARGIN,
                footer_y,
                FULL_CONTENT_W,
                meta_font.line_height,
            ),
            meta_font,
        )
        .alignment(Alignment::CenterLeft);
        let _ = write!(
            footer,
            "Solitaire: OK pick/drop. Moves: {}",
            self.solitaire_game.moves()
        );
        footer.draw(strip).unwrap();
    }

    fn draw_ludo_game(&self, strip: &mut StripBuffer) {
        let status = if self.ludo_game.finished() {
            "Done"
        } else {
            "SD Lua"
        };
        self.draw_screen_header(strip, "Ludo", status);

        let cell_font = fonts::chrome_font();
        let meta_font = self.card_meta_font();
        const TRACK_CELLS: usize = 14;
        const TOKEN_LABELS: [char; 4] = ['A', 'B', 'C', 'D'];

        let label_w: u16 = 36;
        let cell_w: u16 = 27;
        let cell_h: u16 = 31;
        let lane_gap: u16 = 12;
        let grid_x = LARGE_MARGIN;
        let track_x = grid_x + label_w + 10;
        let grid_y = crossink_internal::HEADER_RULE_Y + 18;
        let track_w = cell_w.saturating_mul(TRACK_CELLS as u16);

        let mut intro = BitmapDynLabel::<96>::new(
            Region::new(grid_x, grid_y, FULL_CONTENT_W, meta_font.line_height),
            meta_font,
        )
        .alignment(Alignment::CenterLeft);
        let _ = write!(
            intro,
            "Select token, OK rolls. Dice {}  Rolls {}",
            self.ludo_game.last_die(),
            self.ludo_game.rolls()
        );
        intro.draw(strip).unwrap();

        let lanes_y = grid_y + meta_font.line_height + 14;
        for (idx, label) in TOKEN_LABELS.iter().enumerate() {
            let lane_y = lanes_y + (idx as u16).saturating_mul(cell_h + lane_gap);
            let selected = self.ludo_game.selected() as usize == idx;
            let position = self.ludo_game.token_position(idx);
            let progress =
                ((position as usize).saturating_mul(TRACK_CELLS - 1) / 56).min(TRACK_CELLS - 1);

            let label_region = Region::new(grid_x, lane_y, label_w, cell_h);
            self.draw_lua_grid_cell(strip, label_region, *label, selected, cell_font);

            Region::new(track_x, lane_y, track_w, cell_h)
                .to_rect()
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(strip)
                .unwrap();

            for col in 0..TRACK_CELLS {
                let cell = Region::new(
                    track_x + (col as u16).saturating_mul(cell_w),
                    lane_y,
                    cell_w,
                    cell_h,
                );
                let ch = if col == progress {
                    *label
                } else if col == 0 {
                    'S'
                } else if col == TRACK_CELLS - 1 {
                    'F'
                } else {
                    '.'
                };
                self.draw_lua_grid_cell(strip, cell, ch, selected && col == progress, cell_font);
            }

            let mut pos_label = BitmapDynLabel::<24>::new(
                Region::new(track_x + track_w + 8, lane_y + 6, 44, meta_font.line_height),
                meta_font,
            )
            .alignment(Alignment::CenterLeft);
            let _ = write!(pos_label, "{}", position);
            pos_label.draw(strip).unwrap();
        }

        let info_y = lanes_y + 4u16.saturating_mul(cell_h + lane_gap) + 6;
        let mut line = BitmapDynLabel::<112>::new(
            Region::new(LARGE_MARGIN, info_y, FULL_CONTENT_W, meta_font.line_height),
            meta_font,
        )
        .alignment(Alignment::CenterLeft);
        let _ = write!(
            line,
            "Token {} selected. All tokens reach F to finish.",
            self.ludo_game.selected() + 1
        );
        line.draw(strip).unwrap();

        BitmapLabel::new(
            Region::new(
                LARGE_MARGIN,
                SCREEN_H.saturating_sub(BUTTON_BAR_H + meta_font.line_height + 18),
                FULL_CONTENT_W,
                meta_font.line_height,
            ),
            "Arrows choose token. OK rolls.",
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
    }

    fn draw_snakes_game(&self, strip: &mut StripBuffer) {
        let status = if self.snakes_game.won() {
            "Won"
        } else {
            "SD Lua"
        };
        self.draw_screen_header(strip, "Snakes and Ladder", status);
        let cell_font = fonts::chrome_font();
        let meta_font = self.card_meta_font();
        let cell_w: u16 = 40;
        let cell_h: u16 = 40;
        let grid_w = cell_w * 10;
        let grid_h = cell_h * 10;
        let grid_x = SCREEN_W.saturating_sub(grid_w) / 2;
        let grid_y = crossink_internal::HEADER_RULE_Y + 18;
        Region::new(grid_x, grid_y, grid_w, grid_h)
            .to_rect()
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
            .draw(strip)
            .unwrap();

        for row in 0..10usize {
            for col in 0..10usize {
                let cell = Region::new(
                    grid_x + (col as u16).saturating_mul(cell_w),
                    grid_y + (row as u16).saturating_mul(cell_h),
                    cell_w,
                    cell_h,
                );
                self.draw_lua_grid_cell(
                    strip,
                    cell,
                    self.snakes_game.cell_char(row, col),
                    self.snakes_game.is_highlighted(row, col),
                    cell_font,
                );
            }
        }

        let mut line = BitmapDynLabel::<112>::new(
            Region::new(
                LARGE_MARGIN,
                grid_y + grid_h + 12,
                FULL_CONTENT_W,
                meta_font.line_height,
            ),
            meta_font,
        )
        .alignment(Alignment::CenterLeft);
        let _ = write!(
            line,
            "Pos {}  Dice {}  Rolls {}",
            self.snakes_game.position() + 1,
            self.snakes_game.last_die(),
            self.snakes_game.rolls()
        );
        line.draw(strip).unwrap();

        BitmapLabel::new(
            Region::new(
                LARGE_MARGIN,
                SCREEN_H.saturating_sub(BUTTON_BAR_H + meta_font.line_height + 18),
                FULL_CONTENT_W,
                meta_font.line_height,
            ),
            "OK rolls. L ladder, S snake, P player.",
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
    }
    fn draw_dictionary_keyboard_grid(
        &self,
        strip: &mut StripBuffer,
        area: Region,
        key_font: &'static BitmapFont,
        action_font: &'static BitmapFont,
    ) {
        let gap = DICTIONARY_KEYBOARD_GAP;
        let key_w = area
            .w
            .saturating_sub(gap * (DICTIONARY_KEYBOARD_COLS as u16 - 1))
            / DICTIONARY_KEYBOARD_COLS as u16;
        let key_h = area
            .h
            .saturating_sub(gap * (DICTIONARY_KEYBOARD_ROWS as u16 - 1))
            / DICTIONARY_KEYBOARD_ROWS as u16;

        for index in 0..DICTIONARY_KEYBOARD_KEY_COUNT {
            let row = index / DICTIONARY_KEYBOARD_COLS;
            let col = index % DICTIONARY_KEYBOARD_COLS;
            let key = self.dictionary.keyboard_key(index);
            let selected = index == self.dictionary.keyboard_cursor();
            let region = Region::new(
                area.x + col as u16 * (key_w + gap),
                area.y + row as u16 * (key_h + gap),
                key_w,
                key_h,
            );

            if selected {
                region
                    .to_rect()
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(strip)
                    .unwrap();
            } else {
                region
                    .to_rect()
                    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                    .draw(strip)
                    .unwrap();
            }

            let label_font = if key.len() > 1 { action_font } else { key_font };
            BitmapLabel::new(region, key, label_font)
                .alignment(Alignment::Center)
                .inverted(selected)
                .draw(strip)
                .unwrap();
        }
    }

    fn draw_dictionary_tool(&self, strip: &mut StripBuffer) {
        let source = if self.dictionary.loaded_from_sd() {
            "SD Lua"
        } else {
            "Fallback"
        };
        self.draw_screen_header(strip, "Dictionary", source);

        let title_font = fonts::ui_heading_font_fixed();
        let body_font = fonts::ui_body_font_fixed();
        let small_font = fonts::chrome_font();
        let x = crossink_internal::CONTENT_X;
        let w = crossink_internal::CONTENT_W;
        let mut y = crossink_internal::LIST_TOP_NO_TABS;
        let entry = self.dictionary.entry();

        let mut word =
            BitmapDynLabel::<96>::new(Region::new(x, y, w, title_font.line_height), title_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(word, "{}", entry.word());
        word.draw(strip).unwrap();
        y += title_font.line_height + 2;

        let mut meta =
            BitmapDynLabel::<128>::new(Region::new(x, y, w, small_font.line_height), small_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(
            meta,
            "Word {}/{}   {}",
            self.dictionary.selected_index().saturating_add(1),
            self.dictionary.entry_count(),
            self.dictionary.search_label()
        );
        meta.draw(strip).unwrap();
        y += small_font.line_height + 8;

        for line in [
            entry.definition_line1(),
            entry.definition_line2(),
            entry.definition_line3(),
        ] {
            if !line.is_empty() {
                BitmapLabel::new(Region::new(x, y, w, body_font.line_height), line, body_font)
                    .alignment(Alignment::CenterLeft)
                    .draw(strip)
                    .unwrap();
                y += body_font.line_height + 2;
            }
        }

        for line in [entry.synonyms(), entry.antonyms()] {
            if !line.is_empty() {
                BitmapLabel::new(
                    Region::new(x, y, w, small_font.line_height),
                    line,
                    small_font,
                )
                .alignment(Alignment::CenterLeft)
                .draw(strip)
                .unwrap();
                y += small_font.line_height + 2;
            }
        }

        let raw_status = self.dictionary.status();
        let friendly_status = if raw_status.contains("missing")
            || raw_status.contains("parse")
            || raw_status.contains("invalid")
            || raw_status.contains("too large")
        {
            raw_status
        } else if self.dictionary.loaded_from_sd() {
            "Ready - SD dictionary pack"
        } else {
            "Ready - built-in fallback"
        };

        let status_y = SCREEN_H.saturating_sub(BUTTON_BAR_H + small_font.line_height + 14);
        let min_keyboard_top = crossink_internal::HEADER_RULE_Y + 150;
        let max_keyboard_top = status_y.saturating_sub(118);
        let keyboard_top = y
            .saturating_add(12)
            .max(min_keyboard_top)
            .min(max_keyboard_top);
        let keyboard_bottom = status_y.saturating_sub(DICTIONARY_KEYBOARD_STATUS_GAP);
        let keyboard_h = keyboard_bottom.saturating_sub(keyboard_top);

        self.draw_dictionary_keyboard_grid(
            strip,
            Region::new(x, keyboard_top, w, keyboard_h),
            body_font,
            small_font,
        );

        let mut status = BitmapDynLabel::<128>::new(
            Region::new(x, status_y, w, small_font.line_height),
            small_font,
        )
        .alignment(Alignment::CenterLeft);
        let _ = write!(status, "{}", friendly_status);
        status.draw(strip).unwrap();
    }

    fn draw_unit_converter_tool(&self, strip: &mut StripBuffer) {
        self.draw_screen_header(strip, "Unit Converter", "SD Lua");

        let text_font = self.ui_fonts.body;
        let x = LARGE_MARGIN;
        let w = FULL_CONTENT_W;
        let selected = self.unit_converter.selected_index();
        let conversion = self.unit_converter.conversion();
        let mut y = BM_TITLE_Y + self.ui_fonts.heading.line_height + 18;
        let gap = text_font.line_height + 8;

        let mut title =
            BitmapDynLabel::<128>::new(Region::new(x, y, w, text_font.line_height), text_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(
            title,
            "{}: {} -> {}",
            conversion.name, conversion.input_label, conversion.output_label
        );
        title.draw(strip).unwrap();
        y += gap;

        let mut input =
            BitmapDynLabel::<96>::new(Region::new(x, y, w, text_font.line_height), text_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(input, "Input:  {}", self.unit_converter.input_value());
        input.draw(strip).unwrap();
        y += gap;

        let mut result =
            BitmapDynLabel::<96>::new(Region::new(x, y, w, text_font.line_height), text_font)
                .alignment(Alignment::CenterLeft);
        let _ = write!(result, "Result: {}", self.unit_converter.output_value());
        result.draw(strip).unwrap();
        y += gap + 6;

        for idx in 0..UnitConverterState::conversion_count() {
            let row_step = text_font.line_height.saturating_add(4);
            let row_y = y.saturating_add((idx as u16).saturating_mul(row_step));
            let mut row = BitmapDynLabel::<96>::new(
                Region::new(x, row_y, w, text_font.line_height),
                text_font,
            )
            .alignment(Alignment::CenterLeft);
            let marker = if idx == selected { ">" } else { " " };
            let _ = write!(
                row,
                "{} {}",
                marker,
                unit_converter::UNIT_CONVERSIONS[idx].name
            );
            row.draw(strip).unwrap();
        }

        let footer_y = SCREEN_H.saturating_sub(BUTTON_BAR_H + text_font.line_height * 2 + 10);
        BitmapLabel::new(
            Region::new(x, footer_y, w, text_font.line_height),
            "Up/Down conversion. Left/Right/OK amount.",
            text_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();

        BitmapLabel::new(
            Region::new(
                x,
                footer_y + text_font.line_height + 4,
                w,
                text_font.line_height,
            ),
            unit_converter::LUA_UNIT_CONVERTER_PLAYABLE_APP_MARKER,
            text_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
    }

    fn draw_lua_flashcards(&self, strip: &mut StripBuffer) {
        if self.flashcards_screen.is_image_card() && self.flashcards_image_ready {
            strip.blit_1bpp(
                self.flashcards_image_buf.as_slice(),
                FLASHCARDS_X4B_HEADER_LEN,
                FLASHCARDS_X4B_W,
                FLASHCARDS_X4B_H,
                FLASHCARDS_X4B_STRIDE,
                0,
                0,
                true,
            );
            return;
        }
        self.draw_lua_app_text_screen(
            strip,
            self.flashcards_screen.title(),
            self.flashcards_screen.source.label(),
            self.flashcards_screen.topic(),
            self.flashcards_screen.line1(),
            self.flashcards_screen.line2(),
            self.flashcards_screen.line3(),
            self.flashcards_screen.footer(),
        );
    }

    fn draw_lua_tool_stub(&self, strip: &mut StripBuffer) {
        if self.lua_tool_index == 0 {
            self.draw_dictionary_tool(strip);
            return;
        }

        if self.lua_tool_index == 1 {
            self.draw_unit_converter_tool(strip);
            return;
        }

        self.draw_lua_app_text_screen(
            strip,
            self.lua_tool_screen.title(),
            self.lua_tool_screen.source.label(),
            "",
            self.lua_tool_screen.line1(),
            self.lua_tool_screen.line2(),
            self.lua_tool_screen.line3(),
            self.lua_tool_screen.footer(),
        );
    }

    fn draw_lua_game_stub(&self, strip: &mut StripBuffer) {
        match self.lua_game_index {
            0 => {
                self.draw_sudoku_game(strip);
                return;
            }
            1 => {
                self.draw_mines_game(strip);
                return;
            }
            2 => {
                self.draw_freecell_game(strip);
                return;
            }
            3 => {
                self.draw_memory_game(strip);
                return;
            }
            4 => {
                self.draw_solitaire_game(strip);
                return;
            }
            5 => {
                self.draw_ludo_game(strip);
                return;
            }
            6 => {
                self.draw_snakes_game(strip);
                return;
            }

            _ => {}
        }

        self.draw_screen_header(
            strip,
            self.lua_game_screen.title(),
            self.lua_game_screen.source.label(),
        );

        let title_font = self.card_title_font();
        let meta_font = self.card_meta_font();
        let x = LARGE_MARGIN;
        let w = FULL_CONTENT_W;
        let mut y = BM_TITLE_Y + self.ui_fonts.heading.line_height + 28;

        BitmapLabel::new(
            Region::new(x, y, w, title_font.line_height),
            self.lua_game_screen.line1(),
            title_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
        y += title_font.line_height + 12;

        BitmapLabel::new(
            Region::new(x, y, w, meta_font.line_height),
            self.lua_game_screen.line2(),
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
        y += meta_font.line_height + 10;

        BitmapLabel::new(
            Region::new(x, y, w, meta_font.line_height),
            self.lua_game_screen.line3(),
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
        y += meta_font.line_height + 16;

        BitmapLabel::new(
            Region::new(x, y, w, meta_font.line_height),
            self.lua_game_screen.footer(),
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();

        BitmapLabel::new(
            Region::new(
                x,
                SCREEN_H.saturating_sub(BUTTON_BAR_H + 20),
                w,
                meta_font.line_height,
            ),
            game_stub_script::LUA_GAMES_CATALOG_STUB_PACK_MARKER,
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
    }

    fn draw_system_info_row(
        &self,
        strip: &mut StripBuffer,
        y: u16,
        label: &str,
        value: &str,
    ) -> u16 {
        let label_font = self.card_title_font();
        let value_font = self.card_meta_font();
        let row_h = label_font
            .line_height
            .max(value_font.line_height)
            .saturating_add(12);
        let label_w = 158;
        let value_x = LARGE_MARGIN.saturating_add(label_w).saturating_add(8);
        let value_w = FULL_CONTENT_W.saturating_sub(label_w).saturating_sub(8);

        BitmapLabel::new(
            Region::new(
                LARGE_MARGIN,
                y.saturating_add(5),
                label_w,
                label_font.line_height,
            ),
            label,
            label_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();

        BitmapLabel::new(
            Region::new(
                value_x,
                y.saturating_add(5),
                value_w,
                value_font.line_height,
            ),
            value,
            value_font,
        )
        .alignment(Alignment::CenterRight)
        .draw(strip)
        .unwrap();

        y.saturating_add(row_h)
    }

    fn draw_system_info_note(&self, strip: &mut StripBuffer, y: u16, note: &str) -> u16 {
        let font = self.card_meta_font();
        BitmapLabel::new(
            Region::new(LARGE_MARGIN, y, FULL_CONTENT_W, font.line_height),
            note,
            font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
        y.saturating_add(font.line_height).saturating_add(8)
    }

    fn draw_sleep_image_info(&self, strip: &mut StripBuffer) {
        self.draw_screen_header(strip, "Sleep Images", "System");
        let mut y = BM_TITLE_Y
            .saturating_add(self.ui_fonts.heading.line_height)
            .saturating_add(24);

        y = self.draw_system_info_row(strip, y, "Mode", "Random images");
        y = self.draw_system_info_row(strip, y, "Folder", "/RUSTMIX/SLEEP");
        y = self.draw_system_info_row(strip, y, "Names", "SLEEP, SLEEP00..32");
        y = self.draw_system_info_row(strip, y, "Format", "800x480 1-bit BMP");
        y = self.draw_system_info_row(strip, y, "Policy", "folder only");
        let _ = self.draw_system_info_note(
            strip,
            y.saturating_add(6),
            "Upload BMP images over Wi-Fi Transfer into /RUSTMIX/SLEEP.",
        );
    }

    fn draw_device_info(&self, strip: &mut StripBuffer) {
        let status = if self.network_status_loaded {
            "Ready"
        } else {
            "Refresh"
        };
        self.draw_screen_header(strip, "Device Info", status);
        let mut y = BM_TITLE_Y
            .saturating_add(self.ui_fonts.heading.line_height)
            .saturating_add(24);

        y = self.draw_system_info_row(strip, y, "Device", "Xteink X4");
        y = self.draw_system_info_row(strip, y, "MCU", "ESP32-C3");
        y = self.draw_system_info_row(strip, y, "Display", "800x480 SSD1677");
        y = self.draw_system_info_row(strip, y, "UI Font", "Inter e-ink tuned");
        y = self.draw_system_info_row(strip, y, "Home", "Biscuit launcher");
        y = self.draw_system_info_row(strip, y, "Internal UI", "CrossInk style");
        y = self.draw_system_info_row(strip, y, "Reader", "Footer-safe page");

        let sd_status = if self.network_status_loaded {
            if self.network_sd_ok { "OK" } else { "Missing" }
        } else {
            "Select refresh"
        };
        y = self.draw_system_info_row(strip, y, "SD Card", sd_status);

        let mut power_value = alloc::string::String::new();
        if self.network_status_loaded {
            let _ = write!(power_value, "{} mV", self.network_battery_mv);
        } else {
            let _ = write!(power_value, "Select refresh");
        }
        y = self.draw_system_info_row(strip, y, "Battery", power_value.as_str());

        let _ = self.draw_system_info_note(
            strip,
            y.saturating_add(6),
            SYSTEM_ABOUT_DIAGNOSTICS_CROSSINK_POLISH_MARKER,
        );
    }
    fn draw_placeholder(&self, strip: &mut StripBuffer) {
        self.draw_screen_header(strip, self.placeholder_title, "Soon");

        let title_font = self.card_title_font();
        let meta_font = self.card_meta_font();
        let x = LARGE_MARGIN;
        let w = FULL_CONTENT_W;
        let y = BM_TITLE_Y + self.ui_fonts.heading.line_height + 32;

        BitmapLabel::new(
            Region::new(x, y, w, title_font.line_height),
            self.placeholder_detail,
            title_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();

        BitmapLabel::new(
            Region::new(x, y + title_font.line_height + 16, w, meta_font.line_height),
            "Back returns to category",
            meta_font,
        )
        .alignment(Alignment::CenterLeft)
        .draw(strip)
        .unwrap();
    }

    fn draw_bookmarks(&self, strip: &mut StripBuffer) {
        self.draw_screen_header(strip, "Reader", "");
        crossink_internal::draw_reader_tabs_focused(strip, self.reader_tab, self.reader_focus_tabs);

        let mut status = BitmapDynLabel::<20>::new(self.bm_status_region(), fonts::chrome_font())
            .alignment(Alignment::CenterRight);
        match self.reader_tab {
            0 => {
                if self.recent_record.is_some() {
                    let _ = write!(status, "1/1");
                }
            }
            3 => {
                if self.bm_count > 0 {
                    let _ = write!(status, "{}/{}", self.bm_selected + 1, self.bm_count);
                }
            }
            _ => {}
        }
        status.draw(strip).unwrap();

        if self.reader_tab == 0 {
            if let Some(record) = &self.recent_record {
                crossink_internal::draw_reader_compact_item(
                    strip,
                    crossink_internal::reader_list_row_region(0),
                    record.ui_title(),
                    record.format.as_str(),
                    !self.reader_focus_tabs,
                );
            } else {
                crossink_internal::draw_reader_status_message(
                    strip,
                    "No recent book",
                    "Open a book to populate Recent",
                );
            }
            return;
        }

        if matches!(self.reader_tab, 1 | 2) {
            crossink_internal::draw_reader_status_message(strip, "Opening file browser...", "");
            return;
        }

        if self.reader_tab != 3 {
            crossink_internal::draw_reader_status_message(strip, "Reader tab", "");
            return;
        }

        if self.bm_count == 0 {
            crossink_internal::draw_reader_status_message(strip, "No bookmarks yet", "");
            return;
        }

        let grouped_title = self.bm_group_title();

        if let Some(title) = grouped_title {
            let subtitle_text = Self::ellipsize_ascii(title, BM_MAX_TITLE_CHARS);
            crossink_internal::draw_row(
                strip,
                self.bm_subtitle_region(),
                &subtitle_text,
                "",
                crossink_internal::CrossInkRowTone::Section,
            );
        }

        let vis = self.bm_visible_lines();
        let visible = vis.min(self.bm_count.saturating_sub(self.bm_scroll));

        for i in 0..vis {
            let region = self.bm_row_region(i);
            if i < visible {
                let idx = self.bm_scroll + i;
                let entry = &self.bm_entries[idx];
                let label = if grouped_title.is_some() {
                    Self::bm_detail_label(entry)
                } else {
                    Self::bm_mixed_book_label(entry)
                };

                crossink_internal::draw_reader_compact_item(
                    strip,
                    region,
                    &label,
                    "Mark",
                    idx == self.bm_selected && !self.reader_focus_tabs,
                );
            }
        }
    }
}
