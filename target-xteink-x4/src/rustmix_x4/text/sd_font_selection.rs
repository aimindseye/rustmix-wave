//! SD font selection support for Rustmix X4 reader.
//!
//! This module resolves 8.3-safe physical font files from
//! `/RUSTMIX/FONTS/MANIFEST.TXT`.  Files use the physical `.VFN` extension for
//! FAT/Wi-Fi Transfer compatibility, while the file payload still carries the
//! existing `VFNT`/`VFN1` magic header.

pub const SD_FONT_SELECTION_MARKER: &str = "reader-fonts=x4-reader-sd-font-selection-ok";
pub const SD_FONT_MANIFEST_PATH: &str = "RUSTMIX/FONTS";
pub const SD_FONT_MANIFEST_FILE: &str = "MANIFEST.TXT";
pub const SD_UI_FONT_MANIFEST_FILE: &str = "UIFONTS.TXT";
pub const SD_FONT_MAX_MANIFEST_BYTES: usize = 4096;
pub const SD_FONT_ID_CAP: usize = 8;
pub const SD_FONT_FILE_CAP: usize = 12;
pub const SD_FONT_SLOT_COUNT: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SdFontSource {
    BuiltIn,
    SdSlot(u8),
}

impl SdFontSource {
    pub const fn from_cycle(value: u8) -> Self {
        if value == 0 {
            Self::BuiltIn
        } else {
            let slot = value.saturating_sub(1);
            if slot >= SD_FONT_SLOT_COUNT {
                Self::SdSlot(SD_FONT_SLOT_COUNT - 1)
            } else {
                Self::SdSlot(slot)
            }
        }
    }

    pub const fn cycle_value(self) -> u8 {
        match self {
            Self::BuiltIn => 0,
            Self::SdSlot(slot) => slot.saturating_add(1),
        }
    }

    pub const fn slot_index(self) -> u8 {
        match self {
            Self::BuiltIn => 0,
            Self::SdSlot(slot) => slot,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SdFontSelection {
    pub source: SdFontSource,
    pub id: [u8; SD_FONT_ID_CAP],
    pub id_len: u8,
    pub file: [u8; SD_FONT_FILE_CAP],
    pub file_len: u8,
    pub size_bytes: u32,
    pub crc32: u32,
}

impl SdFontSelection {
    pub const fn builtin() -> Self {
        Self {
            source: SdFontSource::BuiltIn,
            id: [0; SD_FONT_ID_CAP],
            id_len: 0,
            file: [0; SD_FONT_FILE_CAP],
            file_len: 0,
            size_bytes: 0,
            crc32: 0,
        }
    }

    pub fn id_str(&self) -> &str {
        core::str::from_utf8(&self.id[..self.id_len as usize]).unwrap_or("")
    }

    pub fn file_str(&self) -> &str {
        core::str::from_utf8(&self.file[..self.file_len as usize]).unwrap_or("")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SdFontSelectionError {
    ManifestMissing,
    SlotMissing,
    InvalidRow,
    MissingVfnt,
    InvalidVfnt,
}

pub fn selected_font_from_manifest(
    manifest: &str,
    requested: SdFontSource,
) -> Result<SdFontSelection, SdFontSelectionError> {
    let SdFontSource::SdSlot(slot) = requested else {
        return Ok(SdFontSelection::builtin());
    };

    let mut seen: u8 = 0;
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || !line.starts_with("FONT|") {
            continue;
        }
        if seen == slot {
            return parse_font_row(line, requested);
        }
        seen = seen.saturating_add(1);
    }

    Err(SdFontSelectionError::SlotMissing)
}

fn parse_font_row(
    line: &str,
    source: SdFontSource,
) -> Result<SdFontSelection, SdFontSelectionError> {
    let mut parts = line.split('|');
    if parts.next() != Some("FONT") {
        return Err(SdFontSelectionError::InvalidRow);
    }

    let id = parts.next().unwrap_or("").trim();
    let _display = parts.next().unwrap_or("").trim();
    let _script = parts.next().unwrap_or("").trim();
    let _style = parts.next().unwrap_or("").trim();
    let _px = parts.next().unwrap_or("").trim();
    let file = parts.next().unwrap_or("").trim();
    let size_bytes = parts
        .next()
        .unwrap_or("0")
        .trim()
        .parse::<u32>()
        .unwrap_or(0);
    let crc32 = parse_crc32(parts.next().unwrap_or("0").trim());

    if !safe_id(id) || !safe_vfn_name(file) || size_bytes == 0 {
        return Err(SdFontSelectionError::InvalidRow);
    }

    let mut selected = SdFontSelection::builtin();
    selected.source = source;
    selected.id_len = copy_ascii_upper(id.as_bytes(), &mut selected.id);
    selected.file_len = copy_ascii_upper(file.as_bytes(), &mut selected.file);
    selected.size_bytes = size_bytes;
    selected.crc32 = crc32;
    Ok(selected)
}

pub fn vfnt_header_looks_valid(data: &[u8]) -> bool {
    data.len() >= 4 && (&data[..4] == b"VFNT" || &data[..4] == b"VFN1")
}

fn copy_ascii_upper(src: &[u8], dst: &mut [u8]) -> u8 {
    let mut n = 0usize;
    while n < dst.len() && n < src.len() {
        dst[n] = src[n].to_ascii_uppercase();
        n += 1;
    }
    n as u8
}

fn parse_crc32(value: &str) -> u32 {
    let mut out = 0u32;
    for b in value.bytes() {
        let nibble = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => return 0,
        };
        out = (out << 4) | u32::from(nibble);
    }
    out
}

fn safe_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= SD_FONT_ID_CAP
        && bytes
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_')
}

fn safe_vfn_name(value: &str) -> bool {
    let Some((stem, ext)) = value.rsplit_once('.') else {
        return false;
    };
    ext.eq_ignore_ascii_case("VFN") && safe_id(stem) && value.len() <= SD_FONT_FILE_CAP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_values_map_to_three_sd_slots() {
        assert_eq!(SdFontSource::from_cycle(0), SdFontSource::BuiltIn);
        assert_eq!(SdFontSource::from_cycle(1), SdFontSource::SdSlot(0));
        assert_eq!(SdFontSource::from_cycle(2), SdFontSource::SdSlot(1));
        assert_eq!(SdFontSource::from_cycle(3), SdFontSource::SdSlot(2));
        assert_eq!(SdFontSource::from_cycle(4), SdFontSource::SdSlot(2));
    }

    #[test]
    fn manifest_uses_83_safe_vfn_files() {
        let manifest = "FONT|CHARIS|Charis|Latin|Regular|18|CHARIS18.VFN|1234|00000000|96\n";
        let selected = selected_font_from_manifest(manifest, SdFontSource::SdSlot(0)).unwrap();
        assert_eq!(selected.id_str(), "CHARIS");
        assert_eq!(selected.file_str(), "CHARIS18.VFN");
    }
}
