//! FAT 8.3-safe voice-note metadata and preferences sidecars.
//!
//! Finalized audio remains stored as `VOICE###.WAV`.  Friendly titles,
//! captured local timestamps and the selected microphone-gain profile live in
//! small atomic text sidecars so the WAV files remain portable and the native
//! recorder stays recovery-safe.

use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use anyhow::{Context, Result};

use crate::voice_notes::VoiceMicGain;

pub const VOICE_NOTES_METADATA_FILE: &str = "META.TXT";
pub const VOICE_NOTES_METADATA_TMP_FILE: &str = "META.TMP";
pub const VOICE_NOTES_SETTINGS_FILE: &str = "SETTINGS.TXT";
pub const VOICE_NOTES_SETTINGS_TMP_FILE: &str = "SETTINGS.TMP";
pub const VOICE_TITLE_MAX_CHARS: usize = 20;
pub const VOICE_UNKNOWN_RECORDED_AT: &str = "DATE UNKNOWN";
const TITLE_ALPHABET: &[u8] = b" ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoiceNoteMetadata {
    pub file_name: String,
    pub recorded_at: String,
    pub title: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VoiceNotesPreferences {
    pub mic_gain: VoiceMicGain,
}

impl Default for VoiceNotesPreferences {
    fn default() -> Self {
        Self {
            mic_gain: VoiceMicGain::default(),
        }
    }
}

pub fn load_voice_notes_preferences(root: &Path) -> Result<VoiceNotesPreferences> {
    let Some(text) = read_sidecar_text(root, VOICE_NOTES_SETTINGS_FILE)? else {
        return Ok(VoiceNotesPreferences::default());
    };
    let mut preferences = VoiceNotesPreferences::default();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("MIC_GAIN=") {
            if let Some(gain) = VoiceMicGain::from_marker(value.trim()) {
                preferences.mic_gain = gain;
            }
        }
    }
    Ok(preferences)
}

pub fn save_voice_notes_preferences(root: &Path, preferences: VoiceNotesPreferences) -> Result<()> {
    fs::create_dir_all(root)
        .with_context(|| format!("create voice-note root {}", root.display()))?;
    atomic_replace_text(
        root,
        VOICE_NOTES_SETTINGS_TMP_FILE,
        VOICE_NOTES_SETTINGS_FILE,
        &format!("MIC_GAIN={}\n", preferences.mic_gain.marker()),
    )
}

pub fn load_voice_note_metadata(root: &Path) -> Result<Vec<VoiceNoteMetadata>> {
    let Some(text) = read_sidecar_text(root, VOICE_NOTES_METADATA_FILE)? else {
        return Ok(Vec::new());
    };
    let mut metadata = Vec::new();
    for line in text.lines() {
        let mut fields = line.splitn(3, '|');
        let Some(file_name) = fields.next() else {
            continue;
        };
        let Some(recorded_at) = fields.next() else {
            continue;
        };
        let Some(title) = fields.next() else {
            continue;
        };
        let file_name = file_name.trim().to_ascii_uppercase();
        if !crate::voice_notes::is_voice_wav_name(&file_name) {
            continue;
        }
        metadata.push(VoiceNoteMetadata {
            file_name,
            recorded_at: sanitize_recorded_at(recorded_at),
            title: sanitize_voice_title(title),
        });
    }
    metadata.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    metadata.dedup_by(|right, left| left.file_name == right.file_name);
    Ok(metadata)
}

pub fn upsert_voice_note_metadata(root: &Path, entry: VoiceNoteMetadata) -> Result<()> {
    let mut metadata = load_voice_note_metadata(root)?;
    let file_name = entry.file_name.to_ascii_uppercase();
    let normalized = VoiceNoteMetadata {
        file_name: file_name.clone(),
        recorded_at: sanitize_recorded_at(&entry.recorded_at),
        title: sanitize_voice_title(&entry.title),
    };
    if let Some(existing) = metadata.iter_mut().find(|item| item.file_name == file_name) {
        *existing = normalized;
    } else {
        metadata.push(normalized);
    }
    write_voice_note_metadata(root, &metadata)
}

pub fn delete_voice_note_metadata(root: &Path, file_name: &str) -> Result<()> {
    let upper = file_name.to_ascii_uppercase();
    let mut metadata = load_voice_note_metadata(root)?;
    metadata.retain(|item| item.file_name != upper);
    write_voice_note_metadata(root, &metadata)
}

pub fn rename_voice_note_title(root: &Path, file_name: &str, title: &str) -> Result<()> {
    let upper = file_name.to_ascii_uppercase();
    let mut metadata = load_voice_note_metadata(root)?;
    let title = sanitize_voice_title(title);
    if let Some(existing) = metadata.iter_mut().find(|item| item.file_name == upper) {
        existing.title = title;
    } else {
        metadata.push(VoiceNoteMetadata {
            file_name: upper.clone(),
            recorded_at: VOICE_UNKNOWN_RECORDED_AT.into(),
            title,
        });
    }
    write_voice_note_metadata(root, &metadata)
}

pub fn metadata_for_file(metadata: &[VoiceNoteMetadata], file_name: &str) -> VoiceNoteMetadata {
    let upper = file_name.to_ascii_uppercase();
    metadata
        .iter()
        .find(|entry| entry.file_name == upper)
        .cloned()
        .unwrap_or_else(|| VoiceNoteMetadata {
            file_name: upper.clone(),
            recorded_at: VOICE_UNKNOWN_RECORDED_AT.into(),
            title: default_voice_title(&upper),
        })
}

#[must_use]
pub fn default_voice_title(file_name: &str) -> String {
    let upper = file_name.to_ascii_uppercase();
    upper
        .strip_prefix("VOICE")
        .and_then(|rest| rest.strip_suffix(".WAV"))
        .map_or_else(
            || "VOICE NOTE".into(),
            |number| format!("VOICE NOTE {number}"),
        )
}

#[must_use]
pub fn sanitize_voice_title(title: &str) -> String {
    let mut value = title
        .chars()
        .map(|character| character.to_ascii_uppercase())
        .filter(|character| character.is_ascii())
        .filter(|character| TITLE_ALPHABET.contains(&(*character as u8)))
        .take(VOICE_TITLE_MAX_CHARS)
        .collect::<String>();
    value = value.trim().to_string();
    if value.is_empty() {
        "VOICE NOTE".into()
    } else {
        value
    }
}

#[must_use]
pub fn editable_voice_title(title: &str) -> Vec<char> {
    let mut chars = sanitize_voice_title(title).chars().collect::<Vec<_>>();
    chars.resize(VOICE_TITLE_MAX_CHARS, ' ');
    chars.truncate(VOICE_TITLE_MAX_CHARS);
    chars
}

pub fn cycle_voice_title_character(buffer: &mut [char], cursor: usize, forward: bool) {
    let Some(character) = buffer.get_mut(cursor) else {
        return;
    };
    let current = character.to_ascii_uppercase() as u8;
    let position = TITLE_ALPHABET
        .iter()
        .position(|candidate| *candidate == current)
        .unwrap_or(0);
    let next = if forward {
        (position + 1) % TITLE_ALPHABET.len()
    } else {
        position.checked_sub(1).unwrap_or(TITLE_ALPHABET.len() - 1)
    };
    *character = TITLE_ALPHABET[next] as char;
}

#[must_use]
pub fn title_from_editable(buffer: &[char]) -> String {
    sanitize_voice_title(&buffer.iter().collect::<String>())
}

#[must_use]
pub fn format_storage_bytes(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return "--".into();
    };
    if bytes >= 1024 * 1024 * 1024 {
        format!("{} GB FREE", bytes / (1024 * 1024 * 1024))
    } else if bytes >= 1024 * 1024 {
        format!("{} MB FREE", bytes / (1024 * 1024))
    } else {
        format!("{} KB FREE", bytes / 1024)
    }
}

fn sanitize_recorded_at(recorded_at: &str) -> String {
    let value = recorded_at
        .chars()
        .filter(|character| character.is_ascii_digit() || matches!(character, '-' | ':' | ' '))
        .take(22)
        .collect::<String>();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        VOICE_UNKNOWN_RECORDED_AT.into()
    } else {
        trimmed.into()
    }
}

fn write_voice_note_metadata(root: &Path, metadata: &[VoiceNoteMetadata]) -> Result<()> {
    fs::create_dir_all(root)
        .with_context(|| format!("create voice-note root {}", root.display()))?;
    let mut sorted = metadata.to_vec();
    sorted.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    let mut text = String::new();
    for entry in sorted {
        text.push_str(&format!(
            "{}|{}|{}\n",
            entry.file_name,
            sanitize_recorded_at(&entry.recorded_at),
            sanitize_voice_title(&entry.title)
        ));
    }
    atomic_replace_text(
        root,
        VOICE_NOTES_METADATA_TMP_FILE,
        VOICE_NOTES_METADATA_FILE,
        &text,
    )
}

fn read_sidecar_text(root: &Path, final_name: &str) -> Result<Option<String>> {
    let final_path = root.join(final_name);
    let backup = final_path.with_extension("BAK");
    let path = if final_path.exists() {
        final_path
    } else if backup.exists() {
        backup
    } else {
        return Ok(None);
    };
    fs::read_to_string(&path)
        .with_context(|| format!("read voice-note sidecar {}", path.display()))
        .map(Some)
}

/// Recovery-safe sidecar replacement. The prior primary remains as `.BAK`
/// until the synchronized `.TMP` file is committed, matching the accepted
/// Reader persistence pattern and retaining a startup fallback after reset.
fn atomic_replace_text(root: &Path, temp_name: &str, final_name: &str, text: &str) -> Result<()> {
    fs::create_dir_all(root)
        .with_context(|| format!("create voice-note root {}", root.display()))?;
    let temp = root.join(temp_name);
    let final_path = root.join(final_name);
    let backup = final_path.with_extension("BAK");
    let _ = fs::remove_file(&temp);
    if !final_path.exists() && backup.exists() {
        fs::rename(&backup, &final_path)
            .with_context(|| format!("restore voice-note sidecar {}", final_path.display()))?;
    }
    {
        let mut file = File::create(&temp)
            .with_context(|| format!("create voice-note sidecar {}", temp.display()))?;
        file.write_all(text.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
    }
    let _ = fs::remove_file(&backup);
    if final_path.exists() {
        fs::rename(&final_path, &backup)
            .with_context(|| format!("backup voice-note sidecar {}", final_path.display()))?;
    }
    if let Err(error) = fs::rename(&temp, &final_path) {
        if backup.exists() {
            let _ = fs::rename(&backup, &final_path);
        }
        return Err(error)
            .with_context(|| format!("commit voice-note sidecar {}", final_path.display()));
    }
    let _ = fs::remove_file(&backup);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn temporary_root(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustmix-wave-{label}-{unique}"))
    }

    #[test]
    fn persists_selected_microphone_gain_in_fat83_settings_sidecar() {
        let root = temporary_root("voice-settings");
        save_voice_notes_preferences(
            &root,
            VoiceNotesPreferences {
                mic_gain: VoiceMicGain::Boost,
            },
        )
        .unwrap();
        assert_eq!(
            load_voice_notes_preferences(&root).unwrap().mic_gain,
            VoiceMicGain::Boost
        );
        assert!(root.join("SETTINGS.TXT").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn metadata_sidecar_keeps_fat83_wav_filename_and_friendly_title() {
        let root = temporary_root("voice-metadata");
        upsert_voice_note_metadata(
            &root,
            VoiceNoteMetadata {
                file_name: "VOICE001.WAV".into(),
                recorded_at: "2026-06-05  21:37:08".into(),
                title: "PROJECT IDEA".into(),
            },
        )
        .unwrap();
        let entries = load_voice_note_metadata(&root).unwrap();
        assert_eq!(entries[0].file_name, "VOICE001.WAV");
        assert_eq!(entries[0].recorded_at, "2026-06-05  21:37:08");
        assert_eq!(entries[0].title, "PROJECT IDEA");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rotary_title_buffer_cycles_and_trims_spaces() {
        let mut buffer = editable_voice_title("A");
        cycle_voice_title_character(&mut buffer, 0, true);
        cycle_voice_title_character(&mut buffer, 1, true);
        assert_eq!(title_from_editable(&buffer), "BA");
    }

    #[test]
    fn settings_loader_accepts_backup_after_interrupted_replace() {
        let root = temporary_root("voice-settings-backup");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("SETTINGS.BAK"), b"MIC_GAIN=normal\n").unwrap();
        assert_eq!(
            load_voice_notes_preferences(&root).unwrap().mic_gain,
            VoiceMicGain::Normal
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn formats_available_sd_capacity_compactly() {
        assert_eq!(format_storage_bytes(Some(8 * 1024 * 1024)), "8 MB FREE");
        assert_eq!(format_storage_bytes(None), "--");
    }
}
