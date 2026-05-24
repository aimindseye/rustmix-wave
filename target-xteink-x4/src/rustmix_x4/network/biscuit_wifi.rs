// Rustmix-owned Biscuit-style Wi-Fi setup activity for Xteink X4.
//
// The Wi-Fi setup screen is intentionally isolated from Home. It owns scan,
// profile selection, SSID assignment, password entry, default selection, and
// SETTINGS.TXT persistence.
//
// Rustmix Wi-Fi model:
// - Credentials remain in _x4/SETTINGS.TXT.
// - There is no separate Wi-Fi profile file.
// - Each profile has exactly one SSID/password pair.
// - One profile is marked default and exported through wifi_ssid/wifi_pass for
//   Wi-Fi Transfer and Date/Time sync.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use embassy_time::{Duration, Timer};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::PrimitiveStyle;
use esp_hal::delay::Delay;
use esp_radio::wifi::{Config, ScanConfig, WifiMode};
use log::info;

use crate::rustmix_x4::x4_apps::fonts;
use crate::rustmix_x4::x4_apps::fonts::bitmap::BitmapFont;
use crate::rustmix_x4::x4_apps::ui::{
    Alignment, BitmapLabel, ButtonFeedback, CONTENT_TOP, LARGE_MARGIN, Region,
};
use crate::rustmix_x4::x4_kernel::board::action::{Action, ActionEvent, ButtonMapper};
use crate::rustmix_x4::x4_kernel::board::{Epd, SCREEN_H, SCREEN_W};
use crate::rustmix_x4::x4_kernel::drivers::sdcard::SdStorage;
use crate::rustmix_x4::x4_kernel::drivers::storage;
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;
use crate::rustmix_x4::x4_kernel::kernel::config::{
    self, SystemSettings, WifiConfig, parse_settings_txt,
};
use crate::rustmix_x4::x4_kernel::kernel::tasks;

pub const BISCUIT_WIFI_MARKER: &str = "wifi-profile-ssid-one-to-one-ok";

const MAX_SCAN_RESULTS: usize = 12;
const SETTINGS_READ_BUF: usize = 1536;
const SETTINGS_WRITE_BUF: usize = 2048;
const PROFILE_COUNT: usize = 3;
const PROFILE_NAMES: [&str; PROFILE_COUNT] = ["Home", "Work", "Other"];

const HEADING_X: u16 = LARGE_MARGIN;
const HEADING_W: u16 = SCREEN_W - HEADING_X * 2;
const BODY_X: u16 = 44;
const BODY_W: u16 = SCREEN_W - BODY_X * 2;
const FOOTER_Y: u16 = SCREEN_H - 124;

const SETUP_ROW_TOP: u16 = CONTENT_TOP + 88;
const SETUP_ROW_H: u16 = 46;
const SETUP_ROW_GAP: u16 = 10;
const SETUP_ROW_COUNT: usize = 4;
const ROW_PROFILE: usize = 0;
const ROW_SSID: usize = 1;
const ROW_PASSWORD: usize = 2;
const ROW_DEFAULT: usize = 3;

const LIST_TOP: u16 = CONTENT_TOP + 128;
const LIST_ROW_H: u16 = 42;
const LIST_VISIBLE_ROWS: usize = 5;

const KEYBOARD_TOP: u16 = CONTENT_TOP + 172;
const KEY_H: u16 = 34;
const KEY_GAP: u16 = 4;
const SELECT_RAIL_W: u16 = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SetupState {
    Scanning,
    ProfileSetup,
    SsidPicker,
    PasswordEntry,
    SaveDone,
    ScanFailed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LayoutMode {
    Lower,
    Upper,
    Symbols,
}

#[derive(Clone)]
struct NetworkEntry {
    ssid: String,
    rssi: i32,
    saved: bool,
}

#[derive(Clone)]
struct WifiProfile {
    ssid: String,
    pass: String,
}

impl WifiProfile {
    fn empty() -> Self {
        Self {
            ssid: String::new(),
            pass: String::new(),
        }
    }
}

struct SetupConfig {
    settings: SystemSettings,
    profiles: [WifiProfile; PROFILE_COUNT],
    default_profile: usize,
    swap_buttons: bool,
}

impl SetupConfig {
    fn load(sd: &SdStorage) -> Self {
        let mut settings = SystemSettings::defaults();
        let mut wifi = WifiConfig::empty();
        let mut profiles = [
            WifiProfile::empty(),
            WifiProfile::empty(),
            WifiProfile::empty(),
        ];
        let mut default_profile = 0usize;

        let mut buf = [0u8; SETTINGS_READ_BUF];
        let read_len = match storage::read_chunk_in_x4(sd, config::SETTINGS_FILE, 0, &mut buf) {
            Ok(n) => Some(n),
            Err(_) => match storage::read_file_start(sd, config::SETTINGS_FILE, &mut buf) {
                Ok((_size, n)) => Some(n),
                Err(_) => None,
            },
        };

        if let Some(n) = read_len {
            if n > 0 {
                parse_settings_txt(&buf[..n], &mut settings, &mut wifi);
                parse_wifi_profile_keys(&buf[..n], &mut profiles, &mut default_profile);
            }
        }

        default_profile = default_profile.min(PROFILE_COUNT - 1);
        if profiles[default_profile].ssid.is_empty() && wifi.has_credentials() {
            profiles[default_profile].ssid.push_str(wifi.ssid());
            profiles[default_profile].pass.push_str(wifi.password());
        }

        Self {
            settings,
            profiles,
            default_profile,
            swap_buttons: settings.swap_buttons,
        }
    }

    fn save(&self, sd: &SdStorage) -> bool {
        let mut buf = [0u8; SETTINGS_WRITE_BUF];
        let len = write_settings_txt_with_profiles(self, &mut buf);
        storage::write_in_x4(sd, config::SETTINGS_FILE, &buf[..len]).is_ok()
    }
}

fn wifi_heading_size(idx: u8) -> u8 {
    if idx < 3 {
        3
    } else {
        idx.min(fonts::max_size_idx())
    }
}

fn wifi_body_size(idx: u8) -> u8 {
    if idx < 2 {
        2
    } else {
        idx.min(fonts::max_size_idx())
    }
}

pub async fn run_biscuit_wifi_setup_mode(
    wifi: esp_hal::peripherals::WIFI<'static>,
    epd: &mut Epd,
    strip: &mut StripBuffer,
    delay: &mut Delay,
    sd: &SdStorage,
    ui_font_size_idx: u8,
    bumps: &ButtonFeedback,
) {
    let heading = fonts::heading_font(wifi_heading_size(ui_font_size_idx));
    let body = fonts::body_font(wifi_body_size(ui_font_size_idx));
    let mut session = WifiSetupSession::new(SetupConfig::load(sd));

    render_screen(epd, strip, delay, heading, body, &session, bumps, true).await;

    let scan_result = scan_networks(
        wifi,
        &session.config,
        epd,
        strip,
        delay,
        heading,
        body,
        bumps,
    )
    .await;
    match scan_result {
        Ok(networks) => {
            session.networks = networks;
            session.recompute_saved_flags();
            session.state = SetupState::ProfileSetup;
            session.message = "Choose profile, SSID, password, default";
        }
        Err(msg) => {
            session.state = SetupState::ScanFailed;
            session.message = msg;
        }
    }

    render_screen(epd, strip, delay, heading, body, &session, bumps, false).await;

    let mut mapper = ButtonMapper::new();
    mapper.set_swap(session.config.swap_buttons);

    loop {
        let hw = tasks::INPUT_EVENTS.receive().await;
        let ev = mapper.map_event(hw);
        let needs_redraw = match session.state {
            SetupState::ProfileSetup => session.handle_profile_setup(ev),
            SetupState::SsidPicker => session.handle_ssid_picker(ev),
            SetupState::PasswordEntry => session.handle_password_entry(ev),
            SetupState::SaveDone => {
                if is_back(ev) || is_ok(ev) {
                    return;
                }
                false
            }
            SetupState::ScanFailed => {
                if is_back(ev) || is_ok(ev) || is_right(ev) {
                    return;
                }
                false
            }
            SetupState::Scanning => false,
        };

        if session.exit_requested {
            return;
        }

        if session.settings_save_requested {
            session.settings_save_requested = false;
            session.saved_ok = session.config.save(sd);
            if session.save_done_after_write {
                session.save_done_after_write = false;
                session.state = SetupState::SaveDone;
                session.message = if session.saved_ok {
                    "Saved to /RUSTMIX/SETTINGS.TXT"
                } else {
                    "Save failed; check SD card"
                };
            } else if session.saved_ok {
                session.message = "Saved to /RUSTMIX/SETTINGS.TXT";
            } else {
                session.message = "Save failed; check SD card";
            }
            render_screen(epd, strip, delay, heading, body, &session, bumps, false).await;
            continue;
        }

        if needs_redraw {
            render_screen(epd, strip, delay, heading, body, &session, bumps, false).await;
        }
    }
}

struct WifiSetupSession {
    state: SetupState,
    networks: Vec<NetworkEntry>,
    selected_network: usize,
    scroll: usize,
    config: SetupConfig,
    selected_profile: usize,
    selected_row: usize,
    password: String,
    layout: LayoutMode,
    key_row: usize,
    key_col: usize,
    message: &'static str,
    settings_save_requested: bool,
    save_done_after_write: bool,
    saved_ok: bool,
    exit_requested: bool,
}

impl WifiSetupSession {
    fn new(config: SetupConfig) -> Self {
        let selected_profile = config.default_profile.min(PROFILE_COUNT - 1);
        let password = config.profiles[selected_profile].pass.clone();
        Self {
            state: SetupState::Scanning,
            networks: Vec::new(),
            selected_network: 0,
            scroll: 0,
            config,
            selected_profile,
            selected_row: ROW_PROFILE,
            password,
            layout: LayoutMode::Lower,
            key_row: 0,
            key_col: 0,
            message: "Scanning nearby Wi-Fi networks...",
            settings_save_requested: false,
            save_done_after_write: false,
            saved_ok: false,
            exit_requested: false,
        }
    }

    fn current_profile(&self) -> &WifiProfile {
        &self.config.profiles[self.selected_profile.min(PROFILE_COUNT - 1)]
    }

    fn current_profile_mut(&mut self) -> &mut WifiProfile {
        let idx = self.selected_profile.min(PROFILE_COUNT - 1);
        &mut self.config.profiles[idx]
    }

    fn current_ssid(&self) -> &str {
        self.current_profile().ssid.as_str()
    }

    fn recompute_saved_flags(&mut self) {
        for net in self.networks.iter_mut() {
            net.saved = self
                .config
                .profiles
                .iter()
                .any(|profile| !profile.ssid.is_empty() && profile.ssid == net.ssid);
        }
    }

    fn selected_profile_name(&self) -> &'static str {
        PROFILE_NAMES[self.selected_profile.min(PROFILE_COUNT - 1)]
    }

    fn set_profile(&mut self, idx: usize) {
        let next = idx.min(PROFILE_COUNT - 1);
        self.selected_profile = next;
        self.password = self.config.profiles[next].pass.clone();
        self.message = "Profile selected";
    }

    fn set_default_to_selected(&mut self) {
        let idx = self.selected_profile.min(PROFILE_COUNT - 1);
        if self.config.default_profile == idx {
            self.message = "Default already set";
            return;
        }
        self.config.default_profile = idx;
        self.settings_save_requested = true;
        self.message = "Default profile changed";
    }

    fn handle_profile_setup(&mut self, ev: ActionEvent) -> bool {
        if is_back(ev) {
            self.exit_requested = true;
            return false;
        }
        if is_down(ev) {
            self.selected_row = (self.selected_row + 1).min(SETUP_ROW_COUNT - 1);
            self.message = "OK selects row; Left/Right changes value";
            return true;
        }
        if is_up(ev) {
            self.selected_row = self.selected_row.saturating_sub(1);
            self.message = "OK selects row; Left/Right changes value";
            return true;
        }

        match self.selected_row {
            ROW_PROFILE => {
                if is_left(ev) {
                    let next = if self.selected_profile == 0 {
                        PROFILE_COUNT - 1
                    } else {
                        self.selected_profile - 1
                    };
                    self.set_profile(next);
                    return true;
                }
                if is_right(ev) || is_ok(ev) {
                    let next = (self.selected_profile + 1) % PROFILE_COUNT;
                    self.set_profile(next);
                    return true;
                }
            }
            ROW_SSID => {
                if is_ok(ev) || is_right(ev) {
                    self.state = SetupState::SsidPicker;
                    self.message = "Choose scanned SSID";
                    self.ensure_selected_visible();
                    return true;
                }
                if is_left(ev) {
                    self.current_profile_mut().ssid.clear();
                    self.current_profile_mut().pass.clear();
                    self.password.clear();
                    self.settings_save_requested = true;
                    self.message = "SSID cleared for profile";
                    return true;
                }
            }
            ROW_PASSWORD => {
                if is_ok(ev) || is_right(ev) {
                    if self.current_ssid().is_empty() {
                        self.message = "Select SSID first";
                    } else {
                        self.password = self.current_profile().pass.clone();
                        self.state = SetupState::PasswordEntry;
                        self.layout = LayoutMode::Lower;
                        self.key_row = 0;
                        self.key_col = 0;
                        self.message = "Enter password, choose done to save";
                    }
                    return true;
                }
                if is_left(ev) {
                    self.current_profile_mut().pass.clear();
                    self.password.clear();
                    self.settings_save_requested = true;
                    self.message = "Password cleared";
                    return true;
                }
            }
            ROW_DEFAULT => {
                if is_ok(ev) || is_left(ev) || is_right(ev) {
                    self.set_default_to_selected();
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    fn handle_ssid_picker(&mut self, ev: ActionEvent) -> bool {
        if is_back(ev) {
            self.state = SetupState::ProfileSetup;
            self.message = "SSID selection cancelled";
            return true;
        }
        if is_down(ev) {
            if !self.networks.is_empty() {
                self.selected_network = (self.selected_network + 1).min(self.networks.len() - 1);
                self.ensure_selected_visible();
                self.message = "OK assigns SSID to profile";
                return true;
            }
        }
        if is_up(ev) {
            if !self.networks.is_empty() {
                self.selected_network = self.selected_network.saturating_sub(1);
                self.ensure_selected_visible();
                self.message = "OK assigns SSID to profile";
                return true;
            }
        }
        if is_ok(ev) || is_right(ev) {
            if let Some(net) = self.networks.get(self.selected_network) {
                let ssid = net.ssid.clone();
                let prior_ssid = self.current_profile().ssid.clone();
                let changed = prior_ssid != ssid;
                let existing_pass = self
                    .config
                    .profiles
                    .iter()
                    .find(|profile| {
                        profile.ssid.as_str() == ssid.as_str() && !profile.pass.is_empty()
                    })
                    .map(|profile| profile.pass.clone());
                {
                    let profile = self.current_profile_mut();
                    profile.ssid = ssid;
                    if changed {
                        profile.pass.clear();
                    }
                }
                if changed {
                    self.password.clear();
                }
                if let Some(pass) = existing_pass {
                    self.password = pass.clone();
                    self.current_profile_mut().pass = pass;
                }
                self.recompute_saved_flags();
                self.state = SetupState::ProfileSetup;
                self.selected_row = ROW_PASSWORD;
                self.settings_save_requested = true;
                self.message = "SSID assigned; enter password";
                return true;
            }
            self.message = "No networks found";
            return true;
        }
        false
    }

    fn handle_password_entry(&mut self, ev: ActionEvent) -> bool {
        if is_back(ev) {
            self.password = self.current_profile().pass.clone();
            self.state = SetupState::ProfileSetup;
            self.message = "Password cancelled";
            return true;
        }
        if matches!(
            ev,
            ActionEvent::LongPress(Action::Select) | ActionEvent::LongPress(Action::NextJump)
        ) {
            self.password.pop();
            self.message = "Deleted one character";
            return true;
        }
        if is_down(ev) {
            self.key_row = (self.key_row + 1).min(key_rows(self.layout).len() - 1);
            self.key_col = self
                .key_col
                .min(key_count(self.layout, self.key_row).saturating_sub(1));
            return true;
        }
        if is_up(ev) {
            self.key_row = self.key_row.saturating_sub(1);
            self.key_col = self
                .key_col
                .min(key_count(self.layout, self.key_row).saturating_sub(1));
            return true;
        }
        if is_right(ev) {
            let count = key_count(self.layout, self.key_row).max(1);
            self.key_col = (self.key_col + 1) % count;
            return true;
        }
        if is_left(ev) {
            let count = key_count(self.layout, self.key_row).max(1);
            self.key_col = if self.key_col == 0 {
                count - 1
            } else {
                self.key_col - 1
            };
            return true;
        }
        if is_ok(ev) {
            let label = key_at(self.layout, self.key_row, self.key_col).unwrap_or("done");
            match label {
                "ABC" => self.layout = LayoutMode::Upper,
                "abc" => self.layout = LayoutMode::Lower,
                "123" => self.layout = LayoutMode::Symbols,
                "space" => {
                    if self.password.len() < config::WIFI_PASS_CAP {
                        self.password.push(' ');
                    }
                }
                "del" => {
                    self.password.pop();
                }
                "clear" => {
                    self.password.clear();
                }
                "done" => {
                    let pass = self.password.clone();
                    self.current_profile_mut().pass = pass;
                    self.settings_save_requested = true;
                    self.save_done_after_write = false;
                    self.state = SetupState::ProfileSetup;
                    self.selected_row = ROW_DEFAULT;
                    self.message = "Saving password...";
                }
                ch => {
                    if self.password.len() < config::WIFI_PASS_CAP {
                        self.password.push_str(ch);
                    }
                }
            }
            self.key_row = self.key_row.min(key_rows(self.layout).len() - 1);
            self.key_col = self
                .key_col
                .min(key_count(self.layout, self.key_row).saturating_sub(1));
            if !self.settings_save_requested {
                self.message = "Password updated";
            }
            return true;
        }
        false
    }

    fn ensure_selected_visible(&mut self) {
        if self.selected_network < self.scroll {
            self.scroll = self.selected_network;
        }
        if self.selected_network >= self.scroll + LIST_VISIBLE_ROWS {
            self.scroll = self.selected_network + 1 - LIST_VISIBLE_ROWS;
        }
    }
}

async fn scan_networks(
    wifi: esp_hal::peripherals::WIFI<'static>,
    cfg: &SetupConfig,
    epd: &mut Epd,
    strip: &mut StripBuffer,
    delay: &mut Delay,
    heading: &'static BitmapFont,
    body: &'static BitmapFont,
    bumps: &ButtonFeedback,
) -> Result<Vec<NetworkEntry>, &'static str> {
    let radio = match esp_radio::init() {
        Ok(r) => r,
        Err(e) => {
            info!("biscuit-wifi: radio init failed: {:?}", e);
            return Err("Radio init failed");
        }
    };

    let (mut wifi_ctrl, _interfaces) = match esp_radio::wifi::new(&radio, wifi, Config::default()) {
        Ok(pair) => pair,
        Err(e) => {
            info!("biscuit-wifi: wifi::new failed: {:?}", e);
            return Err("Wi-Fi init failed");
        }
    };

    if let Err(e) = wifi_ctrl.set_mode(WifiMode::Sta) {
        info!("biscuit-wifi: set_mode failed: {:?}", e);
    }

    if let Err(e) = wifi_ctrl.start_async().await {
        info!("biscuit-wifi: start failed: {:?}", e);
        return Err("Wi-Fi start failed");
    }

    let mut scan_session = WifiSetupSession::new(SetupConfig {
        settings: cfg.settings,
        profiles: cfg.profiles.clone(),
        default_profile: cfg.default_profile,
        swap_buttons: cfg.swap_buttons,
    });
    scan_session.message = "Scanning nearby Wi-Fi networks...";
    render_screen(
        epd,
        strip,
        delay,
        heading,
        body,
        &scan_session,
        bumps,
        false,
    )
    .await;

    let scan_cfg = ScanConfig::default()
        .with_show_hidden(false)
        .with_max(MAX_SCAN_RESULTS * 2);
    let aps = match wifi_ctrl.scan_with_config_async(scan_cfg).await {
        Ok(list) => list,
        Err(e) => {
            info!("biscuit-wifi: scan failed: {:?}", e);
            let _ = wifi_ctrl.stop_async().await;
            return Err("Scan failed; retry near router");
        }
    };

    let mut networks: Vec<NetworkEntry> = Vec::new();
    for ap in aps.iter() {
        let ssid = ap.ssid.as_str();
        if ssid.is_empty() || networks.iter().any(|n| n.ssid == ssid) {
            continue;
        }
        let saved = cfg
            .profiles
            .iter()
            .any(|p| p.ssid == ssid && !p.pass.is_empty());
        networks.push(NetworkEntry {
            ssid: String::from(ssid),
            rssi: ap.signal_strength as i32,
            saved,
        });
        if networks.len() >= MAX_SCAN_RESULTS {
            break;
        }
    }
    networks.sort_by(|a, b| {
        if a.saved != b.saved {
            b.saved.cmp(&a.saved)
        } else {
            b.rssi.cmp(&a.rssi)
        }
    });

    let _ = wifi_ctrl.stop_async().await;
    Timer::after(Duration::from_millis(50)).await;

    Ok(networks)
}

fn is_ok(ev: ActionEvent) -> bool {
    matches!(ev, ActionEvent::Press(Action::Select))
}

fn is_back(ev: ActionEvent) -> bool {
    matches!(
        ev,
        ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back)
    )
}

fn is_up(ev: ActionEvent) -> bool {
    matches!(
        ev,
        ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev)
    )
}

fn is_down(ev: ActionEvent) -> bool {
    matches!(
        ev,
        ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next)
    )
}

fn is_left(ev: ActionEvent) -> bool {
    matches!(
        ev,
        ActionEvent::Press(Action::PrevJump) | ActionEvent::Repeat(Action::PrevJump)
    )
}

fn is_right(ev: ActionEvent) -> bool {
    matches!(
        ev,
        ActionEvent::Press(Action::NextJump) | ActionEvent::Repeat(Action::NextJump)
    )
}

const LOWER_ROW0: [&str; 10] = ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"];
const LOWER_ROW1: [&str; 9] = ["a", "s", "d", "f", "g", "h", "j", "k", "l"];
const LOWER_ROW2: [&str; 10] = ["z", "x", "c", "v", "b", "n", "m", ".", "-", "_"];
const LOWER_ROW3: [&str; 6] = ["ABC", "123", "space", "del", "clear", "done"];

const UPPER_ROW0: [&str; 10] = ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"];
const UPPER_ROW1: [&str; 9] = ["A", "S", "D", "F", "G", "H", "J", "K", "L"];
const UPPER_ROW2: [&str; 10] = ["Z", "X", "C", "V", "B", "N", "M", ".", "-", "_"];
const UPPER_ROW3: [&str; 6] = ["abc", "123", "space", "del", "clear", "done"];

const SYMBOL_ROW0: [&str; 10] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"];
const SYMBOL_ROW1: [&str; 10] = ["!", "@", "#", "$", "%", "^", "&", "*", "(", ")"];
const SYMBOL_ROW2: [&str; 10] = ["+", "=", "/", "?", ":", ";", "'", "\"", ",", "~"];
const SYMBOL_ROW3: [&str; 6] = ["abc", "ABC", "space", "del", "clear", "done"];

fn key_rows(layout: LayoutMode) -> &'static [&'static [&'static str]] {
    match layout {
        LayoutMode::Lower => &[&LOWER_ROW0, &LOWER_ROW1, &LOWER_ROW2, &LOWER_ROW3],
        LayoutMode::Upper => &[&UPPER_ROW0, &UPPER_ROW1, &UPPER_ROW2, &UPPER_ROW3],
        LayoutMode::Symbols => &[&SYMBOL_ROW0, &SYMBOL_ROW1, &SYMBOL_ROW2, &SYMBOL_ROW3],
    }
}

fn key_count(layout: LayoutMode, row: usize) -> usize {
    key_rows(layout).get(row).map(|r| r.len()).unwrap_or(0)
}

fn key_at(layout: LayoutMode, row: usize, col: usize) -> Option<&'static str> {
    key_rows(layout).get(row).and_then(|r| r.get(col).copied())
}

async fn render_screen(
    epd: &mut Epd,
    strip: &mut StripBuffer,
    delay: &mut Delay,
    heading: &'static BitmapFont,
    body: &'static BitmapFont,
    session: &WifiSetupSession,
    bumps: &ButtonFeedback,
    full_refresh: bool,
) {
    let draw = |s: &mut StripBuffer| {
        Region::new(0, 0, SCREEN_W, SCREEN_H)
            .to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(s)
            .unwrap();

        let title = match session.state {
            SetupState::Scanning => "Wi-Fi Networks",
            SetupState::ProfileSetup => "Wi-Fi Setup",
            SetupState::SsidPicker => "Select SSID",
            SetupState::PasswordEntry => "Wi-Fi Password",
            SetupState::SaveDone => "Wi-Fi Saved",
            SetupState::ScanFailed => "Wi-Fi Scan Failed",
        };
        BitmapLabel::new(
            Region::new(HEADING_X, CONTENT_TOP + 8, HEADING_W, heading.line_height),
            title,
            heading,
        )
        .alignment(Alignment::CenterLeft)
        .draw(s)
        .unwrap();

        match session.state {
            SetupState::Scanning => draw_center_lines(
                s,
                body,
                &["Scanning nearby Wi-Fi networks...", "Please wait."],
                session.message,
            ),
            SetupState::ScanFailed => draw_center_lines(
                s,
                body,
                &[
                    session.message,
                    "OK/Right exits; reopen to retry",
                    "Back exits",
                ],
                "",
            ),
            SetupState::ProfileSetup => draw_profile_setup(s, body, session),
            SetupState::SsidPicker => draw_ssid_picker(s, body, session),
            SetupState::PasswordEntry => draw_password_entry(s, body, session),
            SetupState::SaveDone => draw_center_lines(
                s,
                body,
                &[
                    session.message,
                    "Wi-Fi Transfer uses default profile",
                    "OK or Back returns",
                ],
                "",
            ),
        }

        bumps.draw(s);
    };

    if full_refresh {
        epd.full_refresh_async(strip, delay, &draw).await;
    } else {
        epd.partial_refresh_async(strip, delay, 0, 0, SCREEN_W, SCREEN_H, &draw)
            .await;
    }
}

fn draw_center_lines(s: &mut StripBuffer, body: &'static BitmapFont, lines: &[&str], footer: &str) {
    let stride = body.line_height + 12;
    let total_h = if lines.is_empty() {
        0
    } else {
        (lines.len() as u16 - 1) * stride + body.line_height
    };
    let y0 = CONTENT_TOP + 110 + (220u16.saturating_sub(total_h) / 2);
    for (i, line) in lines.iter().enumerate() {
        BitmapLabel::new(
            Region::new(BODY_X, y0 + i as u16 * stride, BODY_W, body.line_height),
            line,
            body,
        )
        .alignment(Alignment::Center)
        .draw(s)
        .unwrap();
    }
    if !footer.is_empty() {
        BitmapLabel::new(
            Region::new(
                BODY_X,
                FOOTER_Y - body.line_height - 8,
                BODY_W,
                body.line_height,
            ),
            footer,
            body,
        )
        .alignment(Alignment::Center)
        .draw(s)
        .unwrap();
    }
}

fn draw_profile_setup(s: &mut StripBuffer, body: &'static BitmapFont, session: &WifiSetupSession) {
    for row in 0..SETUP_ROW_COUNT {
        let y = SETUP_ROW_TOP + row as u16 * (SETUP_ROW_H + SETUP_ROW_GAP);
        draw_setup_row(s, body, session, row, y);
    }

    BitmapLabel::new(
        Region::new(BODY_X, FOOTER_Y, BODY_W, body.line_height),
        "Up/Down row   OK edit/select",
        body,
    )
    .alignment(Alignment::Center)
    .draw(s)
    .unwrap();
    BitmapLabel::new(
        Region::new(
            BODY_X,
            FOOTER_Y + body.line_height + 8,
            BODY_W,
            body.line_height,
        ),
        "Left/Right change   Back exits",
        body,
    )
    .alignment(Alignment::Center)
    .draw(s)
    .unwrap();
}

fn draw_setup_row(
    s: &mut StripBuffer,
    body: &'static BitmapFont,
    session: &WifiSetupSession,
    row: usize,
    y: u16,
) {
    let selected = row == session.selected_row;
    let r = Region::new(BODY_X, y, BODY_W, SETUP_ROW_H);
    r.to_rect()
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(s)
        .unwrap();
    if selected {
        Region::new(BODY_X, y, SELECT_RAIL_W, SETUP_ROW_H)
            .to_rect()
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(s)
            .unwrap();
    }

    let mut line = String::new();
    match row {
        ROW_PROFILE => {
            let _ = write!(line, "Profile: {}", session.selected_profile_name());
        }
        ROW_SSID => {
            let ssid = session.current_ssid();
            if ssid.is_empty() {
                let _ = write!(line, "SSID: Scan SSID and select one");
            } else {
                let _ = write!(line, "SSID: {}", truncate_ssid(ssid));
            }
        }
        ROW_PASSWORD => {
            let pass_len = session.current_profile().pass.len();
            if pass_len == 0 {
                let _ = write!(line, "Password: (empty)");
            } else {
                let _ = write!(line, "Password: set ({} chars)", pass_len);
            }
        }
        ROW_DEFAULT => {
            let yes = session.config.default_profile == session.selected_profile;
            let _ = write!(line, "Default: {}", if yes { "Yes" } else { "No" });
        }
        _ => {}
    }

    BitmapLabel::new(
        Region::new(
            BODY_X + SELECT_RAIL_W + 10,
            y + 9,
            BODY_W - SELECT_RAIL_W - 20,
            body.line_height,
        ),
        line.as_str(),
        body,
    )
    .alignment(Alignment::CenterLeft)
    .draw(s)
    .unwrap();
}

fn draw_ssid_picker(s: &mut StripBuffer, body: &'static BitmapFont, session: &WifiSetupSession) {
    let mut profile_line = String::new();
    let _ = write!(profile_line, "Profile: {}", session.selected_profile_name());
    BitmapLabel::new(
        Region::new(BODY_X, CONTENT_TOP + 70, BODY_W, body.line_height),
        profile_line.as_str(),
        body,
    )
    .alignment(Alignment::CenterLeft)
    .draw(s)
    .unwrap();

    if session.networks.is_empty() {
        draw_center_lines(
            s,
            body,
            &["No networks found", "Back returns to setup"],
            session.message,
        );
        return;
    }

    let end = (session.scroll + LIST_VISIBLE_ROWS).min(session.networks.len());
    for (visible, idx) in (session.scroll..end).enumerate() {
        let y = LIST_TOP + visible as u16 * LIST_ROW_H;
        let row_h = LIST_ROW_H - 6;
        let r = Region::new(BODY_X, y, BODY_W, row_h);
        let selected = idx == session.selected_network;
        r.to_rect()
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(s)
            .unwrap();
        if selected {
            Region::new(BODY_X, y, SELECT_RAIL_W, row_h)
                .to_rect()
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(s)
                .unwrap();
        }

        let mut line = String::new();
        let net = &session.networks[idx];
        let _ = write!(
            line,
            "{}{} {}dBm",
            if net.saved { "saved " } else { "" },
            truncate_ssid(net.ssid.as_str()),
            net.rssi
        );
        BitmapLabel::new(
            Region::new(
                BODY_X + SELECT_RAIL_W + 10,
                y + 5,
                BODY_W - SELECT_RAIL_W - 16,
                body.line_height,
            ),
            line.as_str(),
            body,
        )
        .alignment(Alignment::CenterLeft)
        .draw(s)
        .unwrap();
    }

    BitmapLabel::new(
        Region::new(BODY_X, FOOTER_Y, BODY_W, body.line_height),
        "Up/Down network   OK assign SSID",
        body,
    )
    .alignment(Alignment::Center)
    .draw(s)
    .unwrap();
    BitmapLabel::new(
        Region::new(
            BODY_X,
            FOOTER_Y + body.line_height + 8,
            BODY_W,
            body.line_height,
        ),
        "Back returns to setup",
        body,
    )
    .alignment(Alignment::Center)
    .draw(s)
    .unwrap();
}

fn draw_password_entry(s: &mut StripBuffer, body: &'static BitmapFont, session: &WifiSetupSession) {
    let info_top = CONTENT_TOP + 68;
    let mut profile_line = String::new();
    let _ = write!(profile_line, "Profile: {}", session.selected_profile_name());
    BitmapLabel::new(
        Region::new(BODY_X, info_top, BODY_W, body.line_height),
        profile_line.as_str(),
        body,
    )
    .alignment(Alignment::CenterLeft)
    .draw(s)
    .unwrap();

    let mut ssid_line = String::new();
    let _ = write!(ssid_line, "SSID: {}", truncate_ssid(session.current_ssid()));
    BitmapLabel::new(
        Region::new(
            BODY_X,
            info_top + body.line_height + 8,
            BODY_W,
            body.line_height,
        ),
        ssid_line.as_str(),
        body,
    )
    .alignment(Alignment::CenterLeft)
    .draw(s)
    .unwrap();

    let mut pass_line = String::new();
    let pass_text = if session.password.is_empty() {
        "(empty)"
    } else {
        session.password.as_str()
    };
    let _ = write!(pass_line, "Password: {}", pass_text);
    BitmapLabel::new(
        Region::new(
            BODY_X,
            info_top + (body.line_height + 8) * 2,
            BODY_W,
            body.line_height,
        ),
        pass_line.as_str(),
        body,
    )
    .alignment(Alignment::CenterLeft)
    .draw(s)
    .unwrap();

    draw_keyboard(s, body, session);

    BitmapLabel::new(
        Region::new(BODY_X, FOOTER_Y, BODY_W, body.line_height),
        "OK key   Hold OK delete",
        body,
    )
    .alignment(Alignment::Center)
    .draw(s)
    .unwrap();
    BitmapLabel::new(
        Region::new(
            BODY_X,
            FOOTER_Y + body.line_height + 8,
            BODY_W,
            body.line_height,
        ),
        "Back cancel   done saves",
        body,
    )
    .alignment(Alignment::Center)
    .draw(s)
    .unwrap();
}

fn draw_keyboard(s: &mut StripBuffer, body: &'static BitmapFont, session: &WifiSetupSession) {
    let rows = key_rows(session.layout);
    for (row_idx, row) in rows.iter().enumerate() {
        let y = KEYBOARD_TOP + row_idx as u16 * (KEY_H + KEY_GAP);
        let total_gap = KEY_GAP * (row.len() as u16 - 1);
        let key_w = (SCREEN_W - BODY_X * 2 - total_gap) / row.len() as u16;
        let mut x = BODY_X;
        for (col_idx, label) in row.iter().enumerate() {
            let selected = row_idx == session.key_row && col_idx == session.key_col;
            let r = Region::new(x, y, key_w, KEY_H);
            r.to_rect()
                .into_styled(if selected {
                    PrimitiveStyle::with_fill(BinaryColor::On)
                } else {
                    PrimitiveStyle::with_stroke(BinaryColor::On, 1)
                })
                .draw(s)
                .unwrap();
            BitmapLabel::new(
                Region::new(x + 2, y + 5, key_w.saturating_sub(4), body.line_height),
                label,
                body,
            )
            .alignment(Alignment::Center)
            .inverted(selected)
            .draw(s)
            .unwrap();
            x += key_w + KEY_GAP;
        }
    }
}

fn truncate_ssid(ssid: &str) -> &str {
    ssid
}

fn parse_wifi_profile_keys(
    data: &[u8],
    profiles: &mut [WifiProfile; PROFILE_COUNT],
    default_profile: &mut usize,
) {
    for line in data.split(|&b| b == b'\n') {
        let line = trim_ascii(line);
        if line.is_empty() || line[0] == b'#' {
            continue;
        }
        let Some(eq) = line.iter().position(|&b| b == b'=') else {
            continue;
        };
        let key = trim_ascii(&line[..eq]);
        let value = trim_ascii(&line[eq + 1..]);
        if key == b"wifi_default" {
            if let Some(v) = parse_usize(value) {
                *default_profile = v.min(PROFILE_COUNT - 1);
            }
            continue;
        }
        for idx in 0..PROFILE_COUNT {
            let mut ssid_key = String::new();
            let mut pass_key = String::new();
            let _ = write!(ssid_key, "wifi_profile_{}_ssid", idx);
            let _ = write!(pass_key, "wifi_profile_{}_pass", idx);
            if key == ssid_key.as_bytes() {
                profiles[idx].ssid.clear();
                profiles[idx]
                    .ssid
                    .push_str(core::str::from_utf8(value).unwrap_or(""));
            } else if key == pass_key.as_bytes() {
                profiles[idx].pass.clear();
                profiles[idx]
                    .pass
                    .push_str(core::str::from_utf8(value).unwrap_or(""));
            }
        }
    }
}

fn write_settings_txt_with_profiles(cfg: &SetupConfig, buf: &mut [u8]) -> usize {
    let mut writer = LimitedWriter { buf, pos: 0 };
    let active = cfg.default_profile.min(PROFILE_COUNT - 1);
    let active_profile = &cfg.profiles[active];

    let _ = writeln!(writer, "# rustmix-os settings");
    let _ = writeln!(writer, "# lines starting with # are ignored");
    let _ = writeln!(writer);
    let _ = writeln!(writer, "# power settings");
    let _ = writeln!(writer, "sleep_timeout={}", cfg.settings.sleep_timeout);
    let _ = writeln!(writer, "ghost_clear={}", cfg.settings.ghost_clear_every);
    let _ = writeln!(writer);
    let _ = writeln!(writer, "# font settings");
    let _ = writeln!(writer, "book_font={}", cfg.settings.book_font_size_idx);
    let _ = writeln!(writer, "ui_font={}", cfg.settings.ui_font_size_idx);
    let _ = writeln!(writer);
    let _ = writeln!(writer, "# reading settings");
    let _ = writeln!(writer, "reading_theme={}", cfg.settings.reading_theme);
    let _ = writeln!(
        writer,
        "show_progress={}",
        if cfg.settings.reader_show_progress {
            1
        } else {
            0
        }
    );
    let _ = writeln!(
        writer,
        "prepared_font_profile={}",
        cfg.settings.prepared_font_profile
    );
    let _ = writeln!(
        writer,
        "prepared_fallback_policy={}",
        cfg.settings.prepared_fallback_policy
    );
    let _ = writeln!(writer);
    let _ = writeln!(writer, "# display preferences");
    let _ = writeln!(
        writer,
        "display_refresh_mode={}",
        cfg.settings.display_refresh_mode
    );
    let _ = writeln!(
        writer,
        "display_invert_colors={}",
        if cfg.settings.display_invert_colors {
            1
        } else {
            0
        }
    );
    let _ = writeln!(
        writer,
        "display_contrast_high={}",
        if cfg.settings.display_contrast_high {
            1
        } else {
            0
        }
    );
    let _ = writeln!(writer);
    let _ = writeln!(writer, "# control settings");
    let _ = writeln!(
        writer,
        "swap_buttons={}",
        if cfg.settings.swap_buttons { 1 } else { 0 }
    );
    let _ = writeln!(writer);
    let _ = writeln!(
        writer,
        "# active wifi credentials for Wi-Fi Transfer and Date/Time sync"
    );
    let _ = writeln!(writer, "wifi_ssid={}", active_profile.ssid);
    let _ = writeln!(writer, "wifi_pass={}", active_profile.pass);
    let _ = writeln!(writer, "wifi_default={}", active);
    let _ = writeln!(writer);
    let _ = writeln!(
        writer,
        "# saved wifi profiles; each profile owns exactly one SSID/password pair"
    );
    for idx in 0..PROFILE_COUNT {
        let _ = writeln!(writer, "wifi_profile_{}_name={}", idx, PROFILE_NAMES[idx]);
        let _ = writeln!(
            writer,
            "wifi_profile_{}_ssid={}",
            idx, cfg.profiles[idx].ssid
        );
        let _ = writeln!(
            writer,
            "wifi_profile_{}_pass={}",
            idx, cfg.profiles[idx].pass
        );
    }
    writer.pos
}

struct LimitedWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl core::fmt::Write for LimitedWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.buf.len().saturating_sub(self.pos);
        let n = bytes.len().min(remaining);
        if n > 0 {
            self.buf[self.pos..self.pos + n].copy_from_slice(&bytes[..n]);
            self.pos += n;
        }
        Ok(())
    }
}

fn trim_ascii(mut input: &[u8]) -> &[u8] {
    while let Some((&first, rest)) = input.split_first() {
        if matches!(first, b' ' | b'\t' | b'\r') {
            input = rest;
        } else {
            break;
        }
    }
    while let Some((&last, rest)) = input.split_last() {
        if matches!(last, b' ' | b'\t' | b'\r') {
            input = rest;
        } else {
            break;
        }
    }
    input
}

fn parse_usize(input: &[u8]) -> Option<usize> {
    if input.is_empty() {
        return None;
    }
    let mut out = 0usize;
    for &b in input {
        if !b.is_ascii_digit() {
            return None;
        }
        out = out.checked_mul(10)?.checked_add((b - b'0') as usize)?;
    }
    Some(out)
}
