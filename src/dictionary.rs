//! Native offline Dictionary engine compatible with the Rustmix X4 SD pack.
//!
//! The X4 pack remains authoritative on removable storage:
//! `/sdcard/RUSTMIX/APPS/DICT/INDEX.TXT` selects one bounded
//! `DATA/*.JSN` prefix shard.  The Waveshare port keeps that storage contract
//! but renders the rotary-first UI natively in Rust.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::{buttons::ButtonEvent, keyboard_navigation::KeyboardGridNavigation};

/// Rustmix X4-compatible dictionary app root.
pub const DICTIONARY_ROOT: &str = "/sdcard/RUSTMIX/APPS/DICT";
/// X4 pack shard index filename.
pub const DICTIONARY_INDEX_FILE: &str = "INDEX.TXT";
/// Prefix-shard files stay intentionally small for bounded SD reads.
pub const DICTIONARY_SHARD_MAX_BYTES: usize = 16 * 1024;
/// Search text remains bounded for the rotary keyboard.
pub const DICTIONARY_QUERY_MAX_CHARS: usize = 32;
/// Prefix mode retains only a compact page of matches.
pub const DICTIONARY_MATCH_LIMIT: usize = 8;
/// X4-style keyboard rows retained for pack compatibility and predictable UI.
pub const DICTIONARY_KEY_ROWS: [[&str; 6]; 5] = [
    ["A", "B", "C", "D", "E", "F"],
    ["G", "H", "I", "J", "K", "L"],
    ["M", "N", "O", "P", "Q", "R"],
    ["S", "T", "U", "V", "W", "X"],
    ["Y", "Z", "DEL", "CLR", "GO", "*"],
];
const DICTIONARY_KEY_COLUMNS: usize = 6;
const DICTIONARY_KEY_COUNT: usize = 30;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DictionaryIndexRow {
    pub name: String,
    pub relative_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DictionaryMatch {
    pub word: String,
    pub definition: String,
    pub shard: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DictionaryLookup {
    pub matches: Vec<DictionaryMatch>,
    pub prefix_mode: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DictionaryUiState {
    pub query: String,
    pub keyboard_navigation: KeyboardGridNavigation,
    pub matches: Vec<DictionaryMatch>,
    pub selected_match: usize,
    pub wildcard: bool,
    pub message: String,
    pub pack_ready: bool,
    pub shard_count: usize,
}

impl Default for DictionaryUiState {
    fn default() -> Self {
        Self {
            query: String::new(),
            keyboard_navigation: KeyboardGridNavigation::new(
                DICTIONARY_KEY_COUNT,
                DICTIONARY_KEY_COLUMNS,
            ),
            matches: Vec::new(),
            selected_match: 0,
            wildcard: false,
            message: "Type a word. * does prefix search.".into(),
            pack_ready: false,
            shard_count: 0,
        }
    }
}

impl DictionaryUiState {
    pub fn refresh_pack_status(&mut self) {
        match load_dictionary_index(Path::new(DICTIONARY_ROOT)) {
            Ok(rows) => {
                self.pack_ready = true;
                self.shard_count = rows.len();
                self.message = format!("Pack ready: {} indexed shards", rows.len());
            }
            Err(error) => {
                self.pack_ready = false;
                self.shard_count = 0;
                self.message = compact_error(&error.to_string());
            }
        }
    }

    #[must_use]
    pub fn selected_key_label(&self) -> &'static str {
        flat_key(self.keyboard_navigation.selected())
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

    #[must_use]
    pub fn current_match(&self) -> Option<&DictionaryMatch> {
        self.matches.get(
            self.selected_match
                .min(self.matches.len().saturating_sub(1)),
        )
    }

    #[must_use]
    pub fn match_label(&self) -> String {
        if self.matches.is_empty() {
            "NO RESULT".into()
        } else {
            format!("{} / {}", self.selected_match + 1, self.matches.len())
        }
    }

    pub fn apply_button(&mut self, event: ButtonEvent) {
        match event {
            ButtonEvent::Up => self.keyboard_navigation.move_previous(),
            ButtonEvent::Down => self.keyboard_navigation.move_next(),
            ButtonEvent::Select => self.apply_selected_key(),
        }
    }

    fn apply_selected_key(&mut self) {
        match self.selected_key_label() {
            "DEL" => {
                self.query.pop();
                self.clear_results("Deleted last character");
            }
            "CLR" => {
                self.query.clear();
                self.clear_results("Cleared search");
            }
            "GO" => self.run_lookup(false),
            "*" => self.run_lookup(true),
            letter => {
                if self.query.chars().count() < DICTIONARY_QUERY_MAX_CHARS {
                    self.query.push_str(letter);
                    self.clear_results("Type a word. GO lookup, * prefix.");
                }
            }
        }
    }

    fn clear_results(&mut self, message: &str) {
        self.matches.clear();
        self.selected_match = 0;
        self.wildcard = false;
        self.message = message.into();
    }

    fn run_lookup(&mut self, prefix_mode: bool) {
        let normalized = normalize_query(&self.query);
        if prefix_mode
            && self.wildcard
            && !self.matches.is_empty()
            && normalized == normalize_query(&self.query)
        {
            self.selected_match = (self.selected_match + 1) % self.matches.len();
            self.message = format!(
                "Prefix result {} of {}. Press * for next.",
                self.selected_match + 1,
                self.matches.len()
            );
            return;
        }
        match lookup_dictionary(Path::new(DICTIONARY_ROOT), &normalized, prefix_mode) {
            Ok(result) => {
                self.matches = result.matches;
                self.selected_match = 0;
                self.wildcard = result.prefix_mode;
                self.message = if self.matches.is_empty() {
                    "Word not found".into()
                } else if self.wildcard {
                    format!("Prefix results: {}. Press * for next.", self.matches.len())
                } else {
                    "Exact match".into()
                };
            }
            Err(error) => {
                self.matches.clear();
                self.selected_match = 0;
                self.wildcard = prefix_mode;
                self.message = compact_error(&error.to_string());
            }
        }
    }
}

#[must_use]
pub const fn flat_key(index: usize) -> &'static str {
    DICTIONARY_KEY_ROWS[index / 6][index % 6]
}

pub fn load_dictionary_index(root: &Path) -> Result<Vec<DictionaryIndexRow>> {
    let path = root.join(DICTIONARY_INDEX_FILE);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing dictionary INDEX.TXT at {}", path.display()))?;
    parse_dictionary_index(&text)
}

pub fn parse_dictionary_index(text: &str) -> Result<Vec<DictionaryIndexRow>> {
    let mut rows = Vec::new();
    for (line_number, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, relative_path) = line
            .split_once('|')
            .ok_or_else(|| anyhow::anyhow!("bad dictionary index line {}", line_number + 1))?;
        let name = name.trim().to_ascii_uppercase();
        let relative_path = relative_path.trim().replace('\\', "/");
        if name.is_empty() || relative_path.is_empty() {
            bail!("bad dictionary index line {}", line_number + 1);
        }
        validate_relative_shard_path(&relative_path)?;
        rows.push(DictionaryIndexRow {
            name,
            relative_path,
        });
    }
    if rows.is_empty() {
        bail!("empty dictionary INDEX.TXT");
    }
    Ok(rows)
}

fn validate_relative_shard_path(path: &str) -> Result<()> {
    if path.starts_with('/') || path.contains("..") || !path.ends_with(".JSN") {
        bail!("unsafe dictionary shard path {path:?}");
    }
    let mut segments = path.split('/');
    if segments.next() != Some("DATA") || segments.next().is_none() || segments.next().is_some() {
        bail!("dictionary shard must live under DATA/: {path:?}");
    }
    Ok(())
}

pub fn lookup_dictionary(root: &Path, query: &str, prefix_mode: bool) -> Result<DictionaryLookup> {
    let query = normalize_query(query);
    if query.is_empty() {
        bail!("type a word first");
    }
    let rows = load_dictionary_index(root)?;
    lookup_dictionary_with_index(root, &rows, &query, prefix_mode)
}

pub fn lookup_dictionary_with_index(
    root: &Path,
    rows: &[DictionaryIndexRow],
    query: &str,
    prefix_mode: bool,
) -> Result<DictionaryLookup> {
    let query = normalize_query(query);
    if query.is_empty() {
        bail!("type a word first");
    }
    let candidates = candidate_rows(rows, &query);
    if candidates.is_empty() {
        bail!("no shard in INDEX.TXT for {}", alpha_prefix(&query));
    }

    if !prefix_mode {
        for row in &candidates {
            let shard = read_bounded_shard(root, row)?;
            if let Some(entry) = extract_shard_matches(&shard, &query, false, 1)?
                .into_iter()
                .next()
            {
                return Ok(DictionaryLookup {
                    matches: vec![DictionaryMatch {
                        word: entry.0,
                        definition: entry.1,
                        shard: row.name.clone(),
                    }],
                    prefix_mode: false,
                });
            }
        }
    }

    let mut matches = Vec::new();
    for row in &candidates {
        let shard = read_bounded_shard(root, row)?;
        for (word, definition) in
            extract_shard_matches(&shard, &query, true, DICTIONARY_MATCH_LIMIT - matches.len())?
        {
            if !matches
                .iter()
                .any(|item: &DictionaryMatch| item.word == word)
            {
                matches.push(DictionaryMatch {
                    word,
                    definition,
                    shard: row.name.clone(),
                });
            }
            if matches.len() >= DICTIONARY_MATCH_LIMIT {
                break;
            }
        }
        if matches.len() >= DICTIONARY_MATCH_LIMIT {
            break;
        }
    }
    Ok(DictionaryLookup {
        matches,
        prefix_mode: true,
    })
}

fn read_bounded_shard(root: &Path, row: &DictionaryIndexRow) -> Result<String> {
    let path = root.join(PathBuf::from(&row.relative_path));
    let metadata = fs::metadata(&path)
        .with_context(|| format!("dictionary pack incomplete: missing {}", row.name))?;
    if metadata.len() > DICTIONARY_SHARD_MAX_BYTES as u64 {
        bail!("{} shard too large: {} bytes", row.name, metadata.len());
    }
    fs::read_to_string(&path).with_context(|| format!("read dictionary shard {}", path.display()))
}

fn candidate_rows<'a>(rows: &'a [DictionaryIndexRow], query: &str) -> Vec<&'a DictionaryIndexRow> {
    let prefix = alpha_prefix(query);
    let mut candidates: Vec<_> = rows
        .iter()
        .filter(|row| row_matches(&row.name, &prefix))
        .collect();
    candidates.sort_by(|left, right| {
        let left_base = shard_base(&left.name);
        let right_base = shard_base(&right.name);
        right_base
            .len()
            .cmp(&left_base.len())
            .then_with(|| left.name.cmp(&right.name))
    });
    candidates
}

fn row_matches(row_name: &str, prefix: &str) -> bool {
    let base = shard_base(row_name);
    if prefix == "OTHERS" {
        base == "OTHERS"
    } else {
        base.starts_with(prefix) || prefix.starts_with(base)
    }
}

fn shard_base(name: &str) -> &str {
    name.trim_end_matches(|character: char| character.is_ascii_digit())
}

#[must_use]
pub fn normalize_query(query: &str) -> String {
    query
        .trim()
        .trim_end_matches(|character| matches!(character, '*' | '_'))
        .trim()
        .to_ascii_uppercase()
}

#[must_use]
pub fn alpha_prefix(query: &str) -> String {
    let normalized = normalize_query(query);
    let mut output = String::new();
    for character in normalized.chars().take(5) {
        if character.is_ascii_alphabetic() {
            output.push(character);
        } else {
            break;
        }
    }
    if output.is_empty() && !normalized.is_empty() {
        "OTHERS".into()
    } else {
        output
    }
}

fn extract_shard_matches(
    json: &str,
    query: &str,
    prefix_mode: bool,
    limit: usize,
) -> Result<Vec<(String, String)>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let bytes = json.as_bytes();
    let mut position = skip_ws(bytes, 0);
    if bytes.get(position) != Some(&b'{') {
        bail!("parse failed: shard JSON");
    }
    position += 1;
    let mut matches = Vec::new();
    loop {
        position = skip_ws(bytes, position);
        match bytes.get(position) {
            Some(b'}') => break,
            Some(b'"') => {}
            _ => bail!("parse failed: shard key"),
        }
        let key_end = json_string_end(bytes, position)?;
        let key = &json[position + 1..key_end];
        position = skip_ws(bytes, key_end + 1);
        if bytes.get(position) != Some(&b':') {
            bail!("parse failed: shard separator");
        }
        position = skip_ws(bytes, position + 1);
        let value_end = json_value_end(bytes, position)?;
        if key == query || (prefix_mode && key.starts_with(query)) {
            matches.push((key.into(), compact_definition(&json[position..value_end])));
            if matches.len() >= limit {
                break;
            }
        }
        position = skip_ws(bytes, value_end);
        match bytes.get(position) {
            Some(b',') => position += 1,
            Some(b'}') => break,
            _ => bail!("parse failed: shard delimiter"),
        }
    }
    Ok(matches)
}

fn skip_ws(bytes: &[u8], mut position: usize) -> usize {
    while bytes
        .get(position)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
    {
        position += 1;
    }
    position
}

fn json_string_end(bytes: &[u8], start: usize) -> Result<usize> {
    let mut escaped = false;
    for (offset, byte) in bytes.iter().enumerate().skip(start + 1) {
        if escaped {
            escaped = false;
        } else if *byte == b'\\' {
            escaped = true;
        } else if *byte == b'"' {
            return Ok(offset);
        }
    }
    bail!("parse failed: unterminated string")
}

fn json_value_end(bytes: &[u8], start: usize) -> Result<usize> {
    let first = *bytes
        .get(start)
        .ok_or_else(|| anyhow::anyhow!("parse failed: missing value"))?;
    if first == b'"' {
        return Ok(json_string_end(bytes, start)? + 1);
    }
    if first != b'{' && first != b'[' {
        let mut position = start;
        while !matches!(
            bytes.get(position),
            None | Some(b',') | Some(b'}') | Some(b']')
        ) {
            position += 1;
        }
        return Ok(position);
    }
    let mut object_depth = 0usize;
    let mut array_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut position = start;
    while let Some(byte) = bytes.get(position) {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
        } else {
            match *byte {
                b'"' => in_string = true,
                b'{' => object_depth += 1,
                b'}' => object_depth = object_depth.saturating_sub(1),
                b'[' => array_depth += 1,
                b']' => array_depth = array_depth.saturating_sub(1),
                _ => {}
            }
            if object_depth == 0 && array_depth == 0 {
                return Ok(position + 1);
            }
        }
        position += 1;
    }
    bail!("parse failed: unterminated value")
}

fn compact_definition(raw: &str) -> String {
    for field in ["def", "meaning", "definition", "text"] {
        if let Some(value) = extract_json_string_field(raw, field) {
            return compact_text(&value, 220);
        }
    }
    compact_text(raw, 220)
}

fn extract_json_string_field(raw: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\"");
    let start = raw.find(&needle)? + needle.len();
    let rest = &raw[start..];
    let separator = rest.find(':')?;
    let rest = rest[separator + 1..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let bytes = rest.as_bytes();
    let end = json_string_end(bytes, 0).ok()?;
    Some(unescape_json_string(&rest[1..end]))
}

fn unescape_json_string(raw: &str) -> String {
    raw.replace("\\\"", "\"")
        .replace("\\n", " ")
        .replace("\\r", " ")
        .replace("\\t", " ")
        .replace("\\\\", "\\")
}

fn compact_text(raw: &str, max_chars: usize) -> String {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max_chars {
        return collapsed;
    }
    let mut shortened: String = collapsed
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect();
    shortened.push_str("...");
    shortened
}

fn compact_error(message: &str) -> String {
    compact_text(message, 84)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        alpha_prefix, lookup_dictionary, normalize_query, parse_dictionary_index,
        DictionaryUiState, DICTIONARY_SHARD_MAX_BYTES,
    };
    use crate::buttons::ButtonEvent;

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rustmix-wave-dict-{label}-{nanos}"))
    }

    fn write_pack(root: &std::path::Path) {
        fs::create_dir_all(root.join("DATA")).unwrap();
        fs::write(root.join("INDEX.TXT"), "AA|DATA/AA.JSN\nAB|DATA/AB.JSN\n").unwrap();
        fs::write(root.join("DATA/AA.JSN"), r#"{"AAM":[{"def":"Liquid measure","pos":""}],"AARD-VARK":[{"def":"African mammal","pos":""}]}"#).unwrap();
        fs::write(
            root.join("DATA/AB.JSN"),
            r#"{"AB":[{"def":"Month name","pos":""}],"AB-":[{"def":"Latin prefix","pos":""}]}"#,
        )
        .unwrap();
    }

    #[test]
    fn parses_x4_index_and_rejects_unsafe_paths() {
        let rows = parse_dictionary_index("# pack\nAA|DATA/AA.JSN\nAB|DATA/AB.JSN\n").unwrap();
        assert_eq!(rows.len(), 2);
        assert!(parse_dictionary_index("AA|../AA.JSN\n").is_err());
        assert!(parse_dictionary_index("AA|AA.JSN\n").is_err());
    }

    #[test]
    fn normalizes_queries_and_selects_alpha_prefix() {
        assert_eq!(normalize_query("  aard*  "), "AARD");
        assert_eq!(alpha_prefix("aard-vark"), "AARD");
        assert_eq!(alpha_prefix("123"), "OTHERS");
    }

    #[test]
    fn exact_lookup_and_prefix_fallback_use_bounded_x4_shards() {
        let root = temp_root("lookup");
        write_pack(&root);
        let exact = lookup_dictionary(&root, "AB", false).unwrap();
        assert!(!exact.prefix_mode);
        assert_eq!(exact.matches[0].definition, "Month name");
        let fallback = lookup_dictionary(&root, "AAR", false).unwrap();
        assert!(fallback.prefix_mode);
        assert_eq!(fallback.matches[0].word, "AARD-VARK");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_oversized_shard_before_reading() {
        let root = temp_root("oversized");
        fs::create_dir_all(root.join("DATA")).unwrap();
        fs::write(root.join("INDEX.TXT"), "AA|DATA/AA.JSN\n").unwrap();
        fs::write(
            root.join("DATA/AA.JSN"),
            vec![b' '; DICTIONARY_SHARD_MAX_BYTES + 1],
        )
        .unwrap();
        assert!(lookup_dictionary(&root, "AA", false)
            .unwrap_err()
            .to_string()
            .contains("shard too large"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rotary_keyboard_remains_bounded() {
        let mut state = DictionaryUiState::default();
        for _ in 0..31 {
            state.apply_button(ButtonEvent::Down);
        }
        assert_eq!(state.selected_key_index(), 1);
        state.apply_button(ButtonEvent::Select);
        assert_eq!(state.query, "B");
    }

    #[test]
    fn keyboard_boot_axis_toggle_preserves_key_and_changes_movement_direction() {
        let mut state = DictionaryUiState::default();
        state.apply_button(ButtonEvent::Down);
        assert_eq!(state.selected_key_label(), "B");
        state.toggle_navigation_axis();
        assert_eq!(state.navigation_mode_label(), "NAV V");
        assert_eq!(state.selected_key_label(), "B");
        state.apply_button(ButtonEvent::Down);
        assert_eq!(state.selected_key_label(), "H");
        state.toggle_navigation_axis();
        assert_eq!(state.navigation_mode_label(), "NAV H");
        assert_eq!(state.selected_key_label(), "H");
    }
}
