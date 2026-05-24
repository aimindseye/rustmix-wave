// Biscuit-style Wi-Fi setup activity for Xteink X4.
//
// This replaces the fragile Home-screen inline Wi-Fi keyboard with an isolated
// setup flow similar to Biscuit/CrossPoint:
//   scan networks -> choose SSID -> enter password -> save profile/default.
//
// The activity owns Wi-Fi radio use, keyboard buffer state, and drawing while
// active.  Home is only responsible for launching it.

use alloc::string::{String, ToString};
use core::fmt::Write as _;

use embassy_time::{Duration, Timer};
use esp_hal::delay::Delay;
use esp_radio::wifi::{Config, ScanConfig, WifiMode};
use log::info;

use crate::rustmix_x4::x4_apps::apps::widgets::text_keyboard::{self, TextKeyboardAction};
use crate::rustmix_x4::x4_apps::fonts;
use crate::rustmix_x4::x4_apps::fonts::bitmap::BitmapFont;
use crate::rustmix_x4::x4_apps::ui::{
    Alignment, BitmapDynLabel, BitmapLabel, ButtonFeedback, CONTENT_TOP, LARGE_MARGIN, Region,
};
use crate::rustmix_x4::x4_kernel::board::action::{Action, ActionEvent, ButtonMapper};
use crate::rustmix_x4::x4_kernel::board::{Epd, SCREEN_H, SCREEN_W};
use crate::rustmix_x4::x4_kernel::drivers::sdcard::SdStorage;
use crate::rustmix_x4::x4_kernel::drivers::storage;
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;
use crate::rustmix_x4::x4_kernel::kernel::config::{
    self, SystemSettings, WIFI_PASS_CAP, WIFI_PROFILE_COUNT, WIFI_SSID_CAP, WifiConfig,
    parse_settings_txt, write_settings_txt,
};
use crate::rustmix_x4::x4_kernel::kernel::tasks;

pub const WIFI_SCAN_FILE: &str = "WIFISCAN.TXT";
pub const WIFI_SCAN_MARKER: &str = "biscuit-wifi-setup-activity-ok";

const HEADING_X: u16 = LARGE_MARGIN;
const HEADING_W: u16 = SCREEN_W - HEADING_X * 2;
const BODY_X: u16 = 24;
const BODY_W: u16 = SCREEN_W - BODY_X * 2;
const BODY_LINE_GAP: u16 = 8;
const FOOTER_Y: u16 = SCREEN_H - 58;
const MAX_SCAN_RESULTS: usize = 8;
const LIST_VISIBLE_ROWS: usize = 6;
const KEYBOARD_Y: u16 = 214;
const KEYBOARD_H: u16 = text_keyboard::TEXT_KEYBOARD_LARGE_HEIGHT;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WifiSetupState {
    NetworkList,
    PasswordKeyboard,
    Saved,
    Error,
}

struct WifiSetupSession {
    settings: SystemSettings,
    wifi: WifiConfig,
    profile_slot: usize,
    selected_network: usize,
    scan_count: usize,
    scan_ssids: [[u8; WIFI_SSID_CAP]; MAX_SCAN_RESULTS],
    scan_lens: [u8; MAX_SCAN_RESULTS],
    selected_ssid: String,
    password: String,
    keyboard_layout: u8,
    keyboard_index: u8,
    message: &'static str,
    state: WifiSetupState,
}

impl WifiSetupSession {
    fn new(sd: &SdStorage) -> Self {
        let (settings, wifi) = read_settings(sd);
        let default_slot = wifi.default_slot() as usize;
        Self {
            settings,
            wifi,
            profile_slot: default_slot.min(WIFI_PROFILE_COUNT - 1),
            selected_network: 0,
            scan_count: 0,
            scan_ssids: [[0u8; WIFI_SSID_CAP]; MAX_SCAN_RESULTS],
            scan_lens: [0u8; MAX_SCAN_RESULTS],
            selected_ssid: String::new(),
            password: String::new(),
            keyboard_layout: text_keyboard::LAYOUT_LOWER,
            keyboard_index: text_keyboard::default_index(),
            message: "Scanning networks...",
            state: WifiSetupState::NetworkList,
        }
    }

    fn profile_name(&self) -> &str {
        self.wifi.profile_name(self.profile_slot)
    }

    fn selected_ssid_str(&self) -> &str {
        self.ssid_at(self.selected_network).unwrap_or("")
    }

    fn ssid_at(&self, idx: usize) -> Option<&str> {
        if idx >= self.scan_count {
            return None;
        }
        let n = self.scan_lens[idx] as usize;
        core::str::from_utf8(&self.scan_ssids[idx][..n]).ok()
    }

    fn clear_scan_results(&mut self) {
        self.scan_ssids = [[0u8; WIFI_SSID_CAP]; MAX_SCAN_RESULTS];
        self.scan_lens = [0u8; MAX_SCAN_RESULTS];
        self.scan_count = 0;
        self.selected_network = 0;
    }

    fn contains_ssid(&self, ssid: &str) -> bool {
        let mut idx = 0usize;
        while idx < self.scan_count {
            if self.ssid_at(idx) == Some(ssid) {
                return true;
            }
            idx += 1;
        }
        false
    }

    fn push_scan_ssid(&mut self, ssid: &str) {
        if ssid.is_empty() || self.scan_count >= MAX_SCAN_RESULTS || self.contains_ssid(ssid) {
            return;
        }
        let idx = self.scan_count;
        let bytes = ssid.as_bytes();
        let n = bytes.len().min(WIFI_SSID_CAP);
        self.scan_ssids[idx][..n].copy_from_slice(&bytes[..n]);
        self.scan_lens[idx] = n as u8;
        self.scan_count += 1;
    }

    fn cycle_profile(&mut self, delta: isize) {
        self.profile_slot =
            (self.profile_slot as isize + delta).rem_euclid(WIFI_PROFILE_COUNT as isize) as usize;
        self.message = "Profile changed; OK selects SSID";
    }

    fn move_network(&mut self, delta: isize) {
        if self.scan_count == 0 {
            self.message = "No networks found; Right rescans";
            return;
        }
        self.selected_network =
            (self.selected_network as isize + delta).rem_euclid(self.scan_count as isize) as usize;
        self.message = "OK selects network";
    }

    fn open_keyboard_for_selected(&mut self) {
        if self.scan_count == 0 {
            self.message = "No networks; Right rescans";
            return;
        }
        self.selected_ssid = self.selected_ssid_str().to_string();
        self.password.clear();
        if self.wifi.profile_ssid(self.profile_slot) == self.selected_ssid.as_str() {
            self.password
                .push_str(self.wifi.profile_password(self.profile_slot));
        }
        self.keyboard_layout = text_keyboard::LAYOUT_LOWER;
        self.keyboard_index = text_keyboard::default_index();
        self.message = "Enter password; done saves";
        self.state = WifiSetupState::PasswordKeyboard;
    }

    fn move_key_horizontal(&mut self, delta: isize) {
        self.keyboard_index =
            text_keyboard::move_horizontal(self.keyboard_layout, self.keyboard_index, delta);
    }

    fn move_key_vertical(&mut self, delta: isize) {
        self.keyboard_index =
            text_keyboard::move_vertical(self.keyboard_layout, self.keyboard_index, delta);
    }

    fn delete_password_char(&mut self) {
        let _ = self.password.pop();
        self.message = "Deleted one character";
    }

    fn activate_key(&mut self, sd: &SdStorage) {
        match text_keyboard::activate(self.keyboard_layout, self.keyboard_index) {
            TextKeyboardAction::Insert(ch) => {
                if self.password.len() < WIFI_PASS_CAP {
                    self.password.push(ch as char);
                    self.message = "Password updated";
                } else {
                    self.message = "Password field is full";
                }
            }
            TextKeyboardAction::Space => {
                if self.password.len() < WIFI_PASS_CAP {
                    self.password.push(' ');
                    self.message = "Password updated";
                }
            }
            TextKeyboardAction::Delete => self.delete_password_char(),
            TextKeyboardAction::Clear => {
                self.password.clear();
                self.message = "Password cleared";
            }
            TextKeyboardAction::ToggleCase => {
                let next = if self.keyboard_layout == text_keyboard::LAYOUT_UPPER {
                    text_keyboard::LAYOUT_LOWER
                } else {
                    text_keyboard::LAYOUT_UPPER
                };
                let (layout, index) =
                    text_keyboard::switch_layout(self.keyboard_layout, next, self.keyboard_index);
                self.keyboard_layout = layout;
                self.keyboard_index = index;
                self.message = "Keyboard case changed";
            }
            TextKeyboardAction::ToggleSymbols => {
                let next = if self.keyboard_layout == text_keyboard::LAYOUT_SYMBOLS {
                    text_keyboard::LAYOUT_LOWER
                } else {
                    text_keyboard::LAYOUT_SYMBOLS
                };
                let (layout, index) =
                    text_keyboard::switch_layout(self.keyboard_layout, next, self.keyboard_index);
                self.keyboard_layout = layout;
                self.keyboard_index = index;
                self.message = "Keyboard layout changed";
            }
            TextKeyboardAction::Done => {
                if save_selected_profile(sd, self) {
                    self.state = WifiSetupState::Saved;
                    self.message = "Saved to /RUSTMIX/SETTINGS.TXT";
                } else {
                    self.state = WifiSetupState::Error;
                    self.message = "Save failed";
                }
            }
            TextKeyboardAction::None => {}
        }
    }
}

pub async fn run_wifi_scan_mode(
    wifi: esp_hal::peripherals::WIFI<'static>,
    epd: &mut Epd,
    strip: &mut StripBuffer,
    delay: &mut Delay,
    sd: &SdStorage,
    ui_font_size_idx: u8,
    bumps: &ButtonFeedback,
) {
    let heading = fonts::heading_font(ui_font_size_idx);
    let body = fonts::chrome_font();
    let mapper = ButtonMapper::new();
    let mut session = WifiSetupSession::new(sd);

    render_message(
        epd,
        strip,
        delay,
        heading,
        body,
        "Wi-Fi Setup",
        &[
            "Scanning nearby networks...",
            "This may take a few seconds.",
        ],
        Some("Back exits"),
        bumps,
        true,
    )
    .await;

    let radio = match esp_radio::init() {
        Ok(r) => r,
        Err(e) => {
            info!("wifi-setup: radio init failed: {:?}", e);
            render_error(epd, strip, delay, heading, body, "Radio init failed", bumps).await;
            drain_until_back_or_select(&mapper).await;
            return;
        }
    };

    let (mut wifi_ctrl, _interfaces) = match esp_radio::wifi::new(&radio, wifi, Config::default()) {
        Ok(pair) => pair,
        Err(e) => {
            info!("wifi-setup: wifi::new failed: {:?}", e);
            render_error(epd, strip, delay, heading, body, "Wi-Fi init failed", bumps).await;
            drain_until_back_or_select(&mapper).await;
            return;
        }
    };

    if let Err(e) = wifi_ctrl.set_mode(WifiMode::Sta) {
        info!("wifi-setup: set_mode failed: {:?}", e);
    }

    if let Err(e) = wifi_ctrl.start_async().await {
        info!("wifi-setup: start failed: {:?}", e);
        render_error(
            epd,
            strip,
            delay,
            heading,
            body,
            "Wi-Fi start failed",
            bumps,
        )
        .await;
        drain_until_back_or_select(&mapper).await;
        return;
    }

    scan_networks(&mut wifi_ctrl, &mut session, sd).await;
    render_network_list(epd, strip, delay, heading, body, &session, bumps, false).await;

    loop {
        let hw = tasks::INPUT_EVENTS.receive().await;
        let ev = mapper.map_event(hw);

        match session.state {
            WifiSetupState::NetworkList => match ev {
                ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => break,
                ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next) => {
                    session.move_network(1);
                    render_network_list(epd, strip, delay, heading, body, &session, bumps, false)
                        .await;
                }
                ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev) => {
                    session.move_network(-1);
                    render_network_list(epd, strip, delay, heading, body, &session, bumps, false)
                        .await;
                }
                ActionEvent::Press(Action::NextJump) | ActionEvent::Repeat(Action::NextJump) => {
                    session.cycle_profile(1);
                    render_network_list(epd, strip, delay, heading, body, &session, bumps, false)
                        .await;
                }
                ActionEvent::Press(Action::PrevJump) | ActionEvent::Repeat(Action::PrevJump) => {
                    session.cycle_profile(-1);
                    render_network_list(epd, strip, delay, heading, body, &session, bumps, false)
                        .await;
                }
                ActionEvent::Press(Action::Select) => {
                    session.open_keyboard_for_selected();
                    render_password_keyboard(
                        epd, strip, delay, heading, body, &session, bumps, false,
                    )
                    .await;
                }
                ActionEvent::LongPress(Action::Select) => {
                    render_message(
                        epd,
                        strip,
                        delay,
                        heading,
                        body,
                        "Wi-Fi Setup",
                        &["Rescanning networks..."],
                        Some("Back exits"),
                        bumps,
                        false,
                    )
                    .await;
                    scan_networks(&mut wifi_ctrl, &mut session, sd).await;
                    render_network_list(epd, strip, delay, heading, body, &session, bumps, false)
                        .await;
                }
                _ => {}
            },
            WifiSetupState::PasswordKeyboard => match ev {
                ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                    session.state = WifiSetupState::NetworkList;
                    session.message = "Password not saved";
                    render_network_list(epd, strip, delay, heading, body, &session, bumps, false)
                        .await;
                }
                ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next) => {
                    session.move_key_vertical(1);
                    render_password_keyboard(
                        epd, strip, delay, heading, body, &session, bumps, false,
                    )
                    .await;
                }
                ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev) => {
                    session.move_key_vertical(-1);
                    render_password_keyboard(
                        epd, strip, delay, heading, body, &session, bumps, false,
                    )
                    .await;
                }
                ActionEvent::Press(Action::NextJump) | ActionEvent::Repeat(Action::NextJump) => {
                    session.move_key_horizontal(1);
                    render_password_keyboard(
                        epd, strip, delay, heading, body, &session, bumps, false,
                    )
                    .await;
                }
                ActionEvent::Press(Action::PrevJump) | ActionEvent::Repeat(Action::PrevJump) => {
                    session.move_key_horizontal(-1);
                    render_password_keyboard(
                        epd, strip, delay, heading, body, &session, bumps, false,
                    )
                    .await;
                }
                ActionEvent::Press(Action::Select) => {
                    session.activate_key(sd);
                    match session.state {
                        WifiSetupState::Saved | WifiSetupState::Error => {
                            render_saved_or_error(
                                epd, strip, delay, heading, body, &session, bumps,
                            )
                            .await;
                        }
                        _ => {
                            render_password_keyboard(
                                epd, strip, delay, heading, body, &session, bumps, false,
                            )
                            .await;
                        }
                    }
                }
                ActionEvent::LongPress(Action::Select) => {
                    session.delete_password_char();
                    render_password_keyboard(
                        epd, strip, delay, heading, body, &session, bumps, false,
                    )
                    .await;
                }
                _ => {}
            },
            WifiSetupState::Saved | WifiSetupState::Error => match ev {
                ActionEvent::Press(Action::Back)
                | ActionEvent::LongPress(Action::Back)
                | ActionEvent::Press(Action::Select) => break,
                _ => {}
            },
        }
    }

    let _ = wifi_ctrl.stop_async().await;
    Timer::after(Duration::from_millis(50)).await;
}

async fn scan_networks(
    wifi_ctrl: &mut esp_radio::wifi::WifiController<'_>,
    session: &mut WifiSetupSession,
    sd: &SdStorage,
) {
    session.clear_scan_results();
    session.message = "Scanning networks...";
    let _ = storage::write_in_x4(sd, WIFI_SCAN_FILE, b"");

    let cfg = ScanConfig::default()
        .with_show_hidden(false)
        .with_max(MAX_SCAN_RESULTS * 2);
    let mut aps = match wifi_ctrl.scan_with_config_async(cfg).await {
        Ok(list) => list,
        Err(e) => {
            info!("wifi-setup: scan failed: {:?}", e);
            session.message = "Scan failed; hold OK to rescan";
            return;
        }
    };

    aps.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));
    let mut out = String::new();

    for ap in aps.iter() {
        let ssid = ap.ssid.as_str();
        if ssid.is_empty() {
            continue;
        }
        let before = session.scan_count;
        session.push_scan_ssid(ssid);
        if session.scan_count != before {
            out.push_str(ssid);
            out.push('\n');
        }
        if session.scan_count >= MAX_SCAN_RESULTS {
            break;
        }
    }

    let _ = storage::write_in_x4(sd, WIFI_SCAN_FILE, out.as_bytes());
    session.message = if session.scan_count == 0 {
        "No visible SSIDs; hold OK to rescan"
    } else {
        "OK selects SSID; Left/Right changes profile"
    };
}

fn read_settings(sd: &SdStorage) -> (SystemSettings, WifiConfig) {
    let mut settings = SystemSettings::defaults();
    let mut wifi = WifiConfig::empty();
    let mut buf = [0u8; 1536];

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
        }
    }
    settings.sanitize();
    (settings, wifi)
}

fn save_selected_profile(sd: &SdStorage, session: &mut WifiSetupSession) -> bool {
    let slot = session.profile_slot.min(WIFI_PROFILE_COUNT - 1);
    session.wifi.set_profile_credentials_from_str(
        slot,
        session.selected_ssid.as_str(),
        session.password.as_str(),
    );
    session.wifi.set_default_slot(slot as u8);

    let mut out = [0u8; 1536];
    let len = write_settings_txt(&session.settings, &session.wifi, &mut out);
    storage::write_in_x4(sd, config::SETTINGS_FILE, &out[..len]).is_ok()
}

async fn drain_until_back_or_select(mapper: &ButtonMapper) {
    loop {
        let hw = tasks::INPUT_EVENTS.receive().await;
        let ev = mapper.map_event(hw);
        if matches!(
            ev,
            ActionEvent::Press(Action::Back)
                | ActionEvent::LongPress(Action::Back)
                | ActionEvent::Press(Action::Select)
        ) {
            return;
        }
    }
}

async fn render_error(
    epd: &mut Epd,
    strip: &mut StripBuffer,
    delay: &mut Delay,
    heading: &'static BitmapFont,
    body: &'static BitmapFont,
    detail: &str,
    bumps: &ButtonFeedback,
) {
    render_message(
        epd,
        strip,
        delay,
        heading,
        body,
        "Wi-Fi Setup",
        &["Wi-Fi setup failed", detail],
        Some("Press BACK/OK to return"),
        bumps,
        false,
    )
    .await;
}

async fn render_saved_or_error(
    epd: &mut Epd,
    strip: &mut StripBuffer,
    delay: &mut Delay,
    heading: &'static BitmapFont,
    body: &'static BitmapFont,
    session: &WifiSetupSession,
    bumps: &ButtonFeedback,
) {
    let profile = session.profile_name();
    let mut line1 = String::new();
    let _ = write!(line1, "Profile: {}", profile);
    let mut line2 = String::new();
    let _ = write!(line2, "SSID: {}", session.selected_ssid.as_str());
    let status = if session.state == WifiSetupState::Saved {
        "Saved. Wi-Fi Transfer will use it."
    } else {
        "Could not save SETTINGS.TXT"
    };
    render_message(
        epd,
        strip,
        delay,
        heading,
        body,
        "Wi-Fi Setup",
        &[status, line1.as_str(), line2.as_str()],
        Some("Press BACK/OK to return"),
        bumps,
        false,
    )
    .await;
}

async fn render_network_list(
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
        draw_header(s, "Wi-Fi Setup", heading, body);
        let mut profile = BitmapDynLabel::<96>::new(
            Region::new(BODY_X, CONTENT_TOP + 56, BODY_W, body.line_height),
            body,
        )
        .alignment(Alignment::CenterLeft);
        let mark = if session.profile_slot == session.wifi.default_slot() as usize {
            "*"
        } else {
            " "
        };
        let _ = write!(profile, "Profile: {} {}", mark, session.profile_name());
        profile.draw(s).unwrap();

        if session.scan_count == 0 {
            BitmapLabel::new(
                Region::new(BODY_X, CONTENT_TOP + 108, BODY_W, body.line_height),
                "No visible SSIDs found",
                body,
            )
            .alignment(Alignment::Center)
            .draw(s)
            .unwrap();
        } else {
            let start = session
                .selected_network
                .saturating_sub(LIST_VISIBLE_ROWS / 2);
            let start = start.min(session.scan_count.saturating_sub(LIST_VISIBLE_ROWS));
            let mut row = 0usize;
            while row < LIST_VISIBLE_ROWS && start + row < session.scan_count {
                let idx = start + row;
                let y = CONTENT_TOP + 94 + row as u16 * (body.line_height + 10);
                let selected = idx == session.selected_network;
                let mut label = BitmapDynLabel::<96>::new(
                    Region::new(BODY_X, y, BODY_W, body.line_height),
                    body,
                )
                .alignment(Alignment::CenterLeft)
                .inverted(selected);
                let _ = write!(
                    label,
                    "{} {}",
                    if selected { ">" } else { " " },
                    session.ssid_at(idx).unwrap_or("")
                );
                label.draw(s).unwrap();
                row += 1;
            }
        }

        BitmapLabel::new(
            Region::new(BODY_X, FOOTER_Y - 22, BODY_W, body.line_height),
            session.message,
            body,
        )
        .alignment(Alignment::Center)
        .draw(s)
        .unwrap();
        BitmapLabel::new(
            Region::new(BODY_X, FOOTER_Y, BODY_W, body.line_height),
            "Up/Down SSID · Left/Right profile · OK select · Back",
            body,
        )
        .alignment(Alignment::Center)
        .draw(s)
        .unwrap();
        bumps.draw(s);
    };
    refresh(epd, strip, delay, full_refresh, &draw).await;
}

async fn render_password_keyboard(
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
        draw_header(s, "Wi-Fi Password", heading, body);
        let mut ssid = BitmapDynLabel::<96>::new(
            Region::new(BODY_X, CONTENT_TOP + 54, BODY_W, body.line_height),
            body,
        )
        .alignment(Alignment::CenterLeft);
        let _ = write!(ssid, "SSID: {}", session.selected_ssid.as_str());
        ssid.draw(s).unwrap();

        let mut pass = BitmapDynLabel::<96>::new(
            Region::new(BODY_X, CONTENT_TOP + 82, BODY_W, body.line_height),
            body,
        )
        .alignment(Alignment::CenterLeft);
        let _ = write!(pass, "Password: {} chars", session.password.len());
        pass.draw(s).unwrap();

        text_keyboard::draw(
            s,
            Region::new(BODY_X, KEYBOARD_Y, BODY_W, KEYBOARD_H),
            session.keyboard_layout,
            session.keyboard_index,
            heading,
            body,
        );

        BitmapLabel::new(
            Region::new(BODY_X, FOOTER_Y - 22, BODY_W, body.line_height),
            session.message,
            body,
        )
        .alignment(Alignment::Center)
        .draw(s)
        .unwrap();
        BitmapLabel::new(
            Region::new(BODY_X, FOOTER_Y, BODY_W, body.line_height),
            "Move key · OK press · Hold OK delete · done saves",
            body,
        )
        .alignment(Alignment::Center)
        .draw(s)
        .unwrap();
        bumps.draw(s);
    };
    refresh(epd, strip, delay, full_refresh, &draw).await;
}

async fn render_message(
    epd: &mut Epd,
    strip: &mut StripBuffer,
    delay: &mut Delay,
    heading: &'static BitmapFont,
    body: &'static BitmapFont,
    title: &str,
    lines: &[&str],
    footer: Option<&str>,
    bumps: &ButtonFeedback,
    full_refresh: bool,
) {
    let draw = |s: &mut StripBuffer| {
        draw_header(s, title, heading, body);
        let total_h = if lines.is_empty() {
            0
        } else {
            (lines.len() as u16 - 1) * (body.line_height + BODY_LINE_GAP) + body.line_height
        };
        let y0 = CONTENT_TOP + 120u16.saturating_sub(total_h / 2);
        for (i, line) in lines.iter().enumerate() {
            BitmapLabel::new(
                Region::new(
                    BODY_X,
                    y0 + i as u16 * (body.line_height + BODY_LINE_GAP),
                    BODY_W,
                    body.line_height,
                ),
                line,
                body,
            )
            .alignment(Alignment::Center)
            .draw(s)
            .unwrap();
        }
        if let Some(text) = footer {
            BitmapLabel::new(
                Region::new(BODY_X, FOOTER_Y, BODY_W, body.line_height),
                text,
                body,
            )
            .alignment(Alignment::Center)
            .draw(s)
            .unwrap();
        }
        bumps.draw(s);
    };
    refresh(epd, strip, delay, full_refresh, &draw).await;
}

fn draw_header(
    strip: &mut StripBuffer,
    title: &str,
    heading: &'static BitmapFont,
    body: &'static BitmapFont,
) {
    BitmapLabel::new(
        Region::new(HEADING_X, CONTENT_TOP + 12, HEADING_W, heading.line_height),
        title,
        heading,
    )
    .alignment(Alignment::CenterLeft)
    .draw(strip)
    .unwrap();
    BitmapLabel::new(
        Region::new(
            HEADING_X,
            CONTENT_TOP + 12 + heading.line_height + 8,
            HEADING_W,
            body.line_height,
        ),
        "Biscuit-style scan + keyboard setup",
        body,
    )
    .alignment(Alignment::CenterLeft)
    .draw(strip)
    .unwrap();
}

async fn refresh<F>(
    epd: &mut Epd,
    strip: &mut StripBuffer,
    delay: &mut Delay,
    full_refresh: bool,
    draw: &F,
) where
    F: Fn(&mut StripBuffer),
{
    if full_refresh {
        epd.full_refresh_async(strip, delay, draw).await;
    } else {
        epd.partial_refresh_async(strip, delay, 0, 0, SCREEN_W, SCREEN_H, draw)
            .await;
    }
}
