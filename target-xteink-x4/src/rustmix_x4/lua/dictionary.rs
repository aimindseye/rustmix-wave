// Rustmix Dictionary: canonical prefix-shard reader for /RUSTMIX/APPS/DICT.
// Reads INDEX.TXT to resolve deep shards such as GOO.JSN before loading DATA/*.JSN.
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

pub const LUA_DICTIONARY_PLAYABLE_APP_MARKER: &str = "rustmix-lua-dictionary-playable-app-ok";
pub const LUA_DICTIONARY_JSON_APP_MARKER: &str = "rustmix-lua-dictionary-json-app-ok";
pub const LUA_DICTIONARY_SEARCH_MARKER: &str = "rustmix-lua-dictionary-search-ok";
pub const LUA_DICTIONARY_KEYBOARD_MARKER: &str = "rustmix-lua-dictionary-keyboard-ok";
pub const LUA_DICTIONARY_PACK_INTEGRITY_MARKER: &str = "rustmix-lua-dictionary-pack-integrity-ok";
// Compatibility symbol retained for existing UI code; the user-facing marker is current.
pub const LUA_DICTIONARY_SHARDED_MARKER: &str = LUA_DICTIONARY_PACK_INTEGRITY_MARKER;
pub const LUA_DICTIONARY_APP_FOLDER: &str = "DICT";
pub const LUA_DICTIONARY_INDEX_FILE: &str = "INDEX.TXT";
pub const LUA_DICTIONARY_DATA_DIR: &str = "DATA";
pub const LUA_DICTIONARY_FALLBACK_JSON_FILE: &str = "DICT.JSN";
pub const LUA_DICTIONARY_JSON_FILE: &str = LUA_DICTIONARY_FALLBACK_JSON_FILE;
pub const LUA_DICTIONARY_LOGICAL_JSON_FILE: &str = "dictionary.json";
pub const MAX_DICTIONARY_SHARD_BYTES: usize = 16 * 1024;
pub const MAX_DICTIONARY_INDEX_BYTES: usize = 96 * 1024;
// Keep INDEX.TXT resolution chunked. A full real index is about 80 KiB, and
// allocating a single 96 KiB buffer on ESP32-C3 can panic before the app opens.
pub const DICTIONARY_INDEX_SCAN_BUF_BYTES: usize = 512;
pub const MAX_DICTIONARY_SCRIPT_BYTES: usize = 16 * 1024;
const MAX_DICTIONARY_INDEX_LINE_BYTES: usize = 128;

const DISPLAY_LINE_CHARS: usize = 40;
const MAX_ENTRIES: usize = 128;
const SEARCH_LETTERS: &[u8; 26] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const KEYBOARD_KEYS: [&str; 30] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "DEL", "CLR", "GO", "*",
];
const KEYBOARD_COLS: usize = 6;
const MAX_SEARCH_QUERY: usize = 16;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DictionaryEntry {
    pub word: String,
    pub part_of_speech: String,
    pub definition: String,
    pub classifiers: Vec<String>,
    pub examples: Vec<String>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DictionaryDisplayEntry<'a> {
    word: &'a str,
    definition_line1: &'a str,
    definition_line2: &'a str,
    definition_line3: &'a str,
    synonyms: &'a str,
    antonyms: &'a str,
}

impl<'a> DictionaryDisplayEntry<'a> {
    pub fn word(&self) -> &'a str {
        self.word
    }
    pub fn definition_line1(&self) -> &'a str {
        self.definition_line1
    }
    pub fn definition_line2(&self) -> &'a str {
        self.definition_line2
    }
    pub fn definition_line3(&self) -> &'a str {
        self.definition_line3
    }
    pub fn synonyms(&self) -> &'a str {
        self.synonyms
    }
    pub fn antonyms(&self) -> &'a str {
        self.antonyms
    }
}

#[derive(Clone, Debug)]
pub struct DictionaryState {
    entries: Vec<DictionaryEntry>,
    selected: usize,
    loaded_from_sd: bool,
    search_letter: usize,
    word_line: String,
    definition_line1: String,
    definition_line2: String,
    definition_line3: String,
    synonyms_line: String,
    antonyms_line: String,
    status_line: String,
    search_query: String,
    keyboard_cursor: usize,
    index_loaded: bool,
    available_shards: [bool; 26],
    current_shard_letter: Option<char>,
}

impl Default for DictionaryState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            selected: 0,
            loaded_from_sd: false,
            search_letter: 0,
            word_line: String::new(),
            definition_line1: String::new(),
            definition_line2: String::new(),
            definition_line3: String::new(),
            synonyms_line: String::new(),
            antonyms_line: String::new(),
            status_line: String::new(),
            search_query: String::new(),
            keyboard_cursor: 0,
            index_loaded: false,
            available_shards: [false; 26],
            current_shard_letter: None,
        }
    }
}

impl DictionaryState {
    pub fn reset(&mut self) {
        self.selected = 0;
        self.search_letter = 0;
        self.search_query.clear();
        self.keyboard_cursor = 0;
        self.refresh_display_cache();
    }

    pub fn load_default(&mut self) {
        let _ = self.load_dictionary_json_with_source(default_dictionary_json(), false);
        self.index_loaded = false;
        self.current_shard_letter = None;
    }

    pub fn load_words(&mut self, data: &str) {
        self.load_dictionary_json(data);
    }

    pub fn load_dictionary_json(&mut self, data: &str) {
        let _ = self.load_dictionary_json_with_source(data, true);
    }

    pub fn load_dictionary_json_with_source(
        &mut self,
        data: &str,
        loaded_from_sd: bool,
    ) -> Result<(), String> {
        self.load_entries_from_json(data, loaded_from_sd, None)
    }

    pub fn load_index(&mut self, data: &str) -> Result<(), String> {
        self.available_shards = [false; 26];
        let mut found = false;
        for raw in data.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split('|');
            let letter = parts.next().unwrap_or("").trim();
            let path = parts.next().unwrap_or("").trim();
            let byte = letter
                .as_bytes()
                .first()
                .copied()
                .unwrap_or(b'\0')
                .to_ascii_uppercase();
            if !(b'A'..=b'Z').contains(&byte) {
                continue;
            }
            if !path.is_empty() && !path_ascii_jsn(path) {
                continue;
            }
            self.available_shards[(byte - b'A') as usize] = true;
            found = true;
        }
        self.index_loaded = found;
        if found {
            self.status_line = LUA_DICTIONARY_SHARDED_MARKER.to_string();
            Ok(())
        } else {
            self.status_line = "missing index".to_string();
            self.refresh_display_cache();
            Err(self.status_line.clone())
        }
    }

    pub fn load_shard_json(&mut self, letter: char, data: &str) -> Result<(), String> {
        let normalized = normalize_shard_letter(letter);
        self.load_shard_json_with_label(
            normalized,
            data,
            shard_file_for_letter(normalized).as_str(),
        )
    }

    pub fn load_shard_json_with_label(
        &mut self,
        letter: char,
        data: &str,
        label: &str,
    ) -> Result<(), String> {
        let normalized = normalize_shard_letter(letter);
        match self.load_entries_from_json(data, true, Some(normalized)) {
            Ok(()) => {
                self.current_shard_letter = Some(normalized);
                self.status_line = format!("{} {}", LUA_DICTIONARY_PACK_INTEGRITY_MARKER, label);
                self.jump_to_query_match();
                Ok(())
            }
            Err(err) => {
                self.mark_parse_failed(err.as_str());
                Err(err)
            }
        }
    }

    fn load_entries_from_json(
        &mut self,
        data: &str,
        loaded_from_sd: bool,
        shard: Option<char>,
    ) -> Result<(), String> {
        match parse_dictionary_json(data) {
            Ok(mut entries) if !entries.is_empty() => {
                sort_entries(&mut entries);
                if entries.len() > MAX_ENTRIES {
                    entries.truncate(MAX_ENTRIES);
                }
                self.entries = entries;
                self.selected = self.selected.min(self.entries.len().saturating_sub(1));
                self.loaded_from_sd = loaded_from_sd;
                self.current_shard_letter = shard;
                self.status_line = if loaded_from_sd {
                    LUA_DICTIONARY_JSON_APP_MARKER.to_string()
                } else {
                    "Built-in dictionary fallback".to_string()
                };
                self.refresh_display_cache();
                Ok(())
            }
            Ok(_) => {
                self.entries.clear();
                self.selected = 0;
                self.loaded_from_sd = false;
                self.status_line = "parse failed: dictionary JSON empty".to_string();
                self.refresh_display_cache();
                Err(self.status_line.clone())
            }
            Err(err) => {
                self.entries.clear();
                self.selected = 0;
                self.loaded_from_sd = false;
                self.status_line = format!("parse failed: {}", err);
                self.refresh_display_cache();
                Err(self.status_line.clone())
            }
        }
    }

    pub fn loaded_from_sd(&self) -> bool {
        self.loaded_from_sd
    }
    pub fn index_loaded(&self) -> bool {
        self.index_loaded
    }

    pub fn query_text(&self) -> &str {
        self.search_query.as_str()
    }

    pub fn mark_index_loaded(&mut self) {
        self.index_loaded = true;
        self.status_line = LUA_DICTIONARY_PACK_INTEGRITY_MARKER.to_string();
        self.refresh_display_cache();
    }

    pub fn desired_shard_letter(&self) -> char {
        self.search_query
            .as_bytes()
            .first()
            .copied()
            .map(|b| char::from(b.to_ascii_uppercase()))
            .filter(|c| c.is_ascii_alphabetic())
            .unwrap_or_else(|| self.search_prefix())
    }

    pub fn desired_shard_file(&self) -> String {
        shard_file_for_letter(self.desired_shard_letter())
    }

    pub fn desired_shard_path(&self) -> String {
        format!("{}/{}", LUA_DICTIONARY_DATA_DIR, self.desired_shard_file())
    }

    pub fn resolved_shard_path_from_index(&self, index: &str) -> Option<String> {
        resolve_shard_path_from_index(index, self.search_query.as_str(), self.search_prefix())
    }

    pub fn current_shard_declared(&self) -> bool {
        let letter = self.desired_shard_letter() as u8;
        if !(b'A'..=b'Z').contains(&letter) {
            return false;
        }
        self.available_shards[(letter - b'A') as usize]
    }

    pub fn mark_missing_index(&mut self) {
        self.entries.clear();
        self.loaded_from_sd = false;
        self.index_loaded = false;
        self.status_line = "missing index".to_string();
        self.refresh_display_cache();
    }

    pub fn mark_missing_shard(&mut self, shard: &str) {
        self.entries.clear();
        self.loaded_from_sd = false;
        self.status_line = format!("missing shard {}", shard);
        self.refresh_display_cache();
    }

    pub fn mark_shard_too_large(&mut self, shard: &str, size: u32) {
        self.entries.clear();
        self.loaded_from_sd = false;
        self.status_line = format!("shard too large {} {}B", shard, size);
        self.refresh_display_cache();
    }

    pub fn mark_parse_failed(&mut self, reason: &str) {
        self.entries.clear();
        self.loaded_from_sd = false;
        if reason.starts_with("parse failed:") {
            self.status_line = reason.to_string();
        } else {
            self.status_line = format!("parse failed: {}", reason);
        }
        self.refresh_display_cache();
    }

    pub fn mark_no_match(&mut self) {
        self.status_line = format!("no match {}", self.search_label());
        self.refresh_display_cache();
    }

    pub fn next_entry(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1) % self.entries.len();
        }
        self.refresh_display_cache();
    }

    pub fn prev_entry(&mut self) {
        if !self.entries.is_empty() {
            self.selected = if self.selected == 0 {
                self.entries.len().saturating_sub(1)
            } else {
                self.selected.saturating_sub(1)
            };
        }
        self.refresh_display_cache();
    }

    pub fn next_search_letter(&mut self) {
        self.search_letter = (self.search_letter + 1) % SEARCH_LETTERS.len();
        self.jump_to_first_search_match();
    }
    pub fn prev_search_letter(&mut self) {
        self.search_letter = if self.search_letter == 0 {
            SEARCH_LETTERS.len().saturating_sub(1)
        } else {
            self.search_letter.saturating_sub(1)
        };
        self.jump_to_first_search_match();
    }
    pub fn next_search_match(&mut self) {
        self.jump_to_next_search_match(1);
    }
    pub fn prev_search_match(&mut self) {
        self.jump_to_next_search_match(self.entries.len().saturating_sub(1));
    }

    pub fn search_prefix(&self) -> char {
        SEARCH_LETTERS
            .get(self.search_letter)
            .copied()
            .map(char::from)
            .unwrap_or('A')
    }

    pub fn search_label(&self) -> String {
        if self.search_query.is_empty() {
            format!("Search: {}*", self.search_prefix())
        } else {
            format!("Search: {}_", self.search_query.as_str())
        }
    }

    pub fn keyboard_cursor(&self) -> usize {
        self.keyboard_cursor
    }
    pub fn keyboard_key(&self, index: usize) -> &'static str {
        KEYBOARD_KEYS.get(index).copied().unwrap_or(" ")
    }

    pub fn keyboard_row_label(&self, row: usize) -> String {
        let mut out = String::new();
        let start = row.saturating_mul(KEYBOARD_COLS);
        for col in 0..KEYBOARD_COLS {
            let idx = start + col;
            if idx >= KEYBOARD_KEYS.len() {
                break;
            }
            if col > 0 {
                out.push(' ');
            }
            let key = self.keyboard_key(idx);
            if idx == self.keyboard_cursor {
                out.push('[');
                out.push_str(key);
                out.push(']');
            } else {
                out.push(' ');
                out.push_str(key);
                out.push(' ');
            }
        }
        out
    }

    pub fn move_keyboard_left(&mut self) {
        let col = self.keyboard_cursor % KEYBOARD_COLS;
        if col == 0 {
            self.keyboard_cursor = (self.keyboard_cursor + KEYBOARD_COLS - 1)
                .min(KEYBOARD_KEYS.len().saturating_sub(1));
        } else {
            self.keyboard_cursor = self.keyboard_cursor.saturating_sub(1);
        }
    }
    pub fn move_keyboard_right(&mut self) {
        let next = self.keyboard_cursor.saturating_add(1);
        if next >= KEYBOARD_KEYS.len() || next % KEYBOARD_COLS == 0 {
            self.keyboard_cursor = self
                .keyboard_cursor
                .saturating_sub(self.keyboard_cursor % KEYBOARD_COLS);
        } else {
            self.keyboard_cursor = next;
        }
    }
    pub fn move_keyboard_up(&mut self) {
        if self.keyboard_cursor >= KEYBOARD_COLS {
            self.keyboard_cursor = self.keyboard_cursor.saturating_sub(KEYBOARD_COLS);
        }
    }
    pub fn move_keyboard_down(&mut self) {
        let next = self.keyboard_cursor.saturating_add(KEYBOARD_COLS);
        if next < KEYBOARD_KEYS.len() {
            self.keyboard_cursor = next;
        }
    }

    pub fn select_keyboard_key(&mut self) {
        match self.keyboard_key(self.keyboard_cursor) {
            "DEL" => {
                self.search_query.pop();
                self.jump_to_query_match();
            }
            "CLR" => {
                self.search_query.clear();
                self.search_letter = 0;
                self.jump_to_first_search_match();
            }
            "GO" => self.jump_to_next_query_match(),
            "*" => {
                if self.search_query.is_empty() {
                    self.jump_to_first_search_match();
                } else {
                    self.jump_to_query_match();
                }
            }
            key => {
                if self.search_query.len() < MAX_SEARCH_QUERY {
                    self.search_query.push_str(key);
                }
                self.jump_to_query_match();
            }
        }
    }

    pub fn entry(&self) -> DictionaryDisplayEntry<'_> {
        DictionaryDisplayEntry {
            word: self.current_word(),
            definition_line1: self.current_definition_line1(),
            definition_line2: self.current_definition_line2(),
            definition_line3: self.current_definition_line3(),
            synonyms: self.current_synonyms(),
            antonyms: self.current_antonyms(),
        }
    }

    pub fn current_word(&self) -> &str {
        self.word_line.as_str()
    }
    pub fn current_definition(&self) -> &str {
        self.definition_line1.as_str()
    }
    pub fn current_definition_line1(&self) -> &str {
        self.definition_line1.as_str()
    }
    pub fn current_definition_line2(&self) -> &str {
        self.definition_line2.as_str()
    }
    pub fn current_definition_line3(&self) -> &str {
        self.definition_line3.as_str()
    }
    pub fn current_synonyms(&self) -> &str {
        self.synonyms_line.as_str()
    }
    pub fn current_antonyms(&self) -> &str {
        self.antonyms_line.as_str()
    }
    pub fn status(&self) -> &str {
        self.status_line.as_str()
    }
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    fn jump_to_first_search_match(&mut self) {
        if self.entries.is_empty() {
            self.refresh_display_cache();
            return;
        }
        let prefix = self.search_prefix();
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| starts_with_ascii_ci(entry.word.as_str(), prefix))
        {
            self.selected = index;
            self.status_line = LUA_DICTIONARY_SEARCH_MARKER.to_string();
        } else {
            self.status_line = format!("no match {}*", prefix);
        }
        self.refresh_display_cache();
    }

    fn jump_to_next_search_match(&mut self, step: usize) {
        if self.entries.is_empty() {
            self.refresh_display_cache();
            return;
        }
        let prefix = self.search_prefix();
        let mut index = self.selected;
        for _ in 0..self.entries.len() {
            index = (index + step) % self.entries.len();
            if starts_with_ascii_ci(self.entries[index].word.as_str(), prefix) {
                self.selected = index;
                self.status_line = LUA_DICTIONARY_SEARCH_MARKER.to_string();
                self.refresh_display_cache();
                return;
            }
        }
        self.status_line = format!("no match {}*", prefix);
        self.refresh_display_cache();
    }

    fn jump_to_query_match(&mut self) {
        if self.search_query.is_empty() {
            self.jump_to_first_search_match();
            return;
        }
        if let Some(index) = self.entries.iter().position(|entry| {
            starts_with_ascii_query(entry.word.as_str(), self.search_query.as_str())
        }) {
            self.selected = index;
            self.status_line = LUA_DICTIONARY_KEYBOARD_MARKER.to_string();
        } else {
            self.status_line = format!("no match {}", self.search_query.as_str());
        }
        self.refresh_display_cache();
    }

    fn jump_to_next_query_match(&mut self) {
        if self.search_query.is_empty() {
            self.next_entry();
            return;
        }
        if self.entries.is_empty() {
            self.refresh_display_cache();
            return;
        }
        let mut index = self.selected;
        for _ in 0..self.entries.len() {
            index = (index + 1) % self.entries.len();
            if starts_with_ascii_query(
                self.entries[index].word.as_str(),
                self.search_query.as_str(),
            ) {
                self.selected = index;
                self.status_line = LUA_DICTIONARY_KEYBOARD_MARKER.to_string();
                self.refresh_display_cache();
                return;
            }
        }
        self.status_line = format!("no match {}", self.search_query.as_str());
        self.refresh_display_cache();
    }

    fn refresh_display_cache(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            self.word_line = entry.word.clone();
            let mut body = String::new();
            if !entry.part_of_speech.is_empty() {
                body.push_str(entry.part_of_speech.as_str());
                body.push_str(": ");
            }
            body.push_str(entry.definition.as_str());
            if let Some(example) = entry.examples.first() {
                body.push_str(" Ex: ");
                body.push_str(example.as_str());
            }
            let lines = wrap_three_lines(body.as_str(), DISPLAY_LINE_CHARS);
            self.definition_line1 = lines[0].clone();
            self.definition_line2 = lines[1].clone();
            self.definition_line3 = lines[2].clone();
            self.synonyms_line = join_labeled("Syn", &entry.synonyms, DISPLAY_LINE_CHARS);
            self.antonyms_line = join_labeled("Ant", &entry.antonyms, DISPLAY_LINE_CHARS);
            if self.status_line.is_empty()
                || self.status_line.starts_with("Dictionary JSON")
                || self.status_line.starts_with(LUA_DICTIONARY_JSON_APP_MARKER)
            {
                self.status_line = if self.loaded_from_sd {
                    LUA_DICTIONARY_SHARDED_MARKER.to_string()
                } else {
                    "Built-in dictionary fallback".to_string()
                };
            }
        } else {
            self.word_line = "Dictionary".to_string();
            self.definition_line1 = self.status_line.clone();
            if self.definition_line1.is_empty() {
                self.definition_line1 =
                    format!("missing index or {}", LUA_DICTIONARY_FALLBACK_JSON_FILE);
            }
            self.definition_line2 = "Use INDEX.TXT + DATA/*.JSN shards".to_string();
            self.definition_line3 = format!("Shard limit: {} bytes", MAX_DICTIONARY_SHARD_BYTES);
            self.synonyms_line.clear();
            self.antonyms_line.clear();
            if self.status_line.is_empty() {
                self.status_line = "No dictionary entries".to_string();
            }
        }
    }
}

fn normalize_shard_letter(letter: char) -> char {
    letter.to_ascii_uppercase()
}
fn shard_file_for_letter(letter: char) -> String {
    format!("{}.JSN", normalize_shard_letter(letter))
}

fn starts_with_ascii_ci(value: &str, prefix: char) -> bool {
    value
        .as_bytes()
        .first()
        .copied()
        .map(|byte| char::from(byte.to_ascii_uppercase()) == prefix)
        .unwrap_or(false)
}

fn starts_with_ascii_query(value: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let value_bytes = value.as_bytes();
    let query_bytes = query.as_bytes();
    if query_bytes.len() > value_bytes.len() {
        return false;
    }
    for (left, right) in value_bytes.iter().copied().zip(query_bytes.iter().copied()) {
        if left.to_ascii_uppercase() != right.to_ascii_uppercase() {
            return false;
        }
    }
    true
}

pub struct DictionaryIndexResolver {
    target: String,
    best_path: String,
    best_score: i32,
    line: [u8; MAX_DICTIONARY_INDEX_LINE_BYTES],
    line_len: usize,
    line_overflow: bool,
    saw_valid_line: bool,
}

impl DictionaryIndexResolver {
    pub fn new(query: &str, fallback_prefix: char) -> Self {
        Self {
            target: normalized_index_target(query, fallback_prefix),
            best_path: String::new(),
            best_score: -1,
            line: [0; MAX_DICTIONARY_INDEX_LINE_BYTES],
            line_len: 0,
            line_overflow: false,
            saw_valid_line: false,
        }
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes.iter().copied() {
            match byte {
                b'\n' => self.flush_line(),
                b'\r' => {}
                _ => {
                    if self.line_overflow {
                        continue;
                    }
                    if self.line_len < self.line.len() {
                        self.line[self.line_len] = byte;
                        self.line_len += 1;
                    } else {
                        self.line_overflow = true;
                    }
                }
            }
        }
    }

    pub fn finish(mut self) -> Option<String> {
        self.flush_line();
        if self.best_score >= 0 {
            Some(self.best_path)
        } else {
            None
        }
    }

    pub fn finish_with_status(mut self) -> (bool, Option<String>) {
        self.flush_line();
        let path = if self.best_score >= 0 {
            Some(self.best_path)
        } else {
            None
        };
        (self.saw_valid_line, path)
    }

    pub fn saw_valid_line(&self) -> bool {
        self.saw_valid_line
    }

    fn flush_line(&mut self) {
        if self.line_overflow {
            self.line_len = 0;
            self.line_overflow = false;
            return;
        }
        if self.line_len == 0 {
            return;
        }
        let Ok(raw) = core::str::from_utf8(&self.line[..self.line_len]) else {
            self.line_len = 0;
            return;
        };
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            self.line_len = 0;
            return;
        }
        let mut parts = line.split('|');
        let name = parts.next().unwrap_or("").trim();
        let path = parts.next().unwrap_or("").trim();
        if name.is_empty() || !path_ascii_jsn(path) {
            self.line_len = 0;
            return;
        }

        let key = normalized_shard_key(name);
        if key.is_empty() || self.target.is_empty() {
            self.line_len = 0;
            return;
        }

        self.saw_valid_line = true;
        let score = if self.target == key {
            10_000 + key.len() as i32
        } else if self.target.starts_with(key.as_str()) {
            8_000 + key.len() as i32
        } else if key.starts_with(self.target.as_str()) {
            4_000 - key.len() as i32
        } else {
            -1
        };

        if score > self.best_score {
            self.best_score = score;
            self.best_path.clear();
            if path.is_empty() {
                self.best_path.push_str(LUA_DICTIONARY_DATA_DIR);
                self.best_path.push('/');
                self.best_path.push_str(name);
                if !name.ends_with(".JSN") && !name.ends_with(".jsn") {
                    self.best_path.push_str(".JSN");
                }
            } else {
                self.best_path.push_str(path);
            }
        }
        self.line_len = 0;
    }
}

pub fn resolve_shard_path_from_index(
    index: &str,
    query: &str,
    fallback_prefix: char,
) -> Option<String> {
    let target = normalized_index_target(query, fallback_prefix);
    if target.is_empty() {
        return None;
    }

    let mut best_path = String::new();
    let mut best_score: i32 = -1;

    for raw in index.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split('|');
        let name = parts.next().unwrap_or("").trim();
        let path = parts.next().unwrap_or("").trim();
        if name.is_empty() || !path_ascii_jsn(path) {
            continue;
        }

        let key = normalized_shard_key(name);
        if key.is_empty() {
            continue;
        }

        let score = if target == key {
            10_000 + key.len() as i32
        } else if target.starts_with(key.as_str()) {
            8_000 + key.len() as i32
        } else if key.starts_with(target.as_str()) {
            4_000 - key.len() as i32
        } else {
            -1
        };

        if score > best_score {
            best_score = score;
            best_path.clear();
            if path.is_empty() {
                best_path.push_str(LUA_DICTIONARY_DATA_DIR);
                best_path.push('/');
                best_path.push_str(name);
                if !name.ends_with(".JSN") && !name.ends_with(".jsn") {
                    best_path.push_str(".JSN");
                }
            } else {
                best_path.push_str(path);
            }
        }
    }

    if best_score >= 0 {
        Some(best_path)
    } else {
        None
    }
}

fn normalized_index_target(query: &str, fallback_prefix: char) -> String {
    let mut out = String::new();
    let mut source = query.trim();
    while source.ends_with('*') || source.ends_with('_') {
        source = source[..source.len().saturating_sub(1)].trim_end();
    }

    if source.is_empty() {
        let c = fallback_prefix.to_ascii_uppercase();
        if c.is_ascii_alphabetic() {
            out.push(c);
        }
        return out;
    }

    for ch in source.chars() {
        let up = ch.to_ascii_uppercase();
        if up.is_ascii_alphabetic() {
            out.push(up);
        } else {
            break;
        }
        if out.len() >= 5 {
            break;
        }
    }

    if out.is_empty() {
        out.push_str("OTHERS");
    }
    out
}

fn normalized_shard_key(name: &str) -> String {
    let tail = name.rsplit('/').next().unwrap_or(name);
    let tail = tail.rsplit('\\').next().unwrap_or(tail);
    let base = tail.split('.').next().unwrap_or(tail).trim();
    let mut out = String::new();
    for ch in base.chars() {
        let up = ch.to_ascii_uppercase();
        if up.is_ascii_alphabetic() {
            out.push(up);
        } else if up.is_ascii_digit() {
            break;
        } else if up == '_' {
            out.push(up);
        } else {
            break;
        }
    }
    out
}

fn path_ascii_jsn(path: &str) -> bool {
    let upper = path_ascii_upper(path);
    upper.ends_with(".JSN") || upper.is_empty()
}

fn path_ascii_upper(path: &str) -> String {
    let mut out = String::new();
    for ch in path.chars() {
        out.push(ch.to_ascii_uppercase());
    }
    out
}

fn sort_entries(entries: &mut [DictionaryEntry]) {
    entries.sort_by(|left, right| left.word.as_str().cmp(right.word.as_str()));
}

fn join_labeled(label: &str, values: &[String], limit: usize) -> String {
    if values.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(label);
    out.push_str(": ");
    for (idx, item) in values.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        if out.len().saturating_add(item.len()) > limit {
            out.push('…');
            break;
        }
        out.push_str(item.as_str());
    }
    out
}

fn wrap_three_lines(text: &str, width: usize) -> [String; 3] {
    let mut lines = [String::new(), String::new(), String::new()];
    let mut line_idx = 0usize;
    for word in text.split_whitespace() {
        if line_idx >= 3 {
            break;
        }
        let needs_space = !lines[line_idx].is_empty();
        let projected = lines[line_idx].len() + word.len() + usize::from(needs_space);
        if projected > width && !lines[line_idx].is_empty() {
            line_idx += 1;
            if line_idx >= 3 {
                break;
            }
        }
        if !lines[line_idx].is_empty() {
            lines[line_idx].push(' ');
        }
        lines[line_idx].push_str(word);
    }
    lines
}

pub fn parse_dictionary_json(input: &str) -> Result<Vec<DictionaryEntry>, String> {
    let mut parser = JsonParser::new(input);
    parser.skip_ws();
    parser.expect_char('{')?;
    let mut entries = Vec::new();
    loop {
        parser.skip_ws();
        if parser.consume_char('}') {
            break;
        }
        let word = parser.parse_string()?;
        parser.skip_ws();
        parser.expect_char(':')?;
        let entry = parser.parse_word_entry(word)?;
        entries.push(entry);
        parser.skip_ws();
        if parser.consume_char(',') {
            continue;
        }
        parser.expect_char('}')?;
        break;
    }
    Ok(entries)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            pos: 0,
        }
    }
    fn skip_ws(&mut self) {
        while let Some(byte) = self.bytes.get(self.pos) {
            if matches!(byte, b' ' | b'\n' | b'\r' | b'\t') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }
    fn consume_char(&mut self, expected: char) -> bool {
        self.skip_ws();
        if self.peek() == Some(expected as u8) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        self.skip_ws();
        if self.consume_char(expected) {
            Ok(())
        } else {
            Err(format!("Dictionary JSON expected '{}'", expected))
        }
    }
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }
    fn parse_string(&mut self) -> Result<String, String> {
        self.skip_ws();
        self.expect_char('"')?;
        let mut out = String::new();
        while let Some(byte) = self.bytes.get(self.pos).copied() {
            self.pos += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let escaped = self
                        .bytes
                        .get(self.pos)
                        .copied()
                        .ok_or_else(|| "Dictionary JSON bad escape".to_string())?;
                    self.pos += 1;
                    match escaped {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        _ => return Err("Dictionary JSON unsupported escape".to_string()),
                    }
                }
                _ => out.push(byte as char),
            }
        }
        Err("Dictionary JSON unterminated string".to_string())
    }
    fn parse_word_entry(&mut self, word: String) -> Result<DictionaryEntry, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_word_object_entry(word),
            Some(b'[') => self.parse_definition_array_entry(word),
            Some(b'"') => {
                let definition = self.parse_string()?;
                Ok(DictionaryEntry {
                    word,
                    definition,
                    ..DictionaryEntry::default()
                })
            }
            _ => Err("Dictionary JSON unsupported entry value".to_string()),
        }
    }

    fn parse_word_object_entry(&mut self, word: String) -> Result<DictionaryEntry, String> {
        self.expect_char('{')?;
        let mut entry = DictionaryEntry {
            word,
            ..DictionaryEntry::default()
        };
        loop {
            self.skip_ws();
            if self.consume_char('}') {
                break;
            }
            let key = self.parse_string()?;
            self.expect_char(':')?;
            match key.as_str() {
                "MEANINGS" | "meanings" => self.parse_meanings(&mut entry)?,
                "SYNONYMS" | "synonyms" => entry.synonyms = self.parse_optional_string_array()?,
                "ANTONYMS" | "antonyms" => entry.antonyms = self.parse_optional_string_array()?,
                "def" | "definition" | "meaning" | "text" => {
                    if self.peek_string_value() {
                        entry.definition = self.parse_string()?;
                    } else {
                        self.skip_value()?;
                    }
                }
                "pos" | "part_of_speech" | "partOfSpeech" => {
                    if self.peek_string_value() {
                        entry.part_of_speech = self.parse_string()?;
                    } else {
                        self.skip_value()?;
                    }
                }
                "examples" => entry.examples = self.parse_optional_string_array()?,
                "classifiers" => entry.classifiers = self.parse_optional_string_array()?,
                _ => self.skip_value()?,
            }
            self.skip_ws();
            if self.consume_char(',') {
                continue;
            }
            self.expect_char('}')?;
            break;
        }
        Ok(entry)
    }

    fn parse_definition_array_entry(&mut self, word: String) -> Result<DictionaryEntry, String> {
        self.expect_char('[')?;
        let mut entry = DictionaryEntry {
            word,
            ..DictionaryEntry::default()
        };
        loop {
            self.skip_ws();
            if self.consume_char(']') {
                break;
            }
            match self.peek() {
                Some(b'{') => self.parse_definition_object_into_entry(&mut entry)?,
                Some(b'"') => {
                    let value = self.parse_string()?;
                    if entry.definition.is_empty() {
                        entry.definition = value;
                    }
                }
                Some(b'[') => self.skip_value()?,
                Some(_) => self.skip_value()?,
                None => return Err("Dictionary JSON unexpected end".to_string()),
            }
            self.skip_ws();
            if self.consume_char(',') {
                continue;
            }
            self.expect_char(']')?;
            break;
        }
        if entry.definition.is_empty() {
            entry.definition = "No definition text".to_string();
        }
        Ok(entry)
    }

    fn parse_definition_object_into_entry(
        &mut self,
        entry: &mut DictionaryEntry,
    ) -> Result<(), String> {
        self.expect_char('{')?;
        let mut definition = String::new();
        let mut part_of_speech = String::new();
        let mut examples: Vec<String> = Vec::new();
        loop {
            self.skip_ws();
            if self.consume_char('}') {
                break;
            }
            let key = self.parse_string()?;
            self.expect_char(':')?;
            match key.as_str() {
                "def" | "definition" | "meaning" | "text" => {
                    if self.peek_string_value() {
                        definition = self.parse_string()?;
                    } else {
                        self.skip_value()?;
                    }
                }
                "pos" | "part_of_speech" | "partOfSpeech" => {
                    if self.peek_string_value() {
                        part_of_speech = self.parse_string()?;
                    } else {
                        self.skip_value()?;
                    }
                }
                "example" => {
                    if self.peek_string_value() {
                        examples.push(self.parse_string()?);
                    } else {
                        self.skip_value()?;
                    }
                }
                "examples" => examples = self.parse_optional_string_array()?,
                "synonyms" => {
                    if entry.synonyms.is_empty() {
                        entry.synonyms = self.parse_optional_string_array()?;
                    } else {
                        self.skip_value()?;
                    }
                }
                "antonyms" => {
                    if entry.antonyms.is_empty() {
                        entry.antonyms = self.parse_optional_string_array()?;
                    } else {
                        self.skip_value()?;
                    }
                }
                _ => self.skip_value()?,
            }
            self.skip_ws();
            if self.consume_char(',') {
                continue;
            }
            self.expect_char('}')?;
            break;
        }
        if entry.definition.is_empty() && !definition.is_empty() {
            entry.definition = definition;
        }
        if entry.part_of_speech.is_empty() && !part_of_speech.is_empty() {
            entry.part_of_speech = part_of_speech;
        }
        if entry.examples.is_empty() && !examples.is_empty() {
            entry.examples = examples;
        }
        Ok(())
    }

    fn parse_meanings(&mut self, entry: &mut DictionaryEntry) -> Result<(), String> {
        self.expect_char('[')?;
        let mut first = true;
        loop {
            self.skip_ws();
            if self.consume_char(']') {
                break;
            }
            self.expect_char('[')?;
            let part = self.parse_string()?;
            self.expect_char(',')?;
            let definition = self.parse_string()?;
            self.expect_char(',')?;
            let classifiers = self.parse_string_array()?;
            self.expect_char(',')?;
            let examples = self.parse_string_array()?;
            self.skip_ws();
            self.expect_char(']')?;
            if first {
                entry.part_of_speech = part;
                entry.definition = definition;
                entry.classifiers = classifiers;
                entry.examples = examples;
                first = false;
            }
            self.skip_ws();
            if self.consume_char(',') {
                continue;
            }
            self.expect_char(']')?;
            break;
        }
        Ok(())
    }
    fn parse_string_array(&mut self) -> Result<Vec<String>, String> {
        self.expect_char('[')?;
        let mut values = Vec::new();
        loop {
            self.skip_ws();
            if self.consume_char(']') {
                break;
            }
            values.push(self.parse_string()?);
            self.skip_ws();
            if self.consume_char(',') {
                continue;
            }
            self.expect_char(']')?;
            break;
        }
        Ok(values)
    }

    fn parse_optional_string_array(&mut self) -> Result<Vec<String>, String> {
        self.skip_ws();
        if self.peek() != Some(b'[') {
            self.skip_value()?;
            return Ok(Vec::new());
        }
        self.expect_char('[')?;
        let mut values = Vec::new();
        loop {
            self.skip_ws();
            if self.consume_char(']') {
                break;
            }
            if self.peek_string_value() {
                values.push(self.parse_string()?);
            } else {
                self.skip_value()?;
            }
            self.skip_ws();
            if self.consume_char(',') {
                continue;
            }
            self.expect_char(']')?;
            break;
        }
        Ok(values)
    }

    fn peek_string_value(&mut self) -> bool {
        self.skip_ws();
        self.peek() == Some(b'"')
    }

    fn skip_value(&mut self) -> Result<(), String> {
        self.skip_ws();
        match self.peek() {
            Some(b'"') => self.parse_string().map(|_| ()),
            Some(b'{') => {
                self.pos += 1;
                loop {
                    self.skip_ws();
                    if self.consume_char('}') {
                        return Ok(());
                    }
                    self.parse_string()?;
                    self.expect_char(':')?;
                    self.skip_value()?;
                    self.skip_ws();
                    if self.consume_char(',') {
                        continue;
                    }
                    self.expect_char('}')?;
                    return Ok(());
                }
            }
            Some(b'[') => {
                self.pos += 1;
                loop {
                    self.skip_ws();
                    if self.consume_char(']') {
                        return Ok(());
                    }
                    self.skip_value()?;
                    self.skip_ws();
                    if self.consume_char(',') {
                        continue;
                    }
                    self.expect_char(']')?;
                    return Ok(());
                }
            }
            Some(_) => {
                while let Some(byte) = self.peek() {
                    if matches!(byte, b',' | b']' | b'}' | b'\n' | b'\r' | b'\t' | b' ') {
                        break;
                    }
                    self.pos += 1;
                }
                Ok(())
            }
            None => Err("Dictionary JSON unexpected end".to_string()),
        }
    }
}

fn default_dictionary_json() -> &'static str {
    r#"{
  "ABANDON": {
    "MEANINGS": [
      ["Verb", "forsake, leave behind", ["Discard", "Fling", "Toss", "Dispose"], ["We abandoned the old car in the empty parking lot"]],
      ["Verb", "give up with the intent of never claiming again", [], ["Abandon your life to God"]]
    ],
    "ANTONYMS": [],
    "SYNONYMS": ["Abandon", "Desolate", "Vacate", "Desert", "Wantonness"]
  },
  "ABATE": {
    "MEANINGS": [
      ["Verb", "become less intense or widespread", ["Reduce", "Lessen"], ["The storm began to abate after midnight"]]
    ],
    "ANTONYMS": ["Increase"],
    "SYNONYMS": ["Diminish", "Subside", "Decrease"]
  },
  "BOOK": {
    "MEANINGS": [
      ["Noun", "a written or printed work", ["Volume"], ["She opened the book"]]
    ],
    "ANTONYMS": [],
    "SYNONYMS": ["Volume", "Text"]
  }
}"#
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_dictionary_json_sample() {
        let entries = parse_dictionary_json(default_dictionary_json()).unwrap();
        assert_eq!(entries[0].word, "ABANDON");
        assert_eq!(entries[0].part_of_speech, "Verb");
        assert!(entries[0].definition.contains("forsake"));
        assert!(entries[0].synonyms.iter().any(|value| value == "Desert"));
    }
    #[test]
    fn parses_prefix_shard_array_definitions() {
        let shard = r#"{
          "GOOD":[
            {"def":"Possessing desirable qualities; useful; fit; excellent.","pos":"adj"},
            {"def":"Possessing moral excellence or virtue.","pos":"adj"}
          ],
          "GOOSE":[{"def":"A waterfowl.","pos":"noun"}]
        }"#;
        let entries = parse_dictionary_json(shard).unwrap();
        assert_eq!(entries[0].word, "GOOD");
        assert_eq!(entries[0].part_of_speech, "adj");
        assert!(entries[0].definition.contains("Possessing desirable"));
    }

    #[test]
    fn parses_prefix_shard_string_definitions() {
        let shard = r#"{"A":"first letter","AA":["rough lava"]}"#;
        let entries = parse_dictionary_json(shard).unwrap();
        assert_eq!(entries[0].word, "A");
        assert_eq!(entries[0].definition, "first letter");
        assert_eq!(entries[1].definition, "rough lava");
    }

    #[test]
    fn keyboard_builds_query_and_jumps_to_matching_word() {
        let mut state = DictionaryState::default();
        state.load_default();
        state.select_keyboard_key();
        assert!(state.search_label().contains("A_"));
        assert_eq!(state.current_word(), "ABANDON");
        state.move_keyboard_right();
        state.select_keyboard_key();
        assert!(state.search_label().contains("AB_"));
        assert_eq!(state.current_word(), "ABANDON");
    }
    #[test]
    fn index_maps_letters_to_shards() {
        let mut state = DictionaryState::default();
        state.load_index("A|DATA/A.JSN\nB|DATA/B.JSN\n").unwrap();
        assert!(state.index_loaded());
        assert_eq!(state.desired_shard_file(), "A.JSN");
        assert!(state.current_shard_declared());
    }

    #[test]
    fn index_resolver_selects_deep_prefix_shard() {
        let index = "GAB|DATA/GAB.JSN\nGOO|DATA/GOO.JSN\nGRAN1|DATA/GRAN1.JSN\n";
        assert_eq!(
            resolve_shard_path_from_index(index, "GOOD", 'A').as_deref(),
            Some("DATA/GOO.JSN")
        );
        assert_eq!(
            resolve_shard_path_from_index(index, "GRANARY", 'A').as_deref(),
            Some("DATA/GRAN1.JSN")
        );
    }

    #[test]
    fn index_resolver_selects_first_matching_letter_for_browse() {
        let index = "GAB|DATA/GAB.JSN\nGAD|DATA/GAD.JSN\nGE|DATA/GE.JSN\n";
        assert_eq!(
            resolve_shard_path_from_index(index, "", 'G').as_deref(),
            Some("DATA/GAB.JSN")
        );
        assert_eq!(
            resolve_shard_path_from_index(index, "G*", 'A').as_deref(),
            Some("DATA/GAB.JSN")
        );
    }
}
