//! Native SD-backed voice-note catalog and recovery-safe PCM WAV recorder.
//!
//! The firmware keeps ES8311 / I2S ownership in the native hardware runtime.
//! This module owns only bounded catalog state, WAV framing and streamed SD
//! writes so it can be tested on the host without ESP-IDF handles.

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{anyhow, Context, Result};

use crate::{
    buttons::ButtonEvent,
    keyboard_navigation::KeyboardGridNavigation,
    voice_note_metadata::{
        default_voice_title, delete_voice_note_metadata, load_voice_note_metadata,
        metadata_for_file, rename_voice_note_title, sanitize_voice_title, title_from_editable,
        upsert_voice_note_metadata, VoiceNoteMetadata, VOICE_TITLE_MAX_CHARS,
        VOICE_UNKNOWN_RECORDED_AT,
    },
};

pub const VOICE_NOTES_ROOT: &str = "/sdcard/RUSTMIX/VOICE";
pub const VOICE_NOTES_INDEX_FILE: &str = "INDEX.TXT";
pub const VOICE_SAMPLE_RATE_HZ: u32 = 16_000;
pub const VOICE_BITS_PER_SAMPLE: u16 = 16;
pub const VOICE_CHANNELS: u16 = 1;
pub const VOICE_PCM_MONO_CHUNK_BYTES: usize = 4_096;
pub const VOICE_PCM_STEREO_CAPTURE_BYTES: usize = VOICE_PCM_MONO_CHUNK_BYTES * 2;
pub const VOICE_RECORD_MAX_SECONDS: u32 = 30 * 60;
pub const VOICE_NOTE_LIMIT: usize = 128;
pub const VOICE_NOTE_LIST_VISIBLE_ROWS: usize = 6;
pub const VOICE_RECORD_SCREEN_REFRESH_SECONDS: u64 = 5;
/// Shared-grid keyboard used by the friendly-title editor.
pub const VOICE_TITLE_EDITOR_KEY_ROWS: [[&str; 7]; 6] = [
    ["A", "B", "C", "D", "E", "F", "G"],
    ["H", "I", "J", "K", "L", "M", "N"],
    ["O", "P", "Q", "R", "S", "T", "U"],
    ["V", "W", "X", "Y", "Z", "0", "1"],
    ["2", "3", "4", "5", "6", "7", "8"],
    ["9", "-", "_", "SP", "DEL", "SAVE", "CANCEL"],
];
const VOICE_TITLE_EDITOR_KEY_COLUMNS: usize = 7;
const VOICE_TITLE_EDITOR_KEY_COUNT: usize = 42;
const WAV_HEADER_BYTES: usize = 44;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum VoiceMicGain {
    Low,
    Normal,
    #[default]
    High,
    Boost,
}

impl VoiceMicGain {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Normal => "NORMAL",
            Self::High => "HIGH",
            Self::Boost => "BOOST",
        }
    }

    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Boost => "boost",
        }
    }

    #[must_use]
    pub const fn multiplier(self) -> i32 {
        match self {
            Self::Low => 1,
            Self::Normal => 2,
            Self::High => 3,
            Self::Boost => 4,
        }
    }

    #[must_use]
    pub const fn db_label(self) -> &'static str {
        match self {
            Self::Low => "+0 DB",
            Self::Normal => "+6 DB",
            Self::High => "+9 DB",
            Self::Boost => "+12 DB",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Low => Self::Normal,
            Self::Normal => Self::High,
            Self::High => Self::Boost,
            Self::Boost => Self::Low,
        }
    }

    #[must_use]
    pub fn from_marker(marker: &str) -> Option<Self> {
        match marker.trim().to_ascii_lowercase().as_str() {
            "low" => Some(Self::Low),
            "normal" => Some(Self::Normal),
            "high" => Some(Self::High),
            "boost" => Some(Self::Boost),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VoiceCaptureMetrics {
    pub bytes: usize,
    pub peak: u16,
    pub clipped_samples: u32,
}

#[must_use]
pub fn apply_pcm16_gain_in_place(bytes: &mut [u8], gain: VoiceMicGain) -> VoiceCaptureMetrics {
    let mut metrics = VoiceCaptureMetrics {
        bytes: bytes.len(),
        ..VoiceCaptureMetrics::default()
    };
    for sample in bytes.chunks_exact_mut(2) {
        let raw = i16::from_le_bytes([sample[0], sample[1]]);
        let scaled = i32::from(raw).saturating_mul(gain.multiplier());
        let clipped = scaled.clamp(i32::from(i16::MIN), i32::from(i16::MAX));
        if clipped != scaled {
            metrics.clipped_samples = metrics.clipped_samples.saturating_add(1);
        }
        let output = clipped as i16;
        metrics.peak = metrics.peak.max(output.unsigned_abs());
        sample.copy_from_slice(&output.to_le_bytes());
    }
    metrics
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum VoiceNotesMode {
    #[default]
    Idle,
    Recording,
    Paused,
    Playing,
    Saved,
    Error,
}

impl VoiceNotesMode {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "READY",
            Self::Recording => "RECORDING",
            Self::Paused => "PAUSED",
            Self::Playing => "PLAYING",
            Self::Saved => "SAVED",
            Self::Error => "ERROR",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoiceNoteEntry {
    /// Stable FAT 8.3 storage name retained for WAV portability.
    pub file_name: String,
    /// Friendly display title stored in `META.TXT`.
    pub title: String,
    /// Local RTC timestamp captured when recording begins.
    pub recorded_at: String,
    pub wav_bytes: u64,
    pub pcm_bytes: u32,
    pub duration_seconds: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VoiceNotesUiRequest {
    StartRecording,
    StopRecording,
    PauseRecording,
    ResumeRecording,
    CancelRecording,
    StartPlayback,
    StopPlayback,
    PersistMicGain(VoiceMicGain),
    SaveEditedTitle { file_name: String, title: String },
    ExportSelected,
    DeleteSelected,
    RefreshCatalog,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoiceNotesUiState {
    pub notes: Vec<VoiceNoteEntry>,
    /// List cursor. Row zero is always `Record new note`.
    pub selected: usize,
    /// Detail cursor: play or stop, delete, or return.
    pub detail_selected: usize,
    pub mode: VoiceNotesMode,
    pub active_file: Option<String>,
    pub active_recorded_at: Option<String>,
    pub elapsed_seconds: u32,
    pub pcm_bytes: u32,
    pub peak: u16,
    pub clipped_samples: u32,
    pub mic_gain: VoiceMicGain,
    pub recording_paused: bool,
    pub playing_file: Option<String>,
    pub playback_pcm_bytes: u32,
    pub playback_total_pcm_bytes: u32,
    pub available_storage_bytes: Option<u64>,
    pub delete_confirmation: bool,
    pub delete_confirm_selected: usize,
    pub title_editing: bool,
    pub title_edit_navigation: KeyboardGridNavigation,
    pub title_edit_buffer: Vec<char>,
    pub export_file: Option<String>,
    pub export_status: Option<String>,
    pub error: Option<String>,
    request: Option<VoiceNotesUiRequest>,
}

impl Default for VoiceNotesUiState {
    fn default() -> Self {
        Self {
            notes: Vec::new(),
            selected: 0,
            detail_selected: 0,
            mode: VoiceNotesMode::Idle,
            active_file: None,
            active_recorded_at: None,
            elapsed_seconds: 0,
            pcm_bytes: 0,
            peak: 0,
            clipped_samples: 0,
            mic_gain: VoiceMicGain::default(),
            recording_paused: false,
            playing_file: None,
            playback_pcm_bytes: 0,
            playback_total_pcm_bytes: 0,
            available_storage_bytes: None,
            delete_confirmation: false,
            delete_confirm_selected: 0,
            title_editing: false,
            title_edit_navigation: KeyboardGridNavigation::new(
                VOICE_TITLE_EDITOR_KEY_COUNT,
                VOICE_TITLE_EDITOR_KEY_COLUMNS,
            ),
            title_edit_buffer: Vec::new(),
            export_file: None,
            export_status: None,
            error: None,
            request: None,
        }
    }
}

impl VoiceNotesUiState {
    pub fn refresh_catalog(&mut self) {
        match scan_voice_notes(Path::new(VOICE_NOTES_ROOT)) {
            Ok(notes) => {
                self.notes = notes;
                self.selected = self.selected.min(self.notes.len().saturating_add(1));
                self.error = None;
                if self.mode == VoiceNotesMode::Error {
                    self.mode = VoiceNotesMode::Idle;
                }
            }
            Err(error) => self.fail(format!("{error:#}")),
        }
    }

    pub fn apply_list_button(&mut self, event: ButtonEvent) -> bool {
        let rows = self.notes.len().saturating_add(2).max(2);
        match event {
            ButtonEvent::Up => {
                self.selected = self.selected.checked_sub(1).unwrap_or(rows - 1);
                false
            }
            ButtonEvent::Down => {
                self.selected = (self.selected + 1) % rows;
                false
            }
            ButtonEvent::Select if self.selected == 0 => {
                self.request = Some(VoiceNotesUiRequest::StartRecording);
                true
            }
            ButtonEvent::Select if self.selected == 1 => {
                self.mic_gain = self.mic_gain.next();
                self.request = Some(VoiceNotesUiRequest::PersistMicGain(self.mic_gain));
                false
            }
            ButtonEvent::Select => {
                self.detail_selected = 0;
                false
            }
        }
    }

    pub fn apply_detail_button(&mut self, event: ButtonEvent) -> bool {
        if self.title_editing {
            self.apply_title_edit_button(event);
            return false;
        }
        if self.delete_confirmation {
            match event {
                ButtonEvent::Up | ButtonEvent::Down => {
                    self.delete_confirm_selected = (self.delete_confirm_selected + 1) % 2;
                    false
                }
                ButtonEvent::Select if self.delete_confirm_selected == 0 => {
                    self.delete_confirmation = false;
                    false
                }
                ButtonEvent::Select => {
                    self.delete_confirmation = false;
                    self.request = Some(VoiceNotesUiRequest::DeleteSelected);
                    true
                }
            }
        } else {
            match event {
                ButtonEvent::Up => {
                    self.detail_selected = self.detail_selected.checked_sub(1).unwrap_or(4);
                    false
                }
                ButtonEvent::Down => {
                    self.detail_selected = (self.detail_selected + 1) % 5;
                    false
                }
                ButtonEvent::Select if self.detail_selected == 0 => {
                    self.request = Some(if self.is_playing_selected() {
                        VoiceNotesUiRequest::StopPlayback
                    } else {
                        VoiceNotesUiRequest::StartPlayback
                    });
                    true
                }
                ButtonEvent::Select if self.detail_selected == 1 => {
                    self.begin_title_edit();
                    false
                }
                ButtonEvent::Select if self.detail_selected == 2 => {
                    self.request = Some(VoiceNotesUiRequest::ExportSelected);
                    true
                }
                ButtonEvent::Select if self.detail_selected == 3 => {
                    self.delete_confirmation = true;
                    self.delete_confirm_selected = 0;
                    false
                }
                ButtonEvent::Select => false,
            }
        }
    }

    pub fn apply_recording_button(&mut self, event: ButtonEvent) {
        self.request = match event {
            ButtonEvent::Select => Some(VoiceNotesUiRequest::StopRecording),
            ButtonEvent::Up | ButtonEvent::Down if self.recording_paused => {
                Some(VoiceNotesUiRequest::ResumeRecording)
            }
            ButtonEvent::Up | ButtonEvent::Down => Some(VoiceNotesUiRequest::PauseRecording),
        };
    }

    pub fn request_cancel_recording(&mut self) {
        if matches!(
            self.mode,
            VoiceNotesMode::Recording | VoiceNotesMode::Paused
        ) {
            self.request = Some(VoiceNotesUiRequest::CancelRecording);
        }
    }

    pub fn request_stop_playback(&mut self) {
        if self.playing_file.is_some() {
            self.request = Some(VoiceNotesUiRequest::StopPlayback);
        }
    }

    #[must_use]
    pub fn take_request(&mut self) -> Option<VoiceNotesUiRequest> {
        self.request.take()
    }

    pub fn begin_recording(&mut self, file_name: String, recorded_at: String) {
        self.mode = VoiceNotesMode::Recording;
        self.active_file = Some(file_name);
        self.active_recorded_at = Some(recorded_at);
        self.elapsed_seconds = 0;
        self.pcm_bytes = 0;
        self.peak = 0;
        self.clipped_samples = 0;
        self.recording_paused = false;
        self.playing_file = None;
        self.playback_pcm_bytes = 0;
        self.playback_total_pcm_bytes = 0;
        self.error = None;
    }

    pub fn update_recording_progress(&mut self, pcm_bytes: u32, peak: u16, clipped_samples: u32) {
        self.mode = if self.recording_paused {
            VoiceNotesMode::Paused
        } else {
            VoiceNotesMode::Recording
        };
        self.pcm_bytes = pcm_bytes;
        self.elapsed_seconds = pcm_bytes / bytes_per_second();
        self.peak = peak;
        self.clipped_samples = clipped_samples;
    }

    pub fn pause_recording(&mut self) {
        if self.mode == VoiceNotesMode::Recording {
            self.recording_paused = true;
            self.mode = VoiceNotesMode::Paused;
        }
    }

    pub fn resume_recording(&mut self) {
        if self.recording_paused {
            self.recording_paused = false;
            self.mode = VoiceNotesMode::Recording;
        }
    }

    pub fn complete_recording(&mut self, entry: VoiceNoteEntry) {
        self.mode = VoiceNotesMode::Saved;
        self.active_file = Some(entry.file_name.clone());
        self.active_recorded_at = Some(entry.recorded_at.clone());
        self.elapsed_seconds = entry.duration_seconds;
        self.pcm_bytes = entry.pcm_bytes;
        self.notes.push(entry);
        self.notes
            .sort_by(|left, right| left.file_name.cmp(&right.file_name));
        self.selected = self
            .active_file
            .as_deref()
            .and_then(|file| self.notes.iter().position(|entry| entry.file_name == file))
            .map_or(0, |index| index + 2);
        self.error = None;
    }

    pub fn cancel_recording(&mut self) {
        self.mode = VoiceNotesMode::Idle;
        self.active_file = None;
        self.active_recorded_at = None;
        self.elapsed_seconds = 0;
        self.pcm_bytes = 0;
        self.peak = 0;
        self.clipped_samples = 0;
        self.recording_paused = false;
        self.error = None;
    }

    pub fn begin_playback(&mut self, file_name: String, total_pcm_bytes: u32) {
        self.mode = VoiceNotesMode::Playing;
        self.playing_file = Some(file_name);
        self.playback_pcm_bytes = 0;
        self.playback_total_pcm_bytes = total_pcm_bytes;
        self.error = None;
    }

    pub fn update_playback_progress(&mut self, pcm_bytes: u32, total_pcm_bytes: u32) {
        self.mode = VoiceNotesMode::Playing;
        self.playback_pcm_bytes = pcm_bytes.min(total_pcm_bytes);
        self.playback_total_pcm_bytes = total_pcm_bytes;
    }

    pub fn stop_playback(&mut self) {
        self.playing_file = None;
        self.playback_pcm_bytes = 0;
        self.playback_total_pcm_bytes = 0;
        if self.mode == VoiceNotesMode::Playing {
            self.mode = VoiceNotesMode::Idle;
        }
    }

    #[must_use]
    pub fn playback_elapsed_seconds(&self) -> u32 {
        self.playback_pcm_bytes / bytes_per_second()
    }

    #[must_use]
    pub fn is_playing_selected(&self) -> bool {
        self.selected_note().is_some_and(|note| {
            self.playing_file
                .as_deref()
                .is_some_and(|file| file == note.file_name.as_str())
        })
    }

    #[must_use]
    pub fn visible_note_range(&self) -> Range<usize> {
        let selected_note = self.selected.saturating_sub(2).min(self.notes.len());
        let start = selected_note
            .saturating_add(1)
            .saturating_sub(VOICE_NOTE_LIST_VISIBLE_ROWS)
            .min(
                self.notes
                    .len()
                    .saturating_sub(VOICE_NOTE_LIST_VISIBLE_ROWS),
            );
        start..(start + VOICE_NOTE_LIST_VISIBLE_ROWS).min(self.notes.len())
    }

    pub fn begin_title_edit(&mut self) {
        let Some(note) = self.selected_note() else {
            return;
        };
        self.title_edit_buffer = sanitize_voice_title(&note.title).chars().collect();
        self.title_edit_navigation = KeyboardGridNavigation::new(
            VOICE_TITLE_EDITOR_KEY_COUNT,
            VOICE_TITLE_EDITOR_KEY_COLUMNS,
        );
        self.title_editing = true;
    }

    #[must_use]
    pub const fn title_editor_navigation_mode_label(&self) -> &'static str {
        self.title_edit_navigation.status_label()
    }

    #[must_use]
    pub const fn title_editor_selected_key_index(&self) -> usize {
        self.title_edit_navigation.selected()
    }

    pub fn toggle_title_editor_navigation_axis(&mut self) -> bool {
        if !self.title_editing {
            return false;
        }
        self.title_edit_navigation.toggle_axis();
        true
    }

    pub fn apply_title_edit_button(&mut self, event: ButtonEvent) {
        match event {
            ButtonEvent::Up => self.title_edit_navigation.move_previous(),
            ButtonEvent::Down => self.title_edit_navigation.move_next(),
            ButtonEvent::Select => self.apply_title_edit_selected_key(),
        }
    }

    fn apply_title_edit_selected_key(&mut self) {
        match voice_title_editor_key(self.title_edit_navigation.selected()) {
            "SP" => self.push_title_edit_character(' '),
            "DEL" => {
                self.title_edit_buffer.pop();
            }
            "SAVE" => self.finish_title_edit(),
            "CANCEL" => self.cancel_title_edit(),
            label => {
                if let Some(character) = label.chars().next() {
                    self.push_title_edit_character(character);
                }
            }
        }
    }

    fn push_title_edit_character(&mut self, character: char) {
        if self.title_edit_buffer.len() < VOICE_TITLE_MAX_CHARS {
            self.title_edit_buffer.push(character);
        }
    }

    pub fn cancel_title_edit(&mut self) {
        self.title_editing = false;
        self.title_edit_buffer.clear();
    }

    pub fn finish_title_edit(&mut self) {
        if !self.title_editing {
            return;
        }
        self.title_editing = false;
        let Some(file_name) = self.selected_note().map(|note| note.file_name.clone()) else {
            return;
        };
        let title = title_from_editable(&self.title_edit_buffer);
        if let Some(note) = self
            .notes
            .iter_mut()
            .find(|note| note.file_name == file_name)
        {
            note.title.clone_from(&title);
        }
        self.request = Some(VoiceNotesUiRequest::SaveEditedTitle { file_name, title });
    }

    pub fn set_available_storage_bytes(&mut self, available_storage_bytes: Option<u64>) {
        self.available_storage_bytes = available_storage_bytes;
    }

    pub fn mark_export_requested(&mut self, file_name: String) {
        self.export_file = Some(file_name);
        self.export_status = Some("STARTING LAN PORTAL".into());
    }

    pub fn mark_export_ready(&mut self, status: impl Into<String>) {
        self.export_status = Some(status.into());
    }

    pub fn clear_transient_details(&mut self) {
        self.delete_confirmation = false;
        self.title_editing = false;
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.mode = VoiceNotesMode::Error;
        self.error = Some(error.into());
    }

    #[must_use]
    pub fn selected_note(&self) -> Option<&VoiceNoteEntry> {
        self.selected
            .checked_sub(2)
            .and_then(|index| self.notes.get(index))
    }

    pub fn remove_selected_note(&mut self) {
        if let Some(index) = self.selected.checked_sub(2) {
            if index < self.notes.len() {
                self.notes.remove(index);
            }
        }
        self.selected = self.selected.min(self.notes.len().saturating_add(1));
        self.mode = VoiceNotesMode::Idle;
        self.active_file = None;
        self.active_recorded_at = None;
        self.stop_playback();
        self.delete_confirmation = false;
        self.title_editing = false;
        self.export_file = None;
        self.export_status = None;
        self.error = None;
    }
}

#[must_use]
pub const fn bytes_per_second() -> u32 {
    VOICE_SAMPLE_RATE_HZ * VOICE_CHANNELS as u32 * (VOICE_BITS_PER_SAMPLE as u32 / 8)
}

#[must_use]
pub fn format_duration(seconds: u32) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[must_use]
pub fn is_voice_wav_name(name: &str) -> bool {
    is_voice_file_name_with_extension(name, ".WAV")
}

#[must_use]
pub fn is_voice_tmp_name(name: &str) -> bool {
    is_voice_file_name_with_extension(name, ".TMP")
}

#[must_use]
fn is_voice_file_name_with_extension(name: &str, extension: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    if upper.len() != 12 || !upper.starts_with("VOICE") || !upper.ends_with(extension) {
        return false;
    }
    upper.as_bytes()[5..8].iter().all(u8::is_ascii_digit)
}

pub fn cleanup_stale_voice_tmp(root: &Path) -> Result<usize> {
    fs::create_dir_all(root)
        .with_context(|| format!("create voice-note root {}", root.display()))?;
    let mut removed = 0;
    for entry in
        fs::read_dir(root).with_context(|| format!("scan voice-note root {}", root.display()))?
    {
        let entry = entry?;
        let hinted_type = entry.file_type().ok();
        if hinted_type
            .as_ref()
            .is_some_and(|file_type| file_type.is_symlink())
        {
            continue;
        }
        let metadata = entry.metadata()?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_ascii_uppercase();
        if is_voice_tmp_name(&file_name) {
            fs::remove_file(entry.path())
                .with_context(|| format!("remove stale voice note {}", entry.path().display()))?;
            removed += 1;
        }
    }
    Ok(removed)
}

#[must_use]
pub fn next_voice_file_name(root: &Path, notes: &[VoiceNoteEntry]) -> Option<String> {
    (1..=999)
        .map(|index| format!("VOICE{index:03}.WAV"))
        .find(|candidate| {
            !root.join(candidate).exists()
                && notes.iter().all(|entry| entry.file_name != *candidate)
        })
}

pub fn scan_voice_notes(root: &Path) -> Result<Vec<VoiceNoteEntry>> {
    fs::create_dir_all(root)
        .with_context(|| format!("create voice-note root {}", root.display()))?;
    let voice_note_metadata = load_voice_note_metadata(root).unwrap_or_default();
    let mut notes = Vec::new();
    for entry in
        fs::read_dir(root).with_context(|| format!("scan voice-note root {}", root.display()))?
    {
        let entry = entry?;
        let hinted_type = entry.file_type().ok();
        if hinted_type
            .as_ref()
            .is_some_and(|file_type| file_type.is_symlink())
        {
            continue;
        }

        // ESP-IDF FAT VFS may expose an incomplete d_type hint from readdir.
        // Match the accepted SD browser and sleep-image scanner: use the
        // immediately following stat metadata as the final classification.
        let file_metadata = entry.metadata()?;
        if !file_metadata.file_type().is_file() || file_metadata.file_type().is_symlink() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_ascii_uppercase();
        if !is_voice_wav_name(&file_name) {
            continue;
        }
        let note_metadata = metadata_for_file(&voice_note_metadata, &file_name);
        match read_voice_note_entry_with_metadata(&entry.path(), file_name, note_metadata) {
            Ok(note) => notes.push(note),
            Err(_) => continue,
        }
        if notes.len() >= VOICE_NOTE_LIMIT {
            break;
        }
    }
    notes.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    Ok(notes)
}

pub fn read_voice_note_entry(path: &Path, file_name: String) -> Result<VoiceNoteEntry> {
    let metadata = metadata_for_file(&[], &file_name);
    read_voice_note_entry_with_metadata(path, file_name, metadata)
}

fn read_voice_note_entry_with_metadata(
    path: &Path,
    file_name: String,
    metadata: VoiceNoteMetadata,
) -> Result<VoiceNoteEntry> {
    let mut file = File::open(path).with_context(|| format!("open WAV {}", path.display()))?;
    let mut header = [0_u8; WAV_HEADER_BYTES];
    file.read_exact(&mut header)
        .with_context(|| format!("read WAV header {}", path.display()))?;
    let pcm_bytes = parse_pcm_wav_header(&header)?;
    let wav_bytes = file.metadata()?.len();
    Ok(VoiceNoteEntry {
        file_name,
        title: metadata.title,
        recorded_at: metadata.recorded_at,
        wav_bytes,
        pcm_bytes,
        duration_seconds: pcm_bytes / bytes_per_second(),
    })
}

#[must_use]
pub fn build_pcm_wav_header(pcm_bytes: u32) -> [u8; WAV_HEADER_BYTES] {
    let mut bytes = [0_u8; WAV_HEADER_BYTES];
    bytes[0..4].copy_from_slice(b"RIFF");
    bytes[4..8].copy_from_slice(&(36_u32.saturating_add(pcm_bytes)).to_le_bytes());
    bytes[8..12].copy_from_slice(b"WAVE");
    bytes[12..16].copy_from_slice(b"fmt ");
    bytes[16..20].copy_from_slice(&16_u32.to_le_bytes());
    bytes[20..22].copy_from_slice(&1_u16.to_le_bytes());
    bytes[22..24].copy_from_slice(&VOICE_CHANNELS.to_le_bytes());
    bytes[24..28].copy_from_slice(&VOICE_SAMPLE_RATE_HZ.to_le_bytes());
    bytes[28..32].copy_from_slice(&bytes_per_second().to_le_bytes());
    bytes[32..34].copy_from_slice(&(VOICE_CHANNELS * (VOICE_BITS_PER_SAMPLE / 8)).to_le_bytes());
    bytes[34..36].copy_from_slice(&VOICE_BITS_PER_SAMPLE.to_le_bytes());
    bytes[36..40].copy_from_slice(b"data");
    bytes[40..44].copy_from_slice(&pcm_bytes.to_le_bytes());
    bytes
}

pub fn parse_pcm_wav_header(header: &[u8; WAV_HEADER_BYTES]) -> Result<u32> {
    if &header[0..4] != b"RIFF"
        || &header[8..12] != b"WAVE"
        || &header[12..16] != b"fmt "
        || &header[36..40] != b"data"
    {
        return Err(anyhow!("unsupported WAV container"));
    }
    if u16::from_le_bytes([header[20], header[21]]) != 1
        || u16::from_le_bytes([header[22], header[23]]) != VOICE_CHANNELS
        || u32::from_le_bytes([header[24], header[25], header[26], header[27]])
            != VOICE_SAMPLE_RATE_HZ
        || u16::from_le_bytes([header[34], header[35]]) != VOICE_BITS_PER_SAMPLE
    {
        return Err(anyhow!("unsupported WAV format; require PCM16 mono 16 kHz"));
    }
    Ok(u32::from_le_bytes([
        header[40], header[41], header[42], header[43],
    ]))
}

#[must_use]
pub fn expand_pcm16_mono_to_stereo(mono: &[u8], stereo: &mut [u8]) -> Result<usize> {
    if mono.len() % 2 != 0 || stereo.len() < mono.len().saturating_mul(2) {
        return Err(anyhow!("invalid PCM16 mono-to-stereo buffers"));
    }
    for (input, output) in mono.chunks_exact(2).zip(stereo.chunks_exact_mut(4)) {
        output[..2].copy_from_slice(input);
        output[2..].copy_from_slice(input);
    }
    Ok(mono.len() * 2)
}

pub struct VoicePlaybackSession {
    file_name: String,
    file: File,
    total_pcm_bytes: u32,
    played_pcm_bytes: u32,
}

impl VoicePlaybackSession {
    pub fn open(root: &Path, file_name: &str) -> Result<Self> {
        if !is_voice_wav_name(file_name) {
            return Err(anyhow!("unsafe voice-note filename"));
        }
        let path = root.join(file_name);
        let mut file = File::open(&path).with_context(|| format!("open WAV {}", path.display()))?;
        let mut header = [0_u8; WAV_HEADER_BYTES];
        file.read_exact(&mut header)
            .with_context(|| format!("read WAV header {}", path.display()))?;
        let total_pcm_bytes = parse_pcm_wav_header(&header)?;
        if total_pcm_bytes % 2 != 0 {
            return Err(anyhow!("voice-note WAV has odd PCM16 data length"));
        }
        let required_bytes = WAV_HEADER_BYTES as u64 + u64::from(total_pcm_bytes);
        if file.metadata()?.len() < required_bytes {
            return Err(anyhow!("truncated voice-note WAV {}", path.display()));
        }
        Ok(Self {
            file_name: file_name.to_ascii_uppercase(),
            file,
            total_pcm_bytes,
            played_pcm_bytes: 0,
        })
    }

    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    #[must_use]
    pub const fn total_pcm_bytes(&self) -> u32 {
        self.total_pcm_bytes
    }

    #[must_use]
    pub const fn played_pcm_bytes(&self) -> u32 {
        self.played_pcm_bytes
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.played_pcm_bytes >= self.total_pcm_bytes
    }

    pub fn read_pcm16_mono(&mut self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < 2 {
            return Err(anyhow!("voice playback buffer is too small"));
        }
        let available = self.total_pcm_bytes.saturating_sub(self.played_pcm_bytes) as usize;
        let bytes = available.min(buffer.len()) & !1;
        if bytes == 0 {
            return Ok(0);
        }
        self.file.read_exact(&mut buffer[..bytes])?;
        self.played_pcm_bytes = self.played_pcm_bytes.saturating_add(bytes as u32);
        Ok(bytes)
    }
}

pub struct VoiceRecordingSession {
    root: PathBuf,
    temp_path: PathBuf,
    final_path: PathBuf,
    final_name: String,
    file: File,
    started_at: Instant,
    recorded_at: String,
    pcm_bytes: u32,
    peak: u16,
    clipped_samples: u32,
}

impl VoiceRecordingSession {
    pub fn start(root: &Path) -> Result<Self> {
        Self::start_with_recorded_at(root, VOICE_UNKNOWN_RECORDED_AT.into())
    }

    pub fn start_with_recorded_at(root: &Path, recorded_at: String) -> Result<Self> {
        let notes = scan_voice_notes(root)?;
        if notes.len() >= VOICE_NOTE_LIMIT {
            return Err(anyhow!("voice-note catalog limit reached"));
        }
        let final_name = next_voice_file_name(root, &notes)
            .ok_or_else(|| anyhow!("no FAT 8.3 voice-note filename is available"))?;
        let temp_name = final_name.replace(".WAV", ".TMP");
        let final_path = root.join(&final_name);
        let temp_path = root.join(&temp_name);
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
            .open(&temp_path)
            .with_context(|| format!("create temporary voice note {}", temp_path.display()))?;
        file.write_all(&build_pcm_wav_header(0))?;
        file.flush()?;
        Ok(Self {
            root: root.to_path_buf(),
            temp_path,
            final_path,
            final_name,
            file,
            started_at: Instant::now(),
            recorded_at,
            pcm_bytes: 0,
            peak: 0,
            clipped_samples: 0,
        })
    }

    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.final_name
    }

    #[must_use]
    pub const fn pcm_bytes(&self) -> u32 {
        self.pcm_bytes
    }

    #[must_use]
    pub const fn peak(&self) -> u16 {
        self.peak
    }

    #[must_use]
    pub const fn clipped_samples(&self) -> u32 {
        self.clipped_samples
    }

    pub fn add_clipped_samples(&mut self, clipped_samples: u32) {
        self.clipped_samples = self.clipped_samples.saturating_add(clipped_samples);
    }

    #[must_use]
    pub fn elapsed_seconds(&self) -> u32 {
        self.started_at.elapsed().as_secs().min(u64::from(u32::MAX)) as u32
    }

    pub fn append_pcm16_mono(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.len() % 2 != 0 {
            return Err(anyhow!("PCM16 chunk has odd byte length"));
        }
        let next = self.pcm_bytes.saturating_add(bytes.len() as u32);
        if next > bytes_per_second().saturating_mul(VOICE_RECORD_MAX_SECONDS) {
            return Err(anyhow!("recording limit reached"));
        }
        self.file.write_all(bytes)?;
        self.pcm_bytes = next;
        for sample in bytes.chunks_exact(2) {
            let magnitude = i16::from_le_bytes([sample[0], sample[1]]).unsigned_abs();
            self.peak = self.peak.max(magnitude);
        }
        Ok(())
    }

    pub fn finalize(mut self) -> Result<VoiceNoteEntry> {
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&build_pcm_wav_header(self.pcm_bytes))?;
        self.file.flush()?;
        self.file.sync_all()?;
        drop(self.file);
        if self.final_path.exists() {
            return Err(anyhow!(
                "refuse to replace existing voice note {}",
                self.final_path.display()
            ));
        }
        fs::rename(&self.temp_path, &self.final_path)
            .with_context(|| format!("commit voice note {}", self.final_path.display()))?;
        let metadata = VoiceNoteMetadata {
            file_name: self.final_name.clone(),
            title: default_voice_title(&self.final_name),
            recorded_at: self.recorded_at,
        };
        upsert_voice_note_metadata(&self.root, metadata.clone())?;
        let entry =
            read_voice_note_entry_with_metadata(&self.final_path, self.final_name, metadata)?;
        rebuild_index(&self.root)?;
        Ok(entry)
    }

    pub fn cancel(self) -> Result<()> {
        drop(self.file);
        if self.temp_path.exists() {
            fs::remove_file(&self.temp_path)?;
        }
        Ok(())
    }
}

pub fn delete_voice_note(root: &Path, file_name: &str) -> Result<()> {
    if !is_voice_wav_name(file_name) {
        return Err(anyhow!("unsafe voice-note filename"));
    }
    let path = root.join(file_name);
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("delete voice note {}", path.display()))?;
    }
    delete_voice_note_metadata(root, file_name)?;
    rebuild_index(root)
}

pub fn save_voice_note_title(root: &Path, file_name: &str, title: &str) -> Result<()> {
    if !is_voice_wav_name(file_name) {
        return Err(anyhow!("unsafe voice-note filename"));
    }
    rename_voice_note_title(root, file_name, title)
}

pub fn rebuild_index(root: &Path) -> Result<()> {
    let notes = scan_voice_notes(root)?;
    let temp = root.join("INDEX.TMP");
    let final_path = root.join(VOICE_NOTES_INDEX_FILE);
    let mut file = File::create(&temp)?;
    for note in notes {
        writeln!(
            file,
            "{}|{}|{}",
            note.file_name, note.duration_seconds, note.wav_bytes
        )?;
    }
    file.flush()?;
    file.sync_all()?;
    drop(file);
    if final_path.exists() {
        fs::remove_file(&final_path)?;
    }
    fs::rename(temp, final_path)?;
    Ok(())
}

#[must_use]
fn voice_title_editor_key(index: usize) -> &'static str {
    VOICE_TITLE_EDITOR_KEY_ROWS
        .iter()
        .flatten()
        .nth(index)
        .copied()
        .unwrap_or("CANCEL")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn temporary_root(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustmix-wave-{label}-{unique}"))
    }

    #[test]
    fn wav_header_is_pcm16_mono_16khz() {
        let header = build_pcm_wav_header(32_000);
        assert_eq!(parse_pcm_wav_header(&header).unwrap(), 32_000);
        assert_eq!(bytes_per_second(), 32_000);
    }

    #[test]
    fn streamed_tmp_recording_finalizes_to_fat83_wav_and_index() {
        let root = temporary_root("voice-record");
        let mut session = VoiceRecordingSession::start(&root).unwrap();
        assert_eq!(session.file_name(), "VOICE001.WAV");
        session
            .append_pcm16_mono(&[1, 0, 2, 0, 3, 0, 4, 0])
            .unwrap();
        let note = session.finalize().unwrap();
        assert_eq!(note.file_name, "VOICE001.WAV");
        assert_eq!(note.pcm_bytes, 8);
        assert!(root.join("INDEX.TXT").exists());
        assert!(!root.join("VOICE001.TMP").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn consecutive_recordings_allocate_distinct_names_and_preserve_prior_wav() {
        let root = temporary_root("voice-sequential");
        let mut first = VoiceRecordingSession::start(&root).unwrap();
        first.append_pcm16_mono(&[1, 0, 2, 0]).unwrap();
        assert_eq!(first.finalize().unwrap().file_name, "VOICE001.WAV");

        let mut second = VoiceRecordingSession::start(&root).unwrap();
        assert_eq!(second.file_name(), "VOICE002.WAV");
        second.append_pcm16_mono(&[3, 0, 4, 0]).unwrap();
        assert_eq!(second.finalize().unwrap().file_name, "VOICE002.WAV");

        let notes = scan_voice_notes(&root).unwrap();
        assert_eq!(notes.len(), 2);
        assert!(root.join("VOICE001.WAV").exists());
        assert!(root.join("VOICE002.WAV").exists());
        let index = fs::read_to_string(root.join("INDEX.TXT")).unwrap();
        assert!(index.contains("VOICE001.WAV"));
        assert!(index.contains("VOICE002.WAV"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn allocator_skips_existing_wav_even_when_catalog_omits_it() {
        let root = temporary_root("voice-existing-guard");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("VOICE001.WAV"), b"existing but malformed").unwrap();
        let session = VoiceRecordingSession::start(&root).unwrap();
        assert_eq!(session.file_name(), "VOICE002.WAV");
        session.cancel().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finalize_refuses_to_replace_existing_target() {
        let root = temporary_root("voice-finalize-existing-guard");
        let mut session = VoiceRecordingSession::start(&root).unwrap();
        session.append_pcm16_mono(&[1, 0, 2, 0]).unwrap();
        fs::write(root.join("VOICE001.WAV"), b"preserve-me").unwrap();
        let error = session.finalize().unwrap_err();
        assert!(error
            .to_string()
            .contains("refuse to replace existing voice note"));
        assert_eq!(fs::read(root.join("VOICE001.WAV")).unwrap(), b"preserve-me");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cancel_removes_partial_tmp_file() {
        let root = temporary_root("voice-cancel");
        let session = VoiceRecordingSession::start(&root).unwrap();
        assert!(root.join("VOICE001.TMP").exists());
        session.cancel().unwrap();
        assert!(!root.join("VOICE001.TMP").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gain_profiles_cycle_and_saturate_pcm16_safely() {
        assert_eq!(VoiceMicGain::default(), VoiceMicGain::High);
        assert_eq!(VoiceMicGain::High.next(), VoiceMicGain::Boost);
        let mut bytes = [0x00, 0x40, 0x00, 0xE0];
        let metrics = apply_pcm16_gain_in_place(&mut bytes, VoiceMicGain::Boost);
        assert_eq!(metrics.bytes, 4);
        assert_eq!(metrics.clipped_samples, 1);
        assert_eq!(i16::from_le_bytes([bytes[0], bytes[1]]), i16::MAX);
        assert_eq!(i16::from_le_bytes([bytes[2], bytes[3]]), -32768);
    }

    #[test]
    fn list_row_one_cycles_microphone_gain_without_opening_details() {
        let mut ui = VoiceNotesUiState::default();
        ui.selected = 1;
        let initial = ui.mic_gain;
        assert!(!ui.apply_list_button(ButtonEvent::Select));
        assert_eq!(ui.mic_gain, initial.next());
        assert_eq!(
            ui.take_request(),
            Some(VoiceNotesUiRequest::PersistMicGain(initial.next()))
        );
    }

    #[test]
    fn playback_streams_bounded_pcm_and_expands_mono_to_stereo() {
        let root = temporary_root("voice-playback");
        let mut record = VoiceRecordingSession::start(&root).unwrap();
        record.append_pcm16_mono(&[1, 0, 2, 0, 3, 0, 4, 0]).unwrap();
        record.finalize().unwrap();

        let mut playback = VoicePlaybackSession::open(&root, "VOICE001.WAV").unwrap();
        let mut mono = [0_u8; 4];
        let mut stereo = [0_u8; 8];
        assert_eq!(playback.read_pcm16_mono(&mut mono).unwrap(), 4);
        assert_eq!(expand_pcm16_mono_to_stereo(&mono, &mut stereo).unwrap(), 8);
        assert_eq!(stereo, [1, 0, 1, 0, 2, 0, 2, 0]);
        assert_eq!(playback.read_pcm16_mono(&mut mono).unwrap(), 4);
        assert!(playback.is_complete());
        assert_eq!(playback.read_pcm16_mono(&mut mono).unwrap(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn boot_cleanup_removes_stale_recording_tmp_but_preserves_index_tmp() {
        let root = temporary_root("voice-stale-tmp");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("VOICE001.TMP"), b"partial").unwrap();
        fs::write(root.join("INDEX.TMP"), b"preserve").unwrap();
        assert_eq!(cleanup_stale_voice_tmp(&root).unwrap(), 1);
        assert!(!root.join("VOICE001.TMP").exists());
        assert!(root.join("INDEX.TMP").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn visible_note_window_tracks_selected_rows_beyond_first_page() {
        let mut ui = VoiceNotesUiState::default();
        for index in 1..=8 {
            ui.notes.push(VoiceNoteEntry {
                file_name: format!("VOICE{index:03}.WAV"),
                title: format!("VOICE NOTE {index:03}"),
                recorded_at: VOICE_UNKNOWN_RECORDED_AT.into(),
                wav_bytes: 44,
                pcm_bytes: 0,
                duration_seconds: 0,
            });
        }
        ui.selected = 9;
        assert_eq!(ui.visible_note_range(), 2..8);
    }

    #[test]
    fn detail_primary_action_toggles_playback_request() {
        let mut ui = VoiceNotesUiState::default();
        ui.notes.push(VoiceNoteEntry {
            file_name: "VOICE001.WAV".into(),
            title: "VOICE NOTE 001".into(),
            recorded_at: VOICE_UNKNOWN_RECORDED_AT.into(),
            wav_bytes: 44,
            pcm_bytes: 0,
            duration_seconds: 0,
        });
        ui.selected = 2;
        assert!(ui.apply_detail_button(ButtonEvent::Select));
        assert_eq!(ui.take_request(), Some(VoiceNotesUiRequest::StartPlayback));
        ui.begin_playback("VOICE001.WAV".into(), 0);
        assert!(ui.apply_detail_button(ButtonEvent::Select));
        assert_eq!(ui.take_request(), Some(VoiceNotesUiRequest::StopPlayback));
    }

    #[test]
    fn ui_state_routes_start_stop_and_delete_requests() {
        let mut ui = VoiceNotesUiState::default();
        assert!(ui.apply_list_button(ButtonEvent::Select));
        assert_eq!(ui.take_request(), Some(VoiceNotesUiRequest::StartRecording));
        ui.begin_recording("VOICE001.WAV".into(), VOICE_UNKNOWN_RECORDED_AT.into());
        ui.apply_recording_button(ButtonEvent::Select);
        assert_eq!(ui.take_request(), Some(VoiceNotesUiRequest::StopRecording));
        ui.notes.push(VoiceNoteEntry {
            file_name: "VOICE001.WAV".into(),
            title: "VOICE NOTE 001".into(),
            recorded_at: VOICE_UNKNOWN_RECORDED_AT.into(),
            wav_bytes: 44,
            pcm_bytes: 0,
            duration_seconds: 0,
        });
        ui.selected = 2;
        ui.detail_selected = 3;
        assert!(!ui.apply_detail_button(ButtonEvent::Select));
        assert!(ui.delete_confirmation);
        ui.delete_confirm_selected = 1;
        assert!(ui.apply_detail_button(ButtonEvent::Select));
        assert_eq!(ui.take_request(), Some(VoiceNotesUiRequest::DeleteSelected));
    }

    #[test]
    fn recording_pause_resume_routes_without_finalizing_tmp_file() {
        let mut ui = VoiceNotesUiState::default();
        ui.begin_recording("VOICE001.WAV".into(), VOICE_UNKNOWN_RECORDED_AT.into());
        ui.apply_recording_button(ButtonEvent::Up);
        assert_eq!(ui.take_request(), Some(VoiceNotesUiRequest::PauseRecording));
        ui.pause_recording();
        assert_eq!(ui.mode, VoiceNotesMode::Paused);
        ui.apply_recording_button(ButtonEvent::Down);
        assert_eq!(
            ui.take_request(),
            Some(VoiceNotesUiRequest::ResumeRecording)
        );
        ui.resume_recording();
        assert_eq!(ui.mode, VoiceNotesMode::Recording);
    }

    #[test]
    fn title_editor_reuses_boot_axis_grid_and_saves_friendly_label() {
        let mut ui = VoiceNotesUiState::default();
        ui.notes.push(VoiceNoteEntry {
            file_name: "VOICE001.WAV".into(),
            title: "A".into(),
            recorded_at: "2026-06-05  21:37:08".into(),
            wav_bytes: 44,
            pcm_bytes: 0,
            duration_seconds: 0,
        });
        ui.selected = 2;
        ui.begin_title_edit();
        assert_eq!(ui.title_editor_navigation_mode_label(), "NAV H");
        assert_eq!(ui.title_editor_selected_key_index(), 0);
        assert!(ui.toggle_title_editor_navigation_axis());
        assert_eq!(ui.title_editor_navigation_mode_label(), "NAV V");
        assert_eq!(ui.title_editor_selected_key_index(), 0);
        ui.apply_title_edit_button(ButtonEvent::Down);
        assert_eq!(ui.title_editor_selected_key_index(), 7);
        ui.apply_title_edit_button(ButtonEvent::Select);
        ui.finish_title_edit();
        assert_eq!(ui.notes[0].file_name, "VOICE001.WAV");
        assert_eq!(ui.notes[0].title, "AH");
        assert_eq!(
            ui.take_request(),
            Some(VoiceNotesUiRequest::SaveEditedTitle {
                file_name: "VOICE001.WAV".into(),
                title: "AH".into(),
            })
        );
    }

    #[test]
    fn paused_recording_can_still_be_cancelled_by_hierarchical_back() {
        let mut ui = VoiceNotesUiState::default();
        ui.begin_recording("VOICE001.WAV".into(), VOICE_UNKNOWN_RECORDED_AT.into());
        ui.pause_recording();
        ui.request_cancel_recording();
        assert_eq!(
            ui.take_request(),
            Some(VoiceNotesUiRequest::CancelRecording)
        );
    }

    #[test]
    fn recording_timestamp_survives_catalog_rescan_via_metadata_sidecar() {
        let root = temporary_root("voice-recorded-at");
        let mut session =
            VoiceRecordingSession::start_with_recorded_at(&root, "2026-06-05  21:37:08".into())
                .unwrap();
        session.append_pcm16_mono(&[1, 0, 2, 0]).unwrap();
        session.finalize().unwrap();
        let notes = scan_voice_notes(&root).unwrap();
        assert_eq!(notes[0].recorded_at, "2026-06-05  21:37:08");
        assert_eq!(notes[0].title, "VOICE NOTE 001");
        fs::remove_dir_all(root).unwrap();
    }
}
