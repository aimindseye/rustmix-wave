use core::fmt::Write as _;
use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

pub const READER_STATE_DIR: &str = "state";
pub const READER_PROGRESS_EXT: &str = ".PRG";
pub const READER_BOOKMARK_EXT: &str = ".BKM";
pub const READER_BOOKMARK_INDEX_FILE: &str = "BMIDX.TXT";
pub const READER_BOOKMARK_JUMP_PREFIX: &str = "BMJ";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReaderStateFileFormatModel {
    Txt,
    Epub,
    #[default]
    Unknown,
}

impl ReaderStateFileFormatModel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Epub => "epub",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "txt" => Self::Txt,
            "epub" => Self::Epub,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderProgressRecordModel {
    pub book_id: String<32>,
    pub source_path: String<128>,
    pub format: ReaderStateFileFormatModel,
    pub chapter: u16,
    pub page: u32,
    pub byte_offset: u32,
    pub font_size_idx: u8,
}

impl ReaderProgressRecordModel {
    pub fn new(
        book_id: &str,
        source_path: &str,
        format: ReaderStateFileFormatModel,
        chapter: u16,
        page: u32,
        byte_offset: u32,
        font_size_idx: u8,
    ) -> Option<Self> {
        Some(Self {
            book_id: copy_str(book_id)?,
            source_path: copy_str(source_path)?,
            format,
            chapter,
            page,
            byte_offset,
            font_size_idx,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkEntryModel {
    pub book_id: String<32>,
    pub source_path: String<128>,
    pub chapter: u16,
    pub byte_offset: u32,
    pub label: String<96>,
}

impl BookmarkEntryModel {
    pub fn new(
        book_id: &str,
        source_path: &str,
        chapter: u16,
        byte_offset: u32,
        label: &str,
    ) -> Option<Self> {
        Some(Self {
            book_id: copy_str(book_id)?,
            source_path: copy_str(source_path)?,
            chapter,
            byte_offset,
            label: copy_str(label)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkIndexEntryModel {
    pub book_id: String<32>,
    pub source_path: String<128>,
    pub display_title: String<96>,
    pub chapter: u16,
    pub byte_offset: u32,
    pub label: String<96>,
}

impl BookmarkIndexEntryModel {
    pub fn new(
        book_id: &str,
        source_path: &str,
        display_title: &str,
        chapter: u16,
        byte_offset: u32,
        label: &str,
    ) -> Option<Self> {
        Some(Self {
            book_id: copy_str(book_id)?,
            source_path: copy_str(source_path)?,
            display_title: copy_str(display_title)?,
            chapter,
            byte_offset,
            label: copy_str(label)?,
        })
    }

    pub fn from_bookmark(entry: &BookmarkEntryModel, display_title: &str) -> Option<Self> {
        Self::new(
            entry.book_id.as_str(),
            entry.source_path.as_str(),
            display_title,
            entry.chapter,
            entry.byte_offset,
            entry.label.as_str(),
        )
    }
}

pub fn compat_book_id_hex8(book_id: &str) -> String<8> {
    let raw = book_id.strip_prefix("bk-").unwrap_or(book_id);
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_hexdigit() {
            let _ = out.push(ch.to_ascii_uppercase());
            if out.len() >= 8 {
                break;
            }
        }
    }
    while out.len() < 8 {
        let _ = out.push('0');
    }
    out
}

pub fn progress_record_file_for(book_id: &str) -> String<12> {
    typed_state_file_for(book_id, READER_PROGRESS_EXT)
}

pub fn bookmark_record_file_for(book_id: &str) -> String<12> {
    typed_state_file_for(book_id, READER_BOOKMARK_EXT)
}

pub fn progress_record_path_for(book_id: &str) -> String<24> {
    state_path_for(book_id, READER_PROGRESS_EXT)
}

pub fn bookmark_record_path_for(book_id: &str) -> String<24> {
    state_path_for(book_id, READER_BOOKMARK_EXT)
}

pub fn bookmark_index_path() -> String<24> {
    let mut out = String::new();
    let _ = write!(out, "{}/{}", READER_STATE_DIR, READER_BOOKMARK_INDEX_FILE);
    out
}

pub fn parse_progress_record(line: &str) -> Option<ReaderProgressRecordModel> {
    let fields = split_fields::<7>(line);
    if fields.len() != 7 {
        return None;
    }
    let font_size_idx = fields[6].parse::<u16>().ok()? as u8;
    ReaderProgressRecordModel::new(
        fields[0].as_str(),
        fields[1].as_str(),
        ReaderStateFileFormatModel::parse(fields[2].as_str()),
        fields[3].parse().ok()?,
        fields[4].parse().ok()?,
        fields[5].parse().ok()?,
        font_size_idx,
    )
}

pub fn write_progress_record(record: &ReaderProgressRecordModel) -> Option<String<256>> {
    let mut out = String::new();
    push_field(&mut out, record.book_id.as_str())?;
    push_field(&mut out, record.source_path.as_str())?;
    push_field(&mut out, record.format.as_str())?;
    push_u32_field(&mut out, u32::from(record.chapter))?;
    push_u32_field(&mut out, record.page)?;
    push_u32_field(&mut out, record.byte_offset)?;
    push_u32_field(&mut out, u32::from(record.font_size_idx))?;
    Some(out)
}

pub fn parse_bookmark_record(line: &str) -> Option<BookmarkEntryModel> {
    let fields = split_fields::<5>(line);
    if fields.len() != 5 {
        return None;
    }
    BookmarkEntryModel::new(
        fields[0].as_str(),
        fields[1].as_str(),
        fields[2].parse().ok()?,
        fields[3].parse().ok()?,
        fields[4].as_str(),
    )
}

pub fn write_bookmark_record(record: &BookmarkEntryModel) -> Option<String<256>> {
    let mut out = String::new();
    push_field(&mut out, record.book_id.as_str())?;
    push_field(&mut out, record.source_path.as_str())?;
    push_u32_field(&mut out, u32::from(record.chapter))?;
    push_u32_field(&mut out, record.byte_offset)?;
    push_field(&mut out, record.label.as_str())?;
    Some(out)
}

pub fn parse_bookmark_records<const N: usize>(payload: &str) -> Vec<BookmarkEntryModel, N> {
    let mut out = Vec::new();
    for line in payload.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(record) = parse_bookmark_record(line) {
            let _ = out.push(record);
        }
    }
    out
}

pub fn write_bookmark_records<const OUT: usize>(
    records: &[BookmarkEntryModel],
) -> Option<String<OUT>> {
    let mut out = String::new();
    for (idx, record) in records.iter().enumerate() {
        if idx > 0 {
            out.push('\n').ok()?;
        }
        out.push_str(write_bookmark_record(record)?.as_str()).ok()?;
    }
    Some(out)
}

pub fn parse_bookmark_index_entry(line: &str) -> Option<BookmarkIndexEntryModel> {
    let fields = split_fields::<6>(line);
    if fields.len() < 6 {
        return None;
    }
    BookmarkIndexEntryModel::new(
        fields[0].as_str(),
        fields[1].as_str(),
        fields[2].as_str(),
        fields[3].parse().ok()?,
        fields[4].parse().ok()?,
        fields[5].as_str(),
    )
}

pub fn write_bookmark_index_entry(record: &BookmarkIndexEntryModel) -> Option<String<320>> {
    let mut out = String::new();
    push_field(&mut out, record.book_id.as_str())?;
    push_field(&mut out, record.source_path.as_str())?;
    push_field(&mut out, record.display_title.as_str())?;
    push_u32_field(&mut out, u32::from(record.chapter))?;
    push_u32_field(&mut out, record.byte_offset)?;
    push_field(&mut out, record.label.as_str())?;
    Some(out)
}

pub fn parse_bookmark_index<const N: usize>(payload: &str) -> Vec<BookmarkIndexEntryModel, N> {
    let mut out = Vec::new();
    for line in payload.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(record) = parse_bookmark_index_entry(line) {
            let _ = out.push(record);
        }
    }
    out
}

pub fn write_bookmark_index<const OUT: usize>(
    records: &[BookmarkIndexEntryModel],
) -> Option<String<OUT>> {
    let mut out = String::new();
    for (idx, record) in records.iter().enumerate() {
        if idx > 0 {
            out.push('\n').ok()?;
        }
        out.push_str(write_bookmark_index_entry(record)?.as_str())
            .ok()?;
    }
    Some(out)
}

pub fn bookmark_jump_message(record: &BookmarkIndexEntryModel) -> Option<String<192>> {
    let mut out = String::new();
    push_field(&mut out, READER_BOOKMARK_JUMP_PREFIX)?;
    push_field(&mut out, record.source_path.as_str())?;
    push_u32_field(&mut out, u32::from(record.chapter))?;
    push_u32_field(&mut out, record.byte_offset)?;
    Some(out)
}

pub fn parse_bookmark_jump_message(msg: &str) -> Option<(String<128>, u16, u32)> {
    let fields = split_fields::<4>(msg);
    if fields.len() != 4 || fields[0].as_str() != READER_BOOKMARK_JUMP_PREFIX {
        return None;
    }
    Some((
        copy_str(fields[1].as_str())?,
        fields[2].parse().ok()?,
        fields[3].parse().ok()?,
    ))
}

fn typed_state_file_for(book_id: &str, ext: &str) -> String<12> {
    let mut out: String<12> = String::new();
    let _ = out.push_str(compat_book_id_hex8(book_id).as_str());
    let _ = out.push_str(ext);
    out
}

fn state_path_for(book_id: &str, ext: &str) -> String<24> {
    let mut out: String<24> = String::new();
    let _ = out.push_str(READER_STATE_DIR);
    let _ = out.push('/');
    let _ = out.push_str(typed_state_file_for(book_id, ext).as_str());
    out
}

fn copy_str<const N: usize>(value: &str) -> Option<String<N>> {
    let mut out = String::new();
    out.push_str(value).ok()?;
    Some(out)
}

fn push_field<const N: usize>(out: &mut String<N>, field: &str) -> Option<()> {
    if !out.is_empty() {
        out.push('|').ok()?;
    }
    for ch in field.chars() {
        match ch {
            '|' => out.push_str("%7C").ok()?,
            '\n' => out.push_str("%0A").ok()?,
            '\r' => out.push_str("%0D").ok()?,
            _ => out.push(ch).ok()?,
        }
    }
    Some(())
}

fn push_u32_field<const N: usize>(out: &mut String<N>, value: u32) -> Option<()> {
    let mut digits = [0u8; 10];
    let mut n = value;
    let mut len = 0usize;

    if n == 0 {
        digits[0] = b'0';
        len = 1;
    } else {
        while n > 0 {
            digits[len] = b'0' + (n % 10) as u8;
            n /= 10;
            len += 1;
        }
    }

    let mut buf: String<10> = String::new();
    while len > 0 {
        len -= 1;
        buf.push(digits[len] as char).ok()?;
    }

    push_field(out, buf.as_str())
}

fn split_fields<const N: usize>(line: &str) -> Vec<String<160>, N> {
    let mut fields = Vec::new();
    for raw in line.split('|') {
        if let Some(decoded) = percent_decode::<160>(raw) {
            let _ = fields.push(decoded);
        }
    }
    fields
}

fn percent_decode<const N: usize>(value: &str) -> Option<String<N>> {
    let bytes = value.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if i + 2 < bytes.len() && bytes[i] == b'%' {
            match &bytes[i + 1..i + 3] {
                b"7C" => {
                    out.push('|').ok()?;
                    i += 3;
                    continue;
                }
                b"0A" => {
                    out.push('\n').ok()?;
                    i += 3;
                    continue;
                }
                b"0D" => {
                    out.push('\r').ok()?;
                    i += 3;
                    continue;
                }
                _ => {}
            }
        }
        out.push(bytes[i] as char).ok()?;
        i += 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOK_ID: &str = "bk-8a79a61f";
    const SOURCE: &str = "BOOKS/YEARLY_H.TXT";

    #[test]
    fn progress_file_paths_preserve_current_8_3_layout() {
        assert_eq!(progress_record_file_for(BOOK_ID).as_str(), "8A79A61F.PRG");
        assert_eq!(bookmark_record_file_for(BOOK_ID).as_str(), "8A79A61F.BKM");
        assert_eq!(
            progress_record_path_for(BOOK_ID).as_str(),
            "state/8A79A61F.PRG"
        );
        assert_eq!(
            bookmark_record_path_for(BOOK_ID).as_str(),
            "state/8A79A61F.BKM"
        );
        assert_eq!(bookmark_index_path().as_str(), "state/BMIDX.TXT");
    }

    #[test]
    fn progress_record_round_trips_current_pipe_format() {
        let record = ReaderProgressRecordModel::new(
            BOOK_ID,
            SOURCE,
            ReaderStateFileFormatModel::Txt,
            0,
            12,
            3456,
            4,
        )
        .unwrap();

        let line = write_progress_record(&record).unwrap();
        assert_eq!(
            line.as_str(),
            "bk-8a79a61f|BOOKS/YEARLY_H.TXT|txt|0|12|3456|4"
        );

        let parsed = parse_progress_record(line.as_str()).unwrap();
        assert_eq!(parsed, record);
    }

    #[test]
    fn bookmark_record_round_trips_with_escaped_label() {
        let record = BookmarkEntryModel::new(BOOK_ID, SOURCE, 2, 8192, "line 1 | note").unwrap();
        let line = write_bookmark_record(&record).unwrap();
        assert_eq!(
            line.as_str(),
            "bk-8a79a61f|BOOKS/YEARLY_H.TXT|2|8192|line 1 %7C note"
        );

        let parsed = parse_bookmark_record(line.as_str()).unwrap();
        assert_eq!(parsed, record);
    }

    #[test]
    fn bookmark_list_skips_empty_lines() {
        let payload = "\nbk-8a79a61f|BOOKS/YEARLY_H.TXT|1|200|A\n\n";
        let entries = parse_bookmark_records::<4>(payload);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].byte_offset, 200);
    }

    #[test]
    fn bookmark_index_round_trips_current_format() {
        let entry =
            BookmarkIndexEntryModel::new(BOOK_ID, SOURCE, "Yearly Hindi", 3, 9000, "important")
                .unwrap();
        let line = write_bookmark_index_entry(&entry).unwrap();
        assert_eq!(
            line.as_str(),
            "bk-8a79a61f|BOOKS/YEARLY_H.TXT|Yearly Hindi|3|9000|important"
        );

        let parsed = parse_bookmark_index_entry(line.as_str()).unwrap();
        assert_eq!(parsed, entry);
    }

    #[test]
    fn bookmark_index_payload_round_trips_multiple_entries() {
        let a = BookmarkIndexEntryModel::new(BOOK_ID, SOURCE, "Yearly Hindi", 0, 100, "A").unwrap();
        let b = BookmarkIndexEntryModel::new(BOOK_ID, SOURCE, "Yearly Hindi", 1, 200, "B").unwrap();
        let records = [a.clone(), b.clone()];
        let payload = write_bookmark_index::<512>(&records).unwrap();
        let parsed = parse_bookmark_index::<4>(payload.as_str());
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0], a);
        assert_eq!(parsed[1], b);
    }

    #[test]
    fn bookmark_jump_message_matches_current_prefix_contract() {
        let entry =
            BookmarkIndexEntryModel::new(BOOK_ID, SOURCE, "Yearly Hindi", 7, 777, "jump").unwrap();
        let msg = bookmark_jump_message(&entry).unwrap();
        assert_eq!(msg.as_str(), "BMJ|BOOKS/YEARLY_H.TXT|7|777");

        let (path, chapter, offset) = parse_bookmark_jump_message(msg.as_str()).unwrap();
        assert_eq!(path.as_str(), SOURCE);
        assert_eq!(chapter, 7);
        assert_eq!(offset, 777);
    }
}
