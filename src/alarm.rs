//! RTC alarm schedules loaded from removable SD storage.
//!
//! The PCF85063 exposes one hardware alarm slot. This module owns a bounded
//! schedule list, computes the earliest local-time occurrence and exposes
//! runtime-only UI actions. Persistent edits remain file-based so the mounted
//! FAT filesystem stays read-only.

use std::{fs, path::Path};

use anyhow::{anyhow, bail, Context, Result};

use crate::{buttons::ButtonEvent, rtc::RtcDateTime};

/// Removable-SD alarm definition file.
pub const ALARMS_CONFIG_PATH: &str = "/sdcard/RUSTMIX/ALARMS.TXT";
/// Maximum alarm definitions retained from removable storage.
pub const MAX_ALARMS: usize = 6;
/// Maximum snooze interval accepted from configuration.
pub const MAX_SNOOZE_MINUTES: u8 = 60;
/// Default snooze interval.
pub const DEFAULT_SNOOZE_MINUTES: u8 = 10;
/// Bound the next-occurrence search to one leap-year window.
const MAX_SEARCH_DAYS: i32 = 370;

pub const SUN: u8 = 1 << 0;
pub const MON: u8 = 1 << 1;
pub const TUE: u8 = 1 << 2;
pub const WED: u8 = 1 << 3;
pub const THU: u8 = 1 << 4;
pub const FRI: u8 = 1 << 5;
pub const SAT: u8 = 1 << 6;
pub const EVERY_DAY: u8 = SUN | MON | TUE | WED | THU | FRI | SAT;
pub const WEEKDAYS: u8 = MON | TUE | WED | THU | FRI;
pub const WEEKENDS: u8 = SUN | SAT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlarmScheduleKind {
    /// Repeats on the configured weekday mask.
    Recurring { weekdays: u8 },
    /// Fires once on a local calendar date and then disables itself.
    OneTime { year: u16, month: u8, day: u8 },
}

impl AlarmScheduleKind {
    #[must_use]
    pub fn compact_label(self) -> String {
        match self {
            Self::Recurring { weekdays } => weekday_mask_label(weekdays),
            Self::OneTime { year, month, day } => format!("ONCE {year:04}-{month:02}-{day:02}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlarmDefinition {
    pub name: String,
    pub hour: u8,
    pub minute: u8,
    pub enabled: bool,
    pub schedule: AlarmScheduleKind,
}

impl AlarmDefinition {
    #[must_use]
    pub fn time_label(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    #[must_use]
    pub const fn status_label(&self) -> &'static str {
        if self.enabled {
            "ON"
        } else {
            "OFF"
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledOccurrence {
    pub alarm_index: usize,
    pub local: RtcDateTime,
    pub snooze: bool,
}

impl ScheduledOccurrence {
    #[must_use]
    pub fn label(&self, alarms: &[AlarmDefinition]) -> String {
        if self.snooze {
            return format!("SNOOZE {}", self.local.date_time());
        }
        let name = alarms
            .get(self.alarm_index)
            .map_or("alarm", |alarm| alarm.name.as_str());
        format!("{name} {}", self.local.date_time())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveAlarm {
    pub alarm_index: usize,
    pub name: String,
    pub local: RtcDateTime,
    pub snoozed: bool,
}

impl ActiveAlarm {
    #[must_use]
    pub fn label(&self) -> String {
        let prefix = if self.snoozed { "SNOOZE" } else { "ALARM" };
        format!(
            "{prefix}: {} {:02}:{:02}",
            self.name, self.local.hour, self.local.minute
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlarmEditField {
    Hour,
    Minute,
    Enabled,
    ScheduleKind,
    WeekdaysOrDate,
    Save,
}

impl AlarmEditField {
    const COUNT: usize = 6;

    #[must_use]
    pub const fn from_index(index: usize) -> Self {
        match index % Self::COUNT {
            0 => Self::Hour,
            1 => Self::Minute,
            2 => Self::Enabled,
            3 => Self::ScheduleKind,
            4 => Self::WeekdaysOrDate,
            _ => Self::Save,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Hour => "Hour",
            Self::Minute => "Minute",
            Self::Enabled => "Enabled",
            Self::ScheduleKind => "Mode",
            Self::WeekdaysOrDate => "Weekdays / date",
            Self::Save => "Save runtime edit",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlarmEditorSnapshot {
    pub alarm_index: usize,
    pub draft: AlarmDefinition,
    pub field_index: usize,
}

impl AlarmEditorSnapshot {
    #[must_use]
    pub const fn selected_field(&self) -> AlarmEditField {
        AlarmEditField::from_index(self.field_index)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlarmSnapshot {
    pub alarms: Vec<AlarmDefinition>,
    pub selected: usize,
    pub active: Option<ActiveAlarm>,
    pub editor: Option<AlarmEditorSnapshot>,
    pub next: Option<ScheduledOccurrence>,
    pub snooze_minutes: u8,
    pub config_loaded: bool,
    pub hardware_programmed: bool,
    pub error: Option<String>,
}

impl Default for AlarmSnapshot {
    fn default() -> Self {
        Self {
            alarms: Vec::new(),
            selected: 0,
            active: None,
            editor: None,
            next: None,
            snooze_minutes: DEFAULT_SNOOZE_MINUTES,
            config_loaded: false,
            hardware_programmed: false,
            error: None,
        }
    }
}

impl AlarmSnapshot {
    #[must_use]
    pub fn home_badge(&self) -> &'static str {
        if self.active.is_some() {
            "RING"
        } else if self.error.is_some() {
            "ERROR"
        } else if !self.config_loaded {
            "NO CFG"
        } else if self.alarms.iter().any(|alarm| alarm.enabled) {
            "ARMED"
        } else {
            "OFF"
        }
    }

    #[must_use]
    pub fn next_label(&self) -> String {
        self.next.as_ref().map_or_else(
            || "No enabled alarms".into(),
            |next| next.label(&self.alarms),
        )
    }

    #[must_use]
    pub fn row_count(&self) -> usize {
        if self.active.is_some() {
            2
        } else if self.editor.is_some() {
            AlarmEditField::COUNT
        } else {
            self.alarms.len()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlarmUiOutcome {
    None,
    SelectionChanged,
    EditorOpened,
    Edited,
    Saved,
    Snoozed,
    Dismissed,
    ReturnHome,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AlarmPollOutcome {
    pub triggered: bool,
    pub active_changed: bool,
    pub schedule_changed: bool,
}

/// Runtime alarm engine. Persistent definitions are loaded once from SD.
#[derive(Clone, Debug)]
pub struct AlarmEngine {
    alarms: Vec<AlarmDefinition>,
    snooze_minutes: u8,
    selected: usize,
    active: Option<ActiveAlarm>,
    editor: Option<AlarmEditorSnapshot>,
    snooze_until: Option<ScheduledOccurrence>,
    next: Option<ScheduledOccurrence>,
    config_loaded: bool,
    hardware_programmed: bool,
    error: Option<String>,
    last_trigger_key: Option<u64>,
}

impl Default for AlarmEngine {
    fn default() -> Self {
        Self {
            alarms: Vec::new(),
            snooze_minutes: DEFAULT_SNOOZE_MINUTES,
            selected: 0,
            active: None,
            editor: None,
            snooze_until: None,
            next: None,
            config_loaded: false,
            hardware_programmed: false,
            error: None,
            last_trigger_key: None,
        }
    }
}

impl AlarmEngine {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .with_context(|| format!("read alarm config {}", path.display()))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self> {
        let mut engine = Self::default();
        engine.config_loaded = true;
        let mut saw_snooze = false;

        for (line_number, raw_line) in text.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| anyhow!("line {} must contain '='", line_number + 1))?;
            match key.trim() {
                "snooze_minutes" => {
                    if saw_snooze {
                        bail!("duplicate snooze_minutes entry");
                    }
                    let minutes = value
                        .trim()
                        .parse::<u8>()
                        .context("invalid snooze_minutes")?;
                    if !(1..=MAX_SNOOZE_MINUTES).contains(&minutes) {
                        bail!("snooze_minutes must be in 1..={MAX_SNOOZE_MINUTES}");
                    }
                    engine.snooze_minutes = minutes;
                    saw_snooze = true;
                }
                "alarm" => {
                    if engine.alarms.len() >= MAX_ALARMS {
                        bail!("no more than {MAX_ALARMS} alarms are supported");
                    }
                    engine.alarms.push(parse_alarm(value.trim())?);
                }
                other => bail!("unsupported alarm config key {other:?}"),
            }
        }
        Ok(engine)
    }

    #[must_use]
    pub fn unavailable(error: impl Into<String>) -> Self {
        Self {
            error: Some(error.into()),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> AlarmSnapshot {
        AlarmSnapshot {
            alarms: self.alarms.clone(),
            selected: self.selected,
            active: self.active.clone(),
            editor: self.editor.clone(),
            next: self.next.clone(),
            snooze_minutes: self.snooze_minutes,
            config_loaded: self.config_loaded,
            hardware_programmed: self.hardware_programmed,
            error: self.error.clone(),
        }
    }

    pub fn set_hardware_programmed(&mut self, programmed: bool) {
        self.hardware_programmed = programmed;
    }

    #[must_use]
    pub fn next_occurrence(&self) -> Option<ScheduledOccurrence> {
        self.next.clone()
    }

    /// Recompute the earliest upcoming local occurrence.
    pub fn recompute_next(&mut self, now_local: RtcDateTime) -> bool {
        let previous = self.next.clone();
        let mut next = self
            .snooze_until
            .clone()
            .filter(|value| datetime_key(value.local) > datetime_key(now_local));
        for (index, alarm) in self.alarms.iter().enumerate() {
            if let Some(candidate) = next_for_alarm(index, alarm, now_local) {
                if next.as_ref().map_or(true, |value| {
                    datetime_key(candidate.local) < datetime_key(value.local)
                }) {
                    next = Some(candidate);
                }
            }
        }
        self.next = next;
        self.next != previous
    }

    /// Return whether minute polling is useful for this runtime state.
    #[must_use]
    pub fn should_poll(&self) -> bool {
        self.active.is_some()
            || self.snooze_until.is_some()
            || self.alarms.iter().any(|alarm| alarm.enabled)
    }

    /// Poll local time. The hardware flag is advisory; matching the current
    /// minute also keeps alarms functional when no interrupt GPIO is attached.
    pub fn poll(&mut self, now_local: RtcDateTime, hardware_flag: bool) -> AlarmPollOutcome {
        let mut outcome = AlarmPollOutcome::default();
        if self.active.is_some() {
            return outcome;
        }
        let due = self.due_occurrence(now_local);
        if due.is_none() && !hardware_flag {
            outcome.schedule_changed = self.recompute_next(now_local);
            return outcome;
        }
        let Some(next) = due else {
            // A stale hardware flag should be cleared by the caller without
            // producing a false product alarm.
            outcome.schedule_changed = self.recompute_next(now_local);
            return outcome;
        };
        let trigger_key = datetime_key_minute(now_local);
        if self.last_trigger_key == Some(trigger_key) {
            return outcome;
        }
        self.last_trigger_key = Some(trigger_key);
        let name = self
            .alarms
            .get(next.alarm_index)
            .map_or_else(|| "Snoozed alarm".into(), |alarm| alarm.name.clone());
        if next.snooze {
            self.snooze_until = None;
        }
        self.selected = 0;
        self.active = Some(ActiveAlarm {
            alarm_index: next.alarm_index,
            name,
            local: now_local,
            snoozed: next.snooze,
        });
        self.next = None;
        outcome.triggered = true;
        outcome.active_changed = true;
        outcome.schedule_changed = true;
        outcome
    }

    fn due_occurrence(&self, now_local: RtcDateTime) -> Option<ScheduledOccurrence> {
        if let Some(snooze) = self.snooze_until.as_ref() {
            if same_minute(snooze.local, now_local) {
                return Some(snooze.clone());
            }
        }
        self.alarms.iter().enumerate().find_map(|(index, alarm)| {
            (alarm.enabled && alarm_matches_local_minute(alarm, now_local)).then_some(
                ScheduledOccurrence {
                    alarm_index: index,
                    local: now_local,
                    snooze: false,
                },
            )
        })
    }

    pub fn apply_button(&mut self, event: ButtonEvent, now_local: RtcDateTime) -> AlarmUiOutcome {
        if self.active.is_some() {
            return self.apply_active_button(event, now_local);
        }
        if self.editor.is_some() {
            return self.apply_editor_button(event, now_local);
        }
        let row_count = self.alarms.len();
        if row_count == 0 {
            self.selected = 0;
            return AlarmUiOutcome::None;
        }
        self.selected %= row_count;
        match event {
            ButtonEvent::Up => {
                self.selected = self.selected.checked_sub(1).unwrap_or(row_count - 1);
                AlarmUiOutcome::SelectionChanged
            }
            ButtonEvent::Down => {
                self.selected = (self.selected + 1) % row_count;
                AlarmUiOutcome::SelectionChanged
            }
            ButtonEvent::Select => {
                self.editor = Some(AlarmEditorSnapshot {
                    alarm_index: self.selected,
                    draft: self.alarms[self.selected].clone(),
                    field_index: 0,
                });
                AlarmUiOutcome::EditorOpened
            }
        }
    }

    fn apply_active_button(
        &mut self,
        event: ButtonEvent,
        now_local: RtcDateTime,
    ) -> AlarmUiOutcome {
        match event {
            ButtonEvent::Up | ButtonEvent::Down => {
                self.selected = (self.selected + 1) % 2;
                AlarmUiOutcome::SelectionChanged
            }
            ButtonEvent::Select if self.selected == 0 => {
                let active = self.active.take().expect("active alarm checked above");
                self.snooze_until = Some(ScheduledOccurrence {
                    alarm_index: active.alarm_index,
                    local: now_local.shift_minutes(i32::from(self.snooze_minutes)),
                    snooze: true,
                });
                self.selected = 0;
                self.recompute_next(now_local);
                AlarmUiOutcome::Snoozed
            }
            ButtonEvent::Select => {
                self.dismiss_active(now_local);
                AlarmUiOutcome::Dismissed
            }
        }
    }

    fn apply_editor_button(
        &mut self,
        event: ButtonEvent,
        now_local: RtcDateTime,
    ) -> AlarmUiOutcome {
        match event {
            ButtonEvent::Up => {
                self.adjust_editor(1, now_local);
                AlarmUiOutcome::Edited
            }
            ButtonEvent::Down => {
                self.adjust_editor(-1, now_local);
                AlarmUiOutcome::Edited
            }
            ButtonEvent::Select => {
                let save = self
                    .editor
                    .as_ref()
                    .is_some_and(|editor| editor.selected_field() == AlarmEditField::Save);
                if save {
                    let editor = self.editor.take().expect("editor checked above");
                    self.selected = editor.alarm_index;
                    self.alarms[editor.alarm_index] = editor.draft;
                    self.recompute_next(now_local);
                    AlarmUiOutcome::Saved
                } else {
                    if let Some(editor) = self.editor.as_mut() {
                        editor.field_index = (editor.field_index + 1) % AlarmEditField::COUNT;
                    }
                    AlarmUiOutcome::SelectionChanged
                }
            }
        }
    }

    fn adjust_editor(&mut self, delta: i32, now_local: RtcDateTime) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        match editor.selected_field() {
            AlarmEditField::Hour => editor.draft.hour = wrap_u8(editor.draft.hour, delta, 24),
            AlarmEditField::Minute => editor.draft.minute = wrap_u8(editor.draft.minute, delta, 60),
            AlarmEditField::Enabled => editor.draft.enabled = !editor.draft.enabled,
            AlarmEditField::ScheduleKind => {
                editor.draft.schedule = match editor.draft.schedule {
                    AlarmScheduleKind::Recurring { .. } => {
                        let date = if datetime_key(RtcDateTime {
                            hour: editor.draft.hour,
                            minute: editor.draft.minute,
                            second: 0,
                            ..now_local
                        }) > datetime_key(now_local)
                        {
                            now_local
                        } else {
                            now_local.shift_minutes(24 * 60)
                        };
                        AlarmScheduleKind::OneTime {
                            year: date.year,
                            month: date.month,
                            day: date.day,
                        }
                    }
                    AlarmScheduleKind::OneTime { .. } => AlarmScheduleKind::Recurring {
                        weekdays: EVERY_DAY,
                    },
                };
            }
            AlarmEditField::WeekdaysOrDate => {
                editor.draft.schedule = match editor.draft.schedule {
                    AlarmScheduleKind::Recurring { weekdays } => AlarmScheduleKind::Recurring {
                        weekdays: cycle_weekday_mask(weekdays, delta),
                    },
                    AlarmScheduleKind::OneTime { year, month, day } => {
                        let weekday = weekday_for_date(year, month, day);
                        let shifted = RtcDateTime {
                            year,
                            month,
                            day,
                            weekday,
                            hour: 0,
                            minute: 0,
                            second: 0,
                        }
                        .shift_minutes(delta * 24 * 60);
                        AlarmScheduleKind::OneTime {
                            year: shifted.year,
                            month: shifted.month,
                            day: shifted.day,
                        }
                    }
                };
            }
            AlarmEditField::Save => {}
        }
    }

    fn dismiss_active(&mut self, now_local: RtcDateTime) {
        if let Some(active) = self.active.take() {
            if matches!(
                self.alarms
                    .get(active.alarm_index)
                    .map(|alarm| alarm.schedule),
                Some(AlarmScheduleKind::OneTime { .. })
            ) {
                if let Some(alarm) = self.alarms.get_mut(active.alarm_index) {
                    alarm.enabled = false;
                }
            }
        }
        self.snooze_until = None;
        self.selected = 0;
        self.recompute_next(now_local);
    }
}

fn wrap_u8(value: u8, delta: i32, modulus: i32) -> u8 {
    (i32::from(value) + delta).rem_euclid(modulus) as u8
}

fn cycle_weekday_mask(current: u8, delta: i32) -> u8 {
    const OPTIONS: [u8; 10] = [
        EVERY_DAY, WEEKDAYS, WEEKENDS, SUN, MON, TUE, WED, THU, FRI, SAT,
    ];
    let index = OPTIONS
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0) as i32;
    OPTIONS[(index + delta).rem_euclid(OPTIONS.len() as i32) as usize]
}

fn parse_alarm(value: &str) -> Result<AlarmDefinition> {
    let fields: Vec<&str> = value.split(',').map(str::trim).collect();
    if fields.len() != 5 {
        bail!("alarm must use name,time,schedule,on|off,recurring|once");
    }
    let name = fields[0];
    if name.is_empty() || name.len() > 20 {
        bail!("alarm name must contain 1..=20 bytes");
    }
    let (hour, minute) = parse_time(fields[1])?;
    let enabled = parse_enabled(fields[3])?;
    let schedule = match fields[4] {
        "recurring" => AlarmScheduleKind::Recurring {
            weekdays: parse_weekdays(fields[2])?,
        },
        "once" => {
            let (year, month, day) = parse_date(fields[2])?;
            AlarmScheduleKind::OneTime { year, month, day }
        }
        _ => bail!("alarm kind must be recurring or once"),
    };
    Ok(AlarmDefinition {
        name: name.into(),
        hour,
        minute,
        enabled,
        schedule,
    })
}

fn parse_time(value: &str) -> Result<(u8, u8)> {
    let (hour, minute) = value
        .split_once(':')
        .ok_or_else(|| anyhow!("alarm time must use HH:MM"))?;
    let hour = hour.parse::<u8>().context("invalid alarm hour")?;
    let minute = minute.parse::<u8>().context("invalid alarm minute")?;
    if hour > 23 || minute > 59 {
        bail!("alarm time is outside 00:00..23:59");
    }
    Ok((hour, minute))
}

fn parse_enabled(value: &str) -> Result<bool> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => bail!("alarm enabled field must be on or off"),
    }
}

fn parse_date(value: &str) -> Result<(u16, u8, u8)> {
    let mut values = value.split('-');
    let year = values
        .next()
        .ok_or_else(|| anyhow!("missing one-time year"))?
        .parse::<u16>()?;
    let month = values
        .next()
        .ok_or_else(|| anyhow!("missing one-time month"))?
        .parse::<u8>()?;
    let day = values
        .next()
        .ok_or_else(|| anyhow!("missing one-time day"))?
        .parse::<u8>()?;
    if values.next().is_some()
        || !(2000..=2099).contains(&year)
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
    {
        bail!("one-time date must be a valid YYYY-MM-DD value in 2000..=2099");
    }
    Ok((year, month, day))
}

fn parse_weekdays(value: &str) -> Result<u8> {
    match value {
        "daily" => return Ok(EVERY_DAY),
        "weekdays" => return Ok(WEEKDAYS),
        "weekends" => return Ok(WEEKENDS),
        _ => {}
    }
    let mut mask = 0_u8;
    for token in value.split('|') {
        mask |= match token {
            "sun" => SUN,
            "mon" => MON,
            "tue" => TUE,
            "wed" => WED,
            "thu" => THU,
            "fri" => FRI,
            "sat" => SAT,
            _ => bail!("unsupported weekday token {token:?}"),
        };
    }
    if mask == 0 {
        bail!("recurring alarm requires at least one weekday");
    }
    Ok(mask)
}

fn alarm_matches_local_minute(alarm: &AlarmDefinition, now: RtcDateTime) -> bool {
    if alarm.hour != now.hour || alarm.minute != now.minute {
        return false;
    }
    match alarm.schedule {
        AlarmScheduleKind::Recurring { weekdays } => weekdays & (1 << now.weekday) != 0,
        AlarmScheduleKind::OneTime { year, month, day } => {
            year == now.year && month == now.month && day == now.day
        }
    }
}

fn next_for_alarm(
    index: usize,
    alarm: &AlarmDefinition,
    now: RtcDateTime,
) -> Option<ScheduledOccurrence> {
    if !alarm.enabled {
        return None;
    }
    match alarm.schedule {
        AlarmScheduleKind::OneTime { year, month, day } => {
            let local = RtcDateTime {
                year,
                month,
                day,
                weekday: weekday_for_date(year, month, day),
                hour: alarm.hour,
                minute: alarm.minute,
                second: 0,
            };
            (datetime_key(local) > datetime_key(now)).then_some(ScheduledOccurrence {
                alarm_index: index,
                local,
                snooze: false,
            })
        }
        AlarmScheduleKind::Recurring { weekdays } => {
            for day_delta in 0..=MAX_SEARCH_DAYS {
                let date = now.shift_minutes(day_delta * 24 * 60);
                let local = RtcDateTime {
                    hour: alarm.hour,
                    minute: alarm.minute,
                    second: 0,
                    ..date
                };
                if weekdays & (1 << local.weekday) != 0 && datetime_key(local) > datetime_key(now) {
                    return Some(ScheduledOccurrence {
                        alarm_index: index,
                        local,
                        snooze: false,
                    });
                }
            }
            None
        }
    }
}

#[must_use]
pub fn weekday_mask_label(mask: u8) -> String {
    if mask == EVERY_DAY {
        return "DAILY".into();
    }
    if mask == WEEKDAYS {
        return "WEEKDAYS".into();
    }
    if mask == WEEKENDS {
        return "WEEKENDS".into();
    }
    let names = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
    names
        .iter()
        .enumerate()
        .filter_map(|(index, name)| (mask & (1 << index) != 0).then_some(*name))
        .collect::<Vec<_>>()
        .join("|")
}

fn same_minute(left: RtcDateTime, right: RtcDateTime) -> bool {
    datetime_key_minute(left) == datetime_key_minute(right)
}

fn datetime_key_minute(value: RtcDateTime) -> u64 {
    u64::from(value.year) * 100_000_000
        + u64::from(value.month) * 1_000_000
        + u64::from(value.day) * 10_000
        + u64::from(value.hour) * 100
        + u64::from(value.minute)
}

fn datetime_key(value: RtcDateTime) -> u64 {
    datetime_key_minute(value) * 100 + u64::from(value.second)
}

fn weekday_for_date(year: u16, month: u8, day: u8) -> u8 {
    let table = [0_i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut year = i32::from(year);
    if month < 3 {
        year -= 1;
    }
    (year + year / 4 - year / 100 + year / 400 + table[usize::from(month - 1)] + i32::from(day))
        .rem_euclid(7) as u8
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(year: u16, month: u8, day: u8, weekday: u8, hour: u8, minute: u8) -> RtcDateTime {
        RtcDateTime {
            year,
            month,
            day,
            weekday,
            hour,
            minute,
            second: 0,
        }
    }

    #[test]
    fn parses_recurring_and_one_time_alarm_definitions() {
        let engine = AlarmEngine::parse(
            "\
snooze_minutes=9\n\
alarm=Work,07:30,weekdays,on,recurring\n\
alarm=Flight,05:45,2026-06-10,on,once\n",
        )
        .unwrap();
        assert_eq!(engine.snooze_minutes, 9);
        assert_eq!(engine.alarms.len(), 2);
        assert_eq!(
            engine.alarms[0].schedule,
            AlarmScheduleKind::Recurring { weekdays: WEEKDAYS }
        );
        assert_eq!(
            engine.alarms[1].schedule,
            AlarmScheduleKind::OneTime {
                year: 2026,
                month: 6,
                day: 10
            }
        );
    }

    #[test]
    fn rejects_invalid_and_excessive_configuration() {
        assert!(AlarmEngine::parse("alarm=Bad,25:00,daily,on,recurring\n").is_err());
        assert!(AlarmEngine::parse("alarm=Bad,07:00,never,on,recurring\n").is_err());
        assert!(AlarmEngine::parse("snooze_minutes=0\n").is_err());
    }

    #[test]
    fn computes_next_weekday_occurrence() {
        let mut engine = AlarmEngine::parse("alarm=Work,07:30,weekdays,on,recurring\n").unwrap();
        let now = local(2026, 6, 5, 5, 8, 0); // Friday after alarm time.
        engine.recompute_next(now);
        let next = engine.next_occurrence().unwrap();
        assert_eq!(next.local, local(2026, 6, 8, 1, 7, 30));
    }

    #[test]
    fn triggers_snoozes_and_dismisses_without_persistent_write() {
        let mut engine =
            AlarmEngine::parse("snooze_minutes=10\nalarm=Daily,07:30,daily,on,recurring\n")
                .unwrap();
        let due = local(2026, 6, 3, 3, 7, 30);
        assert!(engine.poll(due, true).triggered);
        assert!(engine.snapshot().active.is_some());
        assert_eq!(
            engine.apply_button(ButtonEvent::Select, due),
            AlarmUiOutcome::Snoozed
        );
        assert_eq!(engine.snapshot().next.unwrap().local.minute, 40);
        let snooze_due = local(2026, 6, 3, 3, 7, 40);
        assert!(engine.poll(snooze_due, true).triggered);
        engine.apply_button(ButtonEvent::Down, snooze_due);
        assert_eq!(
            engine.apply_button(ButtonEvent::Select, snooze_due),
            AlarmUiOutcome::Dismissed
        );
        assert!(engine.snapshot().active.is_none());
    }

    #[test]
    fn one_time_alarm_disables_after_dismiss() {
        let mut engine = AlarmEngine::parse("alarm=Once,08:00,2026-06-03,on,once\n").unwrap();
        let due = local(2026, 6, 3, 3, 8, 0);
        assert!(engine.poll(due, true).triggered);
        engine.apply_button(ButtonEvent::Down, due);
        engine.apply_button(ButtonEvent::Select, due);
        assert!(!engine.snapshot().alarms[0].enabled);
    }

    #[test]
    fn runtime_editor_changes_hour_minute_enabled_kind_and_schedule_value() {
        let mut engine = AlarmEngine::parse("alarm=Daily,07:30,daily,on,recurring\n").unwrap();
        let now = local(2026, 6, 3, 3, 6, 0);
        engine.recompute_next(now);
        assert_eq!(
            engine.apply_button(ButtonEvent::Select, now),
            AlarmUiOutcome::EditorOpened
        );
        assert_eq!(
            engine.apply_button(ButtonEvent::Up, now),
            AlarmUiOutcome::Edited
        ); // hour 08
        engine.apply_button(ButtonEvent::Select, now);
        engine.apply_button(ButtonEvent::Down, now); // minute 29
        engine.apply_button(ButtonEvent::Select, now);
        engine.apply_button(ButtonEvent::Up, now); // enabled off
        engine.apply_button(ButtonEvent::Select, now);
        engine.apply_button(ButtonEvent::Up, now); // one time
        engine.apply_button(ButtonEvent::Select, now);
        engine.apply_button(ButtonEvent::Up, now); // date +1
        engine.apply_button(ButtonEvent::Select, now);
        assert_eq!(
            engine.apply_button(ButtonEvent::Select, now),
            AlarmUiOutcome::Saved
        );
        let saved = &engine.snapshot().alarms[0];
        assert_eq!((saved.hour, saved.minute, saved.enabled), (8, 29, false));
        assert!(matches!(saved.schedule, AlarmScheduleKind::OneTime { .. }));
    }
}
