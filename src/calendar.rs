//! RTC-localized Calendar state plus bounded SD event loading.
//!
//! Rustmix-Wave keeps the native month grid while reusing the Rustmix X4
//! removable-storage contract:
//! `/sdcard/RUSTMIX/APPS/CALENDAR/EVENTS.TXT` for personal entries and
//! `/sdcard/RUSTMIX/APPS/CALENDAR/US2026.TXT` for U.S. calendar entries.
//! Hindu calendar data is deliberately excluded from this native milestone.

use std::{
    fs::{self, File},
    io::{ErrorKind, Write},
    ops::Range,
    path::Path,
};

use anyhow::{bail, Context, Result};

use crate::{buttons::ButtonEvent, keyboard_navigation::KeyboardGridNavigation, rtc::RtcDateTime};

/// First year supported by the RTC-backed Calendar screen.
pub const CALENDAR_MIN_YEAR: u16 = 2000;
/// Last year supported by the RTC-backed Calendar screen.
pub const CALENDAR_MAX_YEAR: u16 = 2099;
/// Native Calendar root reused from the Rustmix X4 SD application pack.
pub const CALENDAR_ROOT: &str = "/sdcard/RUSTMIX/APPS/CALENDAR";
/// Personal calendar event file reused from the X4 pack.
pub const CALENDAR_EVENTS_FILE: &str = "EVENTS.TXT";
/// U.S. calendar event file reused from the X4 pack.
pub const CALENDAR_US_EVENTS_FILE: &str = "US2026.TXT";
/// Recovery-safe temporary file used while replacing personal events.
pub const CALENDAR_EVENTS_TEMP_FILE: &str = "EVENTS.TMP";
/// Last known good personal-event file retained for rollback and recovery.
pub const CALENDAR_EVENTS_BACKUP_FILE: &str = "EVENTS.BAK";
/// Keep each text file bounded for SD reads on the main loop.
pub const CALENDAR_FILE_MAX_BYTES: usize = 16 * 1024;
/// Bound total retained events so malformed cards cannot grow runtime state.
pub const CALENDAR_EVENT_LIMIT: usize = 192;
/// Agenda rows visible on one e-paper page.
pub const CALENDAR_AGENDA_VISIBLE_ROWS: usize = 6;
/// Personal-event title length accepted by the rotary editor.
pub const CALENDAR_PERSONAL_TITLE_MAX_CHARS: usize = 32;
/// Optional personal-event detail length accepted by the rotary editor.
pub const CALENDAR_PERSONAL_DETAIL_MAX_CHARS: usize = 80;
/// Shared-grid Calendar editor keyboard rows.
pub const CALENDAR_EDITOR_KEY_ROWS: [[&str; 7]; 6] = [
    ["A", "B", "C", "D", "E", "F", "G"],
    ["H", "I", "J", "K", "L", "M", "N"],
    ["O", "P", "Q", "R", "S", "T", "U"],
    ["V", "W", "X", "Y", "Z", "0", "1"],
    ["2", "3", "4", "5", "6", "7", "8"],
    ["9", ".", "SP", "DEL", "FIELD", "SAVE", "CANCEL"],
];
const CALENDAR_EDITOR_KEY_COLUMNS: usize = 7;
const CALENDAR_EDITOR_KEY_COUNT: usize = 42;

/// One valid Gregorian date inside the RTC-backed supported range.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CalendarDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl Default for CalendarDate {
    fn default() -> Self {
        Self {
            year: CALENDAR_MIN_YEAR,
            month: 1,
            day: 1,
        }
    }
}

impl CalendarDate {
    /// Build a bounded date when all fields are valid.
    #[must_use]
    pub const fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        if year < CALENDAR_MIN_YEAR
            || year > CALENDAR_MAX_YEAR
            || month == 0
            || month > 12
            || day == 0
            || day > days_in_month(year, month)
        {
            return None;
        }
        Some(Self { year, month, day })
    }

    /// Use a localized RTC snapshot as the Calendar cursor when possible.
    #[must_use]
    pub fn from_rtc(value: RtcDateTime) -> Self {
        Self::new(value.year, value.month, value.day).unwrap_or_default()
    }

    /// Sunday-zero weekday index used by the month grid.
    #[must_use]
    pub fn weekday(self) -> u8 {
        weekday(self.year, self.month, self.day)
    }

    /// Move by signed days while clamping at the RTC-supported boundaries.
    #[must_use]
    pub fn shifted_days(mut self, delta: i32) -> Self {
        let mut remaining = delta;
        while remaining > 0 {
            if self == Self::maximum() {
                break;
            }
            self = self.next_day();
            remaining -= 1;
        }
        while remaining < 0 {
            if self == Self::minimum() {
                break;
            }
            self = self.previous_day();
            remaining += 1;
        }
        self
    }

    /// Move by signed months while retaining the selected day when possible.
    #[must_use]
    pub fn shifted_months(self, delta: i32) -> Self {
        let current = i32::from(self.year) * 12 + i32::from(self.month) - 1;
        let minimum = i32::from(CALENDAR_MIN_YEAR) * 12;
        let maximum = i32::from(CALENDAR_MAX_YEAR) * 12 + 11;
        let shifted = current.saturating_add(delta).clamp(minimum, maximum);
        let year = (shifted / 12) as u16;
        let month = (shifted % 12 + 1) as u8;
        let day = self.day.min(days_in_month(year, month));
        Self { year, month, day }
    }

    #[must_use]
    pub const fn minimum() -> Self {
        Self {
            year: CALENDAR_MIN_YEAR,
            month: 1,
            day: 1,
        }
    }

    #[must_use]
    pub const fn maximum() -> Self {
        Self {
            year: CALENDAR_MAX_YEAR,
            month: 12,
            day: 31,
        }
    }

    fn next_day(self) -> Self {
        let days = days_in_month(self.year, self.month);
        if self.day < days {
            return Self {
                day: self.day + 1,
                ..self
            };
        }
        if self.month < 12 {
            return Self {
                year: self.year,
                month: self.month + 1,
                day: 1,
            };
        }
        Self {
            year: self.year + 1,
            month: 1,
            day: 1,
        }
    }

    fn previous_day(self) -> Self {
        if self.day > 1 {
            return Self {
                day: self.day - 1,
                ..self
            };
        }
        if self.month > 1 {
            let month = self.month - 1;
            return Self {
                year: self.year,
                month,
                day: days_in_month(self.year, month),
            };
        }
        Self {
            year: self.year - 1,
            month: 12,
            day: 31,
        }
    }
}

/// Navigation axis selected on the native monthly Calendar page.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CalendarNavigationMode {
    #[default]
    Day,
    Month,
}

impl CalendarNavigationMode {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Day => "DAY MODE",
            Self::Month => "MONTH MODE",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Day => Self::Month,
            Self::Month => Self::Day,
        }
    }
}

/// Native Calendar event origin. U.S. entries remain read-only reference data.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CalendarEventKind {
    Personal,
    UsHoliday,
}

impl CalendarEventKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Personal => "PERSONAL",
            Self::UsHoliday => "US",
        }
    }

    #[must_use]
    pub const fn source_file(self) -> &'static str {
        match self {
            Self::Personal => CALENDAR_EVENTS_FILE,
            Self::UsHoliday => CALENDAR_US_EVENTS_FILE,
        }
    }
}

/// One bounded calendar event parsed from the X4-compatible SD text files.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalendarEvent {
    pub date: CalendarDate,
    pub kind: CalendarEventKind,
    pub title: String,
    pub detail: String,
    /// Stable zero-based row identity inside the source file before sorting.
    pub source_row: usize,
}

/// Personal-event editor operation retained while the user types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalendarEditorMode {
    Create,
    Edit { source_row: usize },
}

impl CalendarEditorMode {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Create => "CREATE PERSONAL",
            Self::Edit { .. } => "EDIT PERSONAL",
        }
    }
}

/// Active text field in the Calendar personal-event editor.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CalendarEditorField {
    #[default]
    Title,
    Detail,
}

impl CalendarEditorField {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Title => "TITLE",
            Self::Detail => "DETAIL",
        }
    }

    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Title => Self::Detail,
            Self::Detail => Self::Title,
        }
    }
}

/// Deferred SD-card mutation request consumed by the native main loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CalendarUiRequest {
    CreatePersonal {
        date: CalendarDate,
        title: String,
        detail: String,
    },
    UpdatePersonal {
        source_row: usize,
        title: String,
        detail: String,
    },
    DeletePersonal {
        source_row: usize,
    },
}

/// Reusable BOOT-H/V keyboard state for personal Calendar rows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalendarEventEditorState {
    pub mode: CalendarEditorMode,
    pub date: CalendarDate,
    pub title: String,
    pub detail: String,
    pub active_field: CalendarEditorField,
    pub keyboard_navigation: KeyboardGridNavigation,
    pub message: String,
}

impl CalendarEventEditorState {
    #[must_use]
    pub fn create(date: CalendarDate) -> Self {
        Self {
            mode: CalendarEditorMode::Create,
            date,
            title: String::new(),
            detail: String::new(),
            active_field: CalendarEditorField::Title,
            keyboard_navigation: KeyboardGridNavigation::new(
                CALENDAR_EDITOR_KEY_COUNT,
                CALENDAR_EDITOR_KEY_COLUMNS,
            ),
            message: "Enter title. FIELD switches title/detail.".into(),
        }
    }

    #[must_use]
    pub fn edit(event: &CalendarEvent) -> Self {
        Self {
            mode: CalendarEditorMode::Edit {
                source_row: event.source_row,
            },
            date: event.date,
            title: event.title.clone(),
            detail: event.detail.clone(),
            active_field: CalendarEditorField::Title,
            keyboard_navigation: KeyboardGridNavigation::new(
                CALENDAR_EDITOR_KEY_COUNT,
                CALENDAR_EDITOR_KEY_COLUMNS,
            ),
            message: "Edit title or detail, then select SAVE.".into(),
        }
    }

    #[must_use]
    pub fn selected_key_label(&self) -> &'static str {
        flat_calendar_editor_key(self.keyboard_navigation.selected())
    }

    #[must_use]
    pub const fn selected_key_index(&self) -> usize {
        self.keyboard_navigation.selected()
    }

    #[must_use]
    pub const fn navigation_mode_label(&self) -> &'static str {
        self.keyboard_navigation.status_label()
    }

    pub fn toggle_navigation_axis(&mut self) {
        self.keyboard_navigation.toggle_axis();
        self.message = format!(
            "Keyboard {}. Rotary moves within active axis.",
            self.navigation_mode_label()
        );
    }

    fn active_text_mut(&mut self) -> &mut String {
        match self.active_field {
            CalendarEditorField::Title => &mut self.title,
            CalendarEditorField::Detail => &mut self.detail,
        }
    }

    fn active_limit(&self) -> usize {
        match self.active_field {
            CalendarEditorField::Title => CALENDAR_PERSONAL_TITLE_MAX_CHARS,
            CalendarEditorField::Detail => CALENDAR_PERSONAL_DETAIL_MAX_CHARS,
        }
    }

    pub fn apply_button(&mut self, event: ButtonEvent) -> CalendarEditorOutcome {
        match event {
            ButtonEvent::Up => self.keyboard_navigation.move_previous(),
            ButtonEvent::Down => self.keyboard_navigation.move_next(),
            ButtonEvent::Select => return self.apply_selected_key(),
        }
        CalendarEditorOutcome::None
    }

    fn apply_selected_key(&mut self) -> CalendarEditorOutcome {
        match self.selected_key_label() {
            "SP" => self.push_char(' '),
            "DEL" => {
                self.active_text_mut().pop();
            }
            "FIELD" => {
                self.active_field = self.active_field.toggled();
                self.message = format!("Editing {}.", self.active_field.label());
            }
            "SAVE" => {
                let title = sanitize_personal_field(&self.title, CALENDAR_PERSONAL_TITLE_MAX_CHARS);
                let detail =
                    sanitize_personal_field(&self.detail, CALENDAR_PERSONAL_DETAIL_MAX_CHARS);
                if title.is_empty() {
                    self.message = "Title is required before SAVE.".into();
                } else {
                    return CalendarEditorOutcome::Save(match self.mode {
                        CalendarEditorMode::Create => CalendarUiRequest::CreatePersonal {
                            date: self.date,
                            title,
                            detail,
                        },
                        CalendarEditorMode::Edit { source_row } => {
                            CalendarUiRequest::UpdatePersonal {
                                source_row,
                                title,
                                detail,
                            }
                        }
                    });
                }
            }
            "CANCEL" => return CalendarEditorOutcome::Cancel,
            label => {
                if let Some(character) = label.chars().next() {
                    self.push_char(character);
                }
            }
        }
        CalendarEditorOutcome::None
    }

    fn push_char(&mut self, character: char) {
        let limit = self.active_limit();
        if self.active_text_mut().chars().count() < limit {
            self.active_text_mut().push(character);
        } else {
            self.message = format!("{} limit reached.", self.active_field.label());
        }
    }
}

/// State-machine outcome returned by one editor SELECT event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CalendarEditorOutcome {
    None,
    Save(CalendarUiRequest),
    Cancel,
}

/// Load snapshot retained for readable missing-file-safe UI diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalendarCatalogSnapshot {
    pub events: Vec<CalendarEvent>,
    pub personal_loaded: bool,
    pub us_loaded: bool,
    pub warning: Option<String>,
}

impl Default for CalendarCatalogSnapshot {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            personal_loaded: false,
            us_loaded: false,
            warning: Some("Calendar files not loaded".into()),
        }
    }
}

impl CalendarCatalogSnapshot {
    #[must_use]
    pub fn status_label(&self) -> String {
        if let Some(warning) = self.warning.as_deref() {
            return compact_text(warning, 36);
        }
        format!(
            "{} EVENTS  |  PERSONAL {}  US {}",
            self.events.len(),
            if self.personal_loaded { "ON" } else { "MISS" },
            if self.us_loaded { "ON" } else { "MISS" }
        )
    }
}

/// UI-owned cursor plus bounded SD-backed event snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalendarUiState {
    pub cursor: CalendarDate,
    pub mode: CalendarNavigationMode,
    pub catalog: CalendarCatalogSnapshot,
    pub agenda_selected: usize,
    pub details_action_selected: usize,
    pub delete_confirmation_selected: usize,
    pub editor: Option<CalendarEventEditorState>,
    pub notice: String,
    request: Option<CalendarUiRequest>,
    initialized: bool,
}

impl Default for CalendarUiState {
    fn default() -> Self {
        Self {
            cursor: CalendarDate::default(),
            mode: CalendarNavigationMode::default(),
            catalog: CalendarCatalogSnapshot::default(),
            agenda_selected: 0,
            details_action_selected: 0,
            delete_confirmation_selected: 0,
            editor: None,
            notice: "BOOT short opens agenda. BOOT short again adds personal event.".into(),
            request: None,
            initialized: false,
        }
    }
}

impl CalendarUiState {
    /// Initialize the cursor once from a localized RTC snapshot. Returning to
    /// Calendar later keeps the user's previous month and selected day.
    pub fn initialize_if_needed(&mut self, local: Option<RtcDateTime>) {
        if self.initialized {
            return;
        }
        if let Some(value) = local {
            self.cursor = CalendarDate::from_rtc(value);
        }
        self.initialized = true;
    }

    /// Rebuild the event snapshot from the accepted X4-compatible SD root.
    pub fn refresh_events(&mut self) {
        self.refresh_events_from_root(Path::new(CALENDAR_ROOT));
    }

    /// Testable root override used by host-side parsing regressions.
    pub fn refresh_events_from_root(&mut self, root: &Path) {
        self.catalog = load_calendar_catalog(root);
        self.agenda_selected = 0;
        self.details_action_selected = 0;
    }

    /// Move backward according to the active month-grid axis.
    pub fn move_previous(&mut self) {
        self.cursor = match self.mode {
            CalendarNavigationMode::Day => self.cursor.shifted_days(-1),
            CalendarNavigationMode::Month => self.cursor.shifted_months(-1),
        };
        self.agenda_selected = 0;
    }

    /// Move forward according to the active month-grid axis.
    pub fn move_next(&mut self) {
        self.cursor = match self.mode {
            CalendarNavigationMode::Day => self.cursor.shifted_days(1),
            CalendarNavigationMode::Month => self.cursor.shifted_months(1),
        };
        self.agenda_selected = 0;
    }

    /// Toggle between selected-day and selected-month month-grid navigation.
    pub fn toggle_mode(&mut self) {
        self.mode = self.mode.next();
    }

    /// Reset daily-agenda cursor before entering the selected day.
    pub fn prepare_agenda(&mut self) {
        self.agenda_selected = 0;
    }

    #[must_use]
    pub fn events_for_date(&self, date: CalendarDate) -> Vec<&CalendarEvent> {
        self.catalog
            .events
            .iter()
            .filter(|event| event.date == date)
            .collect()
    }

    #[must_use]
    pub fn selected_day_events(&self) -> Vec<&CalendarEvent> {
        self.events_for_date(self.cursor)
    }

    #[must_use]
    pub fn selected_day_event_count(&self) -> usize {
        self.catalog
            .events
            .iter()
            .filter(|event| event.date == self.cursor)
            .count()
    }

    #[must_use]
    pub fn event_count_for_date(&self, date: CalendarDate) -> usize {
        self.catalog
            .events
            .iter()
            .filter(|event| event.date == date)
            .count()
    }

    #[must_use]
    pub fn selected_day_summary(&self) -> String {
        let events = self.selected_day_events();
        if events.is_empty() {
            "No events".into()
        } else if events.len() == 1 {
            format!("1 event  |  {}", compact_text(&events[0].title, 28))
        } else {
            format!(
                "{} events  |  {}",
                events.len(),
                compact_text(&events[0].title, 25)
            )
        }
    }

    #[must_use]
    pub fn agenda_visible_range(&self) -> Range<usize> {
        let len = self.selected_day_event_count();
        if len <= CALENDAR_AGENDA_VISIBLE_ROWS {
            return 0..len;
        }
        let max_start = len - CALENDAR_AGENDA_VISIBLE_ROWS;
        let start = self
            .agenda_selected
            .saturating_sub(CALENDAR_AGENDA_VISIBLE_ROWS - 1)
            .min(max_start);
        start..(start + CALENDAR_AGENDA_VISIBLE_ROWS)
    }

    pub fn agenda_previous(&mut self) {
        let count = self.selected_day_event_count();
        if count == 0 {
            self.agenda_selected = 0;
        } else {
            self.agenda_selected = self.agenda_selected.checked_sub(1).unwrap_or(count - 1);
        }
    }

    pub fn agenda_next(&mut self) {
        let count = self.selected_day_event_count();
        if count == 0 {
            self.agenda_selected = 0;
        } else {
            self.agenda_selected = (self.agenda_selected + 1) % count;
        }
    }

    #[must_use]
    pub fn selected_agenda_event(&self) -> Option<&CalendarEvent> {
        self.selected_day_events()
            .get(self.agenda_selected)
            .copied()
    }

    #[must_use]
    pub fn selected_event_is_personal(&self) -> bool {
        self.selected_agenda_event()
            .is_some_and(|event| event.kind == CalendarEventKind::Personal)
    }

    pub fn begin_create_personal(&mut self) {
        self.editor = Some(CalendarEventEditorState::create(self.cursor));
        self.notice = "Creating personal EVENTS.TXT row.".into();
    }

    pub fn begin_edit_selected_personal(&mut self) -> bool {
        let Some(event) = self.selected_agenda_event().cloned() else {
            return false;
        };
        if event.kind != CalendarEventKind::Personal {
            self.notice = "U.S. holiday rows remain read-only.".into();
            return false;
        }
        self.editor = Some(CalendarEventEditorState::edit(&event));
        true
    }

    pub fn clear_editor(&mut self) {
        self.editor = None;
    }

    pub fn toggle_editor_navigation_axis(&mut self) -> bool {
        if let Some(editor) = self.editor.as_mut() {
            editor.toggle_navigation_axis();
            true
        } else {
            false
        }
    }

    pub fn apply_editor_button(&mut self, event: ButtonEvent) -> CalendarEditorOutcome {
        self.editor
            .as_mut()
            .map_or(CalendarEditorOutcome::None, |editor| {
                editor.apply_button(event)
            })
    }

    pub fn select_previous_details_action(&mut self) {
        if self.selected_event_is_personal() {
            self.details_action_selected = self.details_action_selected.checked_sub(1).unwrap_or(2);
        }
    }

    pub fn select_next_details_action(&mut self) {
        if self.selected_event_is_personal() {
            self.details_action_selected = (self.details_action_selected + 1) % 3;
        }
    }

    #[must_use]
    pub fn selected_details_action_label(&self) -> &'static str {
        match self.details_action_selected {
            0 => "Edit personal event",
            1 => "Delete personal event",
            _ => "Return to agenda",
        }
    }

    pub fn prepare_delete_confirmation(&mut self) -> bool {
        if self.selected_event_is_personal() {
            self.delete_confirmation_selected = 0;
            true
        } else {
            self.notice = "U.S. holiday rows remain read-only.".into();
            false
        }
    }

    pub fn select_previous_delete_confirmation(&mut self) {
        self.delete_confirmation_selected = self
            .delete_confirmation_selected
            .checked_sub(1)
            .unwrap_or(1);
    }

    pub fn select_next_delete_confirmation(&mut self) {
        self.delete_confirmation_selected = (self.delete_confirmation_selected + 1) % 2;
    }

    pub fn request_delete_selected_personal(&mut self) -> bool {
        let Some(event) = self.selected_agenda_event() else {
            return false;
        };
        if event.kind != CalendarEventKind::Personal {
            self.notice = "U.S. holiday rows remain read-only.".into();
            return false;
        }
        self.request = Some(CalendarUiRequest::DeletePersonal {
            source_row: event.source_row,
        });
        true
    }

    pub fn queue_request(&mut self, request: CalendarUiRequest) {
        self.request = Some(request);
    }

    #[must_use]
    pub fn take_request(&mut self) -> Option<CalendarUiRequest> {
        self.request.take()
    }

    pub fn mark_persistence_completed(&mut self, message: impl Into<String>) {
        self.editor = None;
        self.details_action_selected = 0;
        self.delete_confirmation_selected = 0;
        self.notice = message.into();
    }

    pub fn fail(&mut self, message: impl Into<String>) {
        self.notice = compact_text(&message.into(), 64);
    }
}

/// Load both accepted calendar sources independently so one missing file never
/// blocks rendering of the other. `HINDU26.TXT` is intentionally not opened.
#[must_use]
pub fn load_calendar_catalog(root: &Path) -> CalendarCatalogSnapshot {
    let mut snapshot = CalendarCatalogSnapshot {
        events: Vec::new(),
        personal_loaded: false,
        us_loaded: false,
        warning: None,
    };
    let mut warnings = Vec::new();

    match load_personal_events_with_backup(root) {
        Ok(Some((mut events, recovered))) => {
            snapshot.personal_loaded = true;
            snapshot.events.append(&mut events);
            if recovered {
                warnings.push("EVENTS.TXT recovered from EVENTS.BAK".into());
            }
        }
        Ok(None) => warnings.push("EVENTS.TXT missing".into()),
        Err(error) => warnings.push(format!("EVENTS.TXT: {error}")),
    }

    match read_bounded_optional(&root.join(CALENDAR_US_EVENTS_FILE), CALENDAR_FILE_MAX_BYTES) {
        Ok(Some(text)) => match parse_us_events(&text) {
            Ok(mut events) => {
                snapshot.us_loaded = true;
                snapshot.events.append(&mut events);
            }
            Err(error) => warnings.push(format!("US2026.TXT: {error}")),
        },
        Ok(None) => warnings.push("US2026.TXT missing".into()),
        Err(error) => warnings.push(format!("US2026.TXT: {error}")),
    }

    snapshot.events.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then(left.kind.cmp(&right.kind))
            .then(left.title.cmp(&right.title))
    });
    snapshot.events.truncate(CALENDAR_EVENT_LIMIT);
    if !warnings.is_empty() {
        snapshot.warning = Some(warnings.join("; "));
    }
    snapshot
}

pub fn parse_personal_events(text: &str) -> Result<Vec<CalendarEvent>> {
    parse_calendar_lines(text, CalendarEventKind::Personal)
}

pub fn parse_us_events(text: &str) -> Result<Vec<CalendarEvent>> {
    parse_calendar_lines(text, CalendarEventKind::UsHoliday)
}

fn parse_calendar_lines(text: &str, kind: CalendarEventKind) -> Result<Vec<CalendarEvent>> {
    let mut events = Vec::new();
    for (line_number, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<_> = line.split('|').map(str::trim).collect();
        let (date_text, title, detail) = match kind {
            CalendarEventKind::Personal if (2..=3).contains(&parts.len()) => {
                (parts[0], parts[1], parts.get(2).copied().unwrap_or(""))
            }
            CalendarEventKind::UsHoliday if parts.len() == 4 => (parts[0], parts[2], parts[3]),
            _ => bail!("bad calendar row {}", line_number + 1),
        };
        let date = parse_calendar_date(date_text)
            .with_context(|| format!("bad date on row {}", line_number + 1))?;
        let title = compact_text(title, 64);
        let detail = compact_text(detail, 120);
        if title.is_empty() {
            bail!("empty title on row {}", line_number + 1);
        }
        events.push(CalendarEvent {
            date,
            kind,
            title,
            detail,
            source_row: events.len(),
        });
        if events.len() >= CALENDAR_EVENT_LIMIT {
            break;
        }
    }
    Ok(events)
}

fn load_personal_events_with_backup(root: &Path) -> Result<Option<(Vec<CalendarEvent>, bool)>> {
    let primary = root.join(CALENDAR_EVENTS_FILE);
    let backup = root.join(CALENDAR_EVENTS_BACKUP_FILE);
    match read_bounded_optional(&primary, CALENDAR_FILE_MAX_BYTES) {
        Ok(Some(text)) => match parse_personal_events(&text) {
            Ok(events) => Ok(Some((events, false))),
            Err(primary_error) => match read_bounded_optional(&backup, CALENDAR_FILE_MAX_BYTES) {
                Ok(Some(backup_text)) => parse_personal_events(&backup_text)
                    .map(|events| Some((events, true)))
                    .with_context(|| format!("primary invalid ({primary_error}); backup invalid")),
                Ok(None) => {
                    Err(primary_error).context("primary EVENTS.TXT invalid; backup missing")
                }
                Err(backup_error) => Err(backup_error).context(format!(
                    "primary EVENTS.TXT invalid ({primary_error}); backup unavailable"
                )),
            },
        },
        Ok(None) => match read_bounded_optional(&backup, CALENDAR_FILE_MAX_BYTES) {
            Ok(Some(text)) => parse_personal_events(&text)
                .map(|events| Some((events, true)))
                .context("EVENTS.TXT missing and EVENTS.BAK invalid"),
            Ok(None) => Ok(None),
            Err(error) => Err(error).context("EVENTS.TXT missing and EVENTS.BAK unavailable"),
        },
        Err(primary_error) => match read_bounded_optional(&backup, CALENDAR_FILE_MAX_BYTES) {
            Ok(Some(text)) => parse_personal_events(&text)
                .map(|events| Some((events, true)))
                .context("EVENTS.TXT unreadable and EVENTS.BAK invalid"),
            Ok(None) => Err(primary_error).context("EVENTS.TXT unreadable and EVENTS.BAK missing"),
            Err(backup_error) => Err(backup_error).context(format!(
                "EVENTS.TXT unreadable ({primary_error}); EVENTS.BAK unavailable"
            )),
        },
    }
}

fn load_personal_events_for_write(root: &Path) -> Result<Vec<CalendarEvent>> {
    Ok(load_personal_events_with_backup(root)?
        .map(|(events, _)| events)
        .unwrap_or_default())
}

pub fn create_personal_event(
    root: &Path,
    date: CalendarDate,
    title: &str,
    detail: &str,
) -> Result<()> {
    let mut events = load_personal_events_for_write(root)?;
    if events.len() >= CALENDAR_EVENT_LIMIT {
        bail!("personal event limit reached");
    }
    let title = sanitize_personal_field(title, CALENDAR_PERSONAL_TITLE_MAX_CHARS);
    if title.is_empty() {
        bail!("personal event title is required");
    }
    events.push(CalendarEvent {
        date,
        kind: CalendarEventKind::Personal,
        title,
        detail: sanitize_personal_field(detail, CALENDAR_PERSONAL_DETAIL_MAX_CHARS),
        source_row: events.len(),
    });
    save_personal_events(root, &events)
}

pub fn update_personal_event(
    root: &Path,
    source_row: usize,
    title: &str,
    detail: &str,
) -> Result<()> {
    let mut events = load_personal_events_for_write(root)?;
    let event = events
        .iter_mut()
        .find(|event| event.source_row == source_row)
        .context("personal event no longer exists")?;
    let title = sanitize_personal_field(title, CALENDAR_PERSONAL_TITLE_MAX_CHARS);
    if title.is_empty() {
        bail!("personal event title is required");
    }
    event.title = title;
    event.detail = sanitize_personal_field(detail, CALENDAR_PERSONAL_DETAIL_MAX_CHARS);
    save_personal_events(root, &events)
}

pub fn delete_personal_event(root: &Path, source_row: usize) -> Result<()> {
    let mut events = load_personal_events_for_write(root)?;
    let original_len = events.len();
    events.retain(|event| event.source_row != source_row);
    if events.len() == original_len {
        bail!("personal event no longer exists");
    }
    save_personal_events(root, &events)
}

pub fn save_personal_events(root: &Path, events: &[CalendarEvent]) -> Result<()> {
    fs::create_dir_all(root).with_context(|| format!("create {}", root.display()))?;
    let text = serialize_personal_events(events)?;
    if text.len() > CALENDAR_FILE_MAX_BYTES {
        bail!("EVENTS.TXT exceeds {CALENDAR_FILE_MAX_BYTES} bytes");
    }
    let primary = root.join(CALENDAR_EVENTS_FILE);
    let temporary = root.join(CALENDAR_EVENTS_TEMP_FILE);
    let backup = root.join(CALENDAR_EVENTS_BACKUP_FILE);
    let _ = fs::remove_file(&temporary);
    {
        let mut file =
            File::create(&temporary).with_context(|| format!("create {}", temporary.display()))?;
        file.write_all(text.as_bytes())
            .with_context(|| format!("write {}", temporary.display()))?;
        file.sync_all()
            .with_context(|| format!("sync {}", temporary.display()))?;
    }
    let _ = fs::remove_file(&backup);
    let moved_primary = if primary.exists() {
        fs::rename(&primary, &backup).with_context(|| format!("backup {}", primary.display()))?;
        true
    } else {
        false
    };
    if let Err(error) = fs::rename(&temporary, &primary) {
        if moved_primary {
            let _ = fs::rename(&backup, &primary);
        }
        let _ = fs::remove_file(&temporary);
        return Err(error).with_context(|| format!("replace {}", primary.display()));
    }
    Ok(())
}

fn serialize_personal_events(events: &[CalendarEvent]) -> Result<String> {
    let mut output = String::new();
    for event in events
        .iter()
        .filter(|event| event.kind == CalendarEventKind::Personal)
    {
        let title = sanitize_personal_field(&event.title, CALENDAR_PERSONAL_TITLE_MAX_CHARS);
        if title.is_empty() {
            bail!("personal event title is required");
        }
        let detail = sanitize_personal_field(&event.detail, CALENDAR_PERSONAL_DETAIL_MAX_CHARS);
        output.push_str(&format!(
            "{:04}-{:02}-{:02}|{}|{}\n",
            event.date.year, event.date.month, event.date.day, title, detail
        ));
    }
    Ok(output)
}

#[must_use]
pub fn sanitize_personal_field(text: &str, max_chars: usize) -> String {
    let safe = text.replace('|', " ").replace('\n', " ").replace('\r', " ");
    compact_text(&safe, max_chars)
}

#[must_use]
fn flat_calendar_editor_key(index: usize) -> &'static str {
    CALENDAR_EDITOR_KEY_ROWS
        .iter()
        .flatten()
        .nth(index)
        .copied()
        .unwrap_or("A")
}

fn read_bounded_optional(path: &Path, max_bytes: usize) -> Result<Option<String>> {
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.len() > max_bytes as u64 {
                bail!("file exceeds {max_bytes} bytes");
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("stat {}", path.display())),
    }
    fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))
        .map(Some)
}

fn parse_calendar_date(text: &str) -> Result<CalendarDate> {
    let mut fields = text.split('-');
    let year = fields
        .next()
        .context("missing year")?
        .parse::<u16>()
        .context("bad year")?;
    let month = fields
        .next()
        .context("missing month")?
        .parse::<u8>()
        .context("bad month")?;
    let day = fields
        .next()
        .context("missing day")?
        .parse::<u8>()
        .context("bad day")?;
    if fields.next().is_some() {
        bail!("extra date field");
    }
    CalendarDate::new(year, month, day).context("date outside supported range")
}

#[must_use]
pub fn compact_text(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut compact = normalized
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    compact.push_str("...");
    compact
}

/// Gregorian leap-year policy used by Calendar.
#[must_use]
pub const fn is_leap_year(year: u16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Number of days in one month. Invalid months safely return zero.
#[must_use]
pub const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Sunday-zero weekday for one valid Gregorian date.
#[must_use]
pub fn weekday(year: u16, month: u8, day: u8) -> u8 {
    const OFFSETS: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut adjusted_year = i32::from(year);
    if month < 3 {
        adjusted_year -= 1;
    }
    let index = usize::from(month.saturating_sub(1).min(11));
    (adjusted_year + adjusted_year / 4 - adjusted_year / 100
        + adjusted_year / 400
        + OFFSETS[index]
        + i32::from(day))
    .rem_euclid(7) as u8
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{
        create_personal_event, days_in_month, delete_personal_event, is_leap_year,
        load_calendar_catalog, parse_personal_events, parse_us_events, update_personal_event,
        weekday, CalendarDate, CalendarEditorOutcome, CalendarEventEditorState, CalendarEventKind,
        CalendarNavigationMode, CalendarUiRequest, CalendarUiState, CALENDAR_AGENDA_VISIBLE_ROWS,
        CALENDAR_EDITOR_KEY_COLUMNS, CALENDAR_EDITOR_KEY_COUNT,
    };
    use crate::{
        buttons::ButtonEvent, keyboard_navigation::KeyboardGridNavigation, rtc::RtcDateTime,
    };

    fn temp_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("rustmix-calendar-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn handles_rtc_range_leap_years() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2025));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2025, 2), 28);
    }

    #[test]
    fn weekday_matches_known_thursday() {
        assert_eq!(weekday(2026, 6, 4), 4);
    }

    #[test]
    fn day_navigation_crosses_month_boundary() {
        assert_eq!(
            CalendarDate::new(2026, 6, 30).unwrap().shifted_days(1),
            CalendarDate::new(2026, 7, 1).unwrap()
        );
        assert_eq!(
            CalendarDate::new(2024, 3, 1).unwrap().shifted_days(-1),
            CalendarDate::new(2024, 2, 29).unwrap()
        );
    }

    #[test]
    fn month_navigation_clamps_selected_day() {
        assert_eq!(
            CalendarDate::new(2026, 1, 31).unwrap().shifted_months(1),
            CalendarDate::new(2026, 2, 28).unwrap()
        );
    }

    #[test]
    fn initializes_once_from_local_rtc_and_toggles_mode() {
        let mut state = CalendarUiState::default();
        state.initialize_if_needed(Some(RtcDateTime {
            year: 2026,
            month: 6,
            day: 4,
            weekday: 4,
            hour: 8,
            minute: 13,
            second: 0,
        }));
        assert_eq!(state.cursor, CalendarDate::new(2026, 6, 4).unwrap());
        state.toggle_mode();
        assert_eq!(state.mode, CalendarNavigationMode::Month);
    }

    #[test]
    fn parses_x4_personal_and_us_rows_without_hindu_loader() {
        let personal = parse_personal_events(
            "2026-06-01|Calendar Follow-up|Add month navigation after basic app proof.\n",
        )
        .unwrap();
        assert_eq!(personal.len(), 1);
        assert_eq!(personal[0].kind, CalendarEventKind::Personal);
        assert_eq!(personal[0].title, "Calendar Follow-up");

        let us =
            parse_us_events("# US calendar\n2026-06-19|US|Juneteenth|Federal holiday\n").unwrap();
        assert_eq!(us.len(), 1);
        assert_eq!(us[0].kind, CalendarEventKind::UsHoliday);
        assert_eq!(us[0].title, "Juneteenth");
    }

    #[test]
    fn missing_files_are_safe_and_hindu_file_is_ignored() {
        let root = temp_root("missing-safe");
        fs::write(
            root.join("HINDU26.TXT"),
            "2026-06-01|HINDU|Ignored|Must not be loaded\n",
        )
        .unwrap();
        let catalog = load_calendar_catalog(&root);
        assert!(catalog.events.is_empty());
        assert!(!catalog.personal_loaded);
        assert!(!catalog.us_loaded);
        assert!(catalog.warning.unwrap().contains("EVENTS.TXT missing"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn agenda_scrolling_tracks_selected_event_beyond_first_page() {
        let root = temp_root("agenda-scroll");
        let rows = (1..=8)
            .map(|index| format!("2026-06-19|Event {index}|Detail {index}"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(root.join("EVENTS.TXT"), rows).unwrap();
        fs::write(root.join("US2026.TXT"), "").unwrap();
        let mut state = CalendarUiState {
            cursor: CalendarDate::new(2026, 6, 19).unwrap(),
            ..CalendarUiState::default()
        };
        state.refresh_events_from_root(&root);
        for _ in 0..7 {
            state.agenda_next();
        }
        assert_eq!(state.agenda_selected, 7);
        assert_eq!(
            state.agenda_visible_range(),
            2..(2 + CALENDAR_AGENDA_VISIBLE_ROWS)
        );
        assert_eq!(state.selected_agenda_event().unwrap().title, "Event 8");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn personal_event_writer_uses_tmp_primary_and_backup_recovery() {
        let root = temp_root("atomic-personal");
        fs::write(root.join("EVENTS.TXT"), "2026-06-10|Original|Keep backup\n").unwrap();
        create_personal_event(
            &root,
            CalendarDate::new(2026, 6, 11).unwrap(),
            "Dentist",
            "Morning visit",
        )
        .unwrap();
        assert!(root.join("EVENTS.BAK").is_file());
        assert!(!root.join("EVENTS.TMP").exists());
        let current = fs::read_to_string(root.join("EVENTS.TXT")).unwrap();
        assert!(current.contains("2026-06-11|Dentist|Morning visit"));
        fs::write(root.join("EVENTS.TXT"), "bad-row\n").unwrap();
        let catalog = load_calendar_catalog(&root);
        assert!(catalog.personal_loaded);
        assert!(catalog
            .warning
            .unwrap()
            .contains("recovered from EVENTS.BAK"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn creates_edits_and_deletes_personal_rows_without_touching_us_pack() {
        let root = temp_root("personal-editor");
        fs::write(
            root.join("US2026.TXT"),
            "2026-07-04|US|Independence Day|Federal holiday\n",
        )
        .unwrap();
        create_personal_event(
            &root,
            CalendarDate::new(2026, 7, 4).unwrap(),
            "Picnic",
            "Bring snacks",
        )
        .unwrap();
        update_personal_event(&root, 0, "Family Picnic", "Bring snacks and water").unwrap();
        let us_before = fs::read_to_string(root.join("US2026.TXT")).unwrap();
        let personal =
            parse_personal_events(&fs::read_to_string(root.join("EVENTS.TXT")).unwrap()).unwrap();
        assert_eq!(personal[0].title, "Family Picnic");
        delete_personal_event(&root, 0).unwrap();
        assert!(fs::read_to_string(root.join("EVENTS.TXT"))
            .unwrap()
            .is_empty());
        assert_eq!(
            fs::read_to_string(root.join("US2026.TXT")).unwrap(),
            us_before
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn calendar_editor_reuses_boot_axis_keyboard_and_queues_save() {
        let mut editor = CalendarEventEditorState::create(CalendarDate::new(2026, 7, 4).unwrap());
        assert_eq!(editor.navigation_mode_label(), "NAV H");
        editor.keyboard_navigation.move_next();
        let selected = editor.selected_key_index();
        editor.toggle_navigation_axis();
        assert_eq!(editor.navigation_mode_label(), "NAV V");
        assert_eq!(editor.selected_key_index(), selected);
        editor.title = "Picnic".into();
        editor.keyboard_navigation =
            KeyboardGridNavigation::new(CALENDAR_EDITOR_KEY_COUNT, CALENDAR_EDITOR_KEY_COLUMNS);
        for _ in 0..5 {
            editor.keyboard_navigation.move_next();
        }
        editor.keyboard_navigation.toggle_axis();
        editor.keyboard_navigation.move_previous();
        assert_eq!(editor.selected_key_label(), "SAVE");
        assert!(matches!(
            editor.apply_button(ButtonEvent::Select),
            CalendarEditorOutcome::Save(CalendarUiRequest::CreatePersonal { .. })
        ));
    }
}
