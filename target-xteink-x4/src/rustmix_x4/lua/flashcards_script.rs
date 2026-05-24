//! SD-backed Flashcards app model for Rustmix.
//!
//! The app is displayed from Productivity and keeps subjects on the SD card
//! under `/RUSTMIX/APPS/FLASHCRD/TOPICS/<TOPIC>`.  Each topic is either a
//! text-card topic or an image-card topic. Image cards avoid device-side font
//! rendering by using pre-rendered X4B bitmaps generated on the host.

pub const LUA_FLASHCARDS_APP_MARKER: &str = "rustmix-lua-flashcards-topic-navigation-ok";
pub const LUA_FLASHCARDS_APP_FOLDER: &str = "FLASHCRD";
pub const LUA_FLASHCARDS_MANIFEST_FILE: &str = "APP.TOM";
pub const LUA_FLASHCARDS_ENTRY_FILE: &str = "MAIN.LUA";
pub const LUA_FLASHCARDS_TOPICS_DIR: &str = "TOPICS";
pub const LUA_FLASHCARDS_TOPIC_INDEX_FILE: &str = "INDEX.TXT";
pub const LUA_FLASHCARDS_CARD_FILE: &str = "CARDS.TXT";
pub const FLASHCARDS_DEFAULT_TOPIC_INDEX: &str =
    "TEXTDEMO|Text Flashcards|TEXT\nIMGDEMO|Image Flashcards|IMAGE\n";
pub const FLASHCARDS_DEFAULT_TEXT_TOPIC_FOLDER: &str = "TEXTDEMO";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlashcardsSource {
    SdLuaScript,
    BuiltInFallback,
    MissingManifest,
    MissingEntry,
    MissingTopicIndex,
    MissingTopicCards,
    MissingTopicImage,
    ManifestInvalidUtf8,
    ScriptInvalidUtf8,
    TopicIndexInvalidUtf8,
    TopicCardsInvalidUtf8,
    InvalidManifestContract,
    EmptyTopicIndex,
    EmptyCards,
    BadImageCard,
}

impl FlashcardsSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SdLuaScript => "SD Lua",
            Self::BuiltInFallback => "Fallback",
            Self::MissingManifest => "Missing APP.TOM",
            Self::MissingEntry => "Missing MAIN.LUA",
            Self::MissingTopicIndex => "Missing topics",
            Self::MissingTopicCards => "Missing cards",
            Self::MissingTopicImage => "Missing image",
            Self::ManifestInvalidUtf8 => "APP.TOM UTF-8 error",
            Self::ScriptInvalidUtf8 => "MAIN.LUA UTF-8 error",
            Self::TopicIndexInvalidUtf8 => "INDEX.TXT UTF-8 error",
            Self::TopicCardsInvalidUtf8 => "CARDS.TXT UTF-8 error",
            Self::InvalidManifestContract => "Bad APP.TOM",
            Self::EmptyTopicIndex => "No topics",
            Self::EmptyCards => "No cards",
            Self::BadImageCard => "Bad image row",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlashcardsText<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> FlashcardsText<N> {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    pub fn from_str(value: &str) -> Self {
        let mut text = Self::empty();
        text.set(value);
        text
    }

    pub fn clear(&mut self) {
        self.bytes = [0; N];
        self.len = 0;
    }

    pub fn set(&mut self, value: &str) {
        self.clear();
        self.push_str(value);
    }

    pub fn push_str(&mut self, value: &str) {
        for ch in value.chars() {
            self.push_char(ch);
        }
    }

    pub fn push_char(&mut self, value: char) {
        let mut buf = [0u8; 4];
        let encoded = value.encode_utf8(&mut buf);
        if self.len + encoded.len() <= N {
            self.bytes[self.len..self.len + encoded.len()].copy_from_slice(encoded.as_bytes());
            self.len += encoded.len();
        }
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlashcardsScreen {
    pub source: FlashcardsSource,
    pub title: FlashcardsText<48>,
    pub topic: FlashcardsText<96>,
    pub line1: FlashcardsText<384>,
    pub line2: FlashcardsText<384>,
    pub line3: FlashcardsText<192>,
    pub footer: FlashcardsText<192>,
    pub image_file: FlashcardsText<40>,
    pub image_card: bool,
    pub topic_count: usize,
    pub card_count: usize,
}

impl FlashcardsScreen {
    pub fn fallback() -> Self {
        Self {
            source: FlashcardsSource::BuiltInFallback,
            title: FlashcardsText::from_str("Flashcards"),
            topic: FlashcardsText::from_str("Upload topics to /RUSTMIX/APPS/FLASHCRD/TOPICS"),
            line1: FlashcardsText::from_str("Topics support TEXT or IMAGE mode."),
            line2: FlashcardsText::from_str("Default topics: TEXTDEMO and IMGDEMO."),
            line3: FlashcardsText::from_str("Use Back to return to Productivity."),
            footer: FlashcardsText::from_str("Folder: /RUSTMIX/APPS/FLASHCRD"),
            image_file: FlashcardsText::empty(),
            image_card: false,
            topic_count: 0,
            card_count: 0,
        }
    }

    pub fn diagnostic(source: FlashcardsSource, primary: &str, remediation: &str) -> Self {
        let mut screen = Self::fallback();
        screen.source = source;
        screen.line1.set(primary);
        screen.line2.set(remediation);
        screen.line3.set("Expected: INDEX.TXT + TOPIC/CARDS.TXT");
        screen.footer.set("Back returns to Productivity");
        screen
    }

    pub fn title(&self) -> &str {
        self.title.as_str()
    }
    pub fn topic(&self) -> &str {
        self.topic.as_str()
    }
    pub fn line1(&self) -> &str {
        self.line1.as_str()
    }
    pub fn line2(&self) -> &str {
        self.line2.as_str()
    }
    pub fn line3(&self) -> &str {
        self.line3.as_str()
    }
    pub fn footer(&self) -> &str {
        self.footer.as_str()
    }
    pub fn image_file(&self) -> &str {
        self.image_file.as_str()
    }
    pub const fn is_image_card(&self) -> bool {
        self.image_card
    }
}

impl Default for FlashcardsScreen {
    fn default() -> Self {
        Self::fallback()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlashcardsTopicKind {
    Text,
    Image,
}

impl FlashcardsTopicKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Text => "TEXT",
            Self::Image => "IMAGE",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlashcardsTopic<'a> {
    pub folder: &'a str,
    pub name: &'a str,
    pub kind: FlashcardsTopicKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlashcardKind {
    Text,
    Image,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlashcardRecord<'a> {
    pub kind: FlashcardKind,
    pub front: &'a str,
    pub back: &'a str,
    pub hint: &'a str,
}

pub fn build_flashcards_topic_list_screen(
    manifest: &str,
    _script: &str,
    topic_index: &str,
    selected_topic: usize,
) -> FlashcardsScreen {
    if !manifest_declares_flashcards(manifest) {
        return FlashcardsScreen::diagnostic(
            FlashcardsSource::InvalidManifestContract,
            "APP.TOM id must be FLASHCRD or flashcards",
            "Fix APP.TOM and reopen Flashcards",
        );
    }

    let topic_count = topic_count(topic_index);
    if topic_count == 0 {
        return FlashcardsScreen::diagnostic(
            FlashcardsSource::EmptyTopicIndex,
            "No topics found in TOPICS/INDEX.TXT",
            "Add rows like: TEXTDEMO|Text Flashcards|TEXT",
        );
    }

    let selected_topic_index = selected_topic.min(topic_count.saturating_sub(1));
    let Some(topic) = topic_at(topic_index, selected_topic_index) else {
        return FlashcardsScreen::diagnostic(
            FlashcardsSource::EmptyTopicIndex,
            "Selected topic could not be parsed",
            "Check TOPICS/INDEX.TXT rows",
        );
    };

    let mut screen = FlashcardsScreen::fallback();
    screen.source = FlashcardsSource::SdLuaScript;
    screen.title.set("Flashcards");
    screen.topic.set("Choose a topic");
    screen.topic_count = topic_count;
    screen.card_count = 0;
    screen.line1.set("Selected: ");
    screen.line1.push_str(topic.name);
    screen.line1.push_str(" [");
    screen.line1.push_str(topic.kind.label());
    screen.line1.push_str("]");

    if let Some(prev_topic) = circular_topic_at(topic_index, selected_topic_index, -1) {
        screen.line2.set("Prev: ");
        screen.line2.push_str(prev_topic.name);
    } else {
        screen.line2.set("Prev: -");
    }

    if let Some(next_topic) = circular_topic_at(topic_index, selected_topic_index, 1) {
        screen.line3.set("Next: ");
        screen.line3.push_str(next_topic.name);
    } else {
        screen.line3.set("Next: -");
    }

    screen
        .footer
        .set("OK opens | Back exits | arrows change topic | ");
    push_usize(&mut screen.footer, selected_topic_index + 1);
    screen.footer.push_str("/");
    push_usize(&mut screen.footer, topic_count);
    screen
}

pub fn build_flashcards_screen(
    manifest: &str,
    _script: &str,
    topic_index: &str,
    cards: &str,
    selected_topic: usize,
    selected_card: usize,
    show_back: bool,
) -> FlashcardsScreen {
    if !manifest_declares_flashcards(manifest) {
        return FlashcardsScreen::diagnostic(
            FlashcardsSource::InvalidManifestContract,
            "APP.TOM id must be FLASHCRD or flashcards",
            "Fix APP.TOM and reopen Flashcards",
        );
    }

    let topic_count = topic_count(topic_index);
    if topic_count == 0 {
        return FlashcardsScreen::diagnostic(
            FlashcardsSource::EmptyTopicIndex,
            "No topics found in TOPICS/INDEX.TXT",
            "Add rows like: TEXTDEMO|Text Flashcards|TEXT",
        );
    }

    let selected_topic_index = selected_topic.min(topic_count.saturating_sub(1));
    let Some(topic) = topic_at(topic_index, selected_topic_index) else {
        return FlashcardsScreen::diagnostic(
            FlashcardsSource::EmptyTopicIndex,
            "Selected topic could not be parsed",
            "Check TOPICS/INDEX.TXT rows",
        );
    };

    let effective_kind = effective_topic_kind(topic.kind, cards);
    let card_count = card_count_for_topic(cards, effective_kind);
    if card_count == 0 {
        let mut screen = FlashcardsScreen::diagnostic(
            FlashcardsSource::EmptyCards,
            "No cards found for selected topic",
            "TEXT rows: front|back|hint; IMAGE rows: IMG|front|back|hint",
        );
        screen.topic.set(topic.name);
        screen.topic_count = topic_count;
        return screen;
    }

    let card_index = selected_card.min(card_count.saturating_sub(1));
    let Some(card) = card_at_for_topic(cards, effective_kind, card_index) else {
        return FlashcardsScreen::diagnostic(
            FlashcardsSource::EmptyCards,
            "Selected card could not be parsed",
            "Check CARDS.TXT rows",
        );
    };

    let mut screen = FlashcardsScreen::fallback();
    screen.source = FlashcardsSource::SdLuaScript;
    screen.topic_count = topic_count;
    screen.card_count = card_count;
    screen.topic.set(topic.name);

    match card.kind {
        FlashcardKind::Image => {
            let image_file = if show_back { card.back } else { card.front };
            if !is_safe_image_ref(image_file) {
                return FlashcardsScreen::diagnostic(
                    FlashcardsSource::BadImageCard,
                    "Image card path must be IMG/<8.3>.X4B",
                    "Regenerate or re-upload the image topic",
                );
            }
            screen.image_card = true;
            screen.image_file.set(image_file);
            screen.line1.set("Image card");
            screen.line2.set(if show_back {
                "Back image"
            } else {
                "Front image"
            });
            screen.line3.set(card.hint);
        }
        FlashcardKind::Text => {
            if show_back {
                screen.line1.set("Answer");
                screen.line2.set(card.back);
                if card.hint.is_empty() {
                    screen
                        .line3
                        .set("Select hides answer; Prev/Next changes card");
                } else {
                    screen.line3.set("Hint: ");
                    screen.line3.push_str(card.hint);
                }
            } else {
                screen.line1.set("Question");
                screen.line2.set(card.front);
                if card.hint.is_empty() {
                    screen
                        .line3
                        .set("Select shows answer; Prev/Next changes card");
                } else {
                    screen.line3.set("Hint: ");
                    screen.line3.push_str(card.hint);
                }
            }
        }
    }

    screen.footer.set(effective_kind.label());
    screen.footer.push_str(" | Card ");
    push_usize(&mut screen.footer, card_index + 1);
    screen.footer.push_str("/");
    push_usize(&mut screen.footer, card_count);
    screen.footer.push_str(" | Topic ");
    push_usize(&mut screen.footer, selected_topic_index + 1);
    screen.footer.push_str("/");
    push_usize(&mut screen.footer, topic_count);
    screen
}

pub fn topic_count(index: &str) -> usize {
    index.lines().filter_map(parse_topic_row).count()
}

pub fn topic_folder_at(index: &str, selected: usize) -> Option<&str> {
    topic_at(index, selected).map(|topic| topic.folder)
}

pub fn topic_kind_at(index: &str, selected: usize) -> Option<FlashcardsTopicKind> {
    topic_at(index, selected).map(|topic| topic.kind)
}

pub fn topic_at(index: &str, selected: usize) -> Option<FlashcardsTopic<'_>> {
    index.lines().filter_map(parse_topic_row).nth(selected)
}

pub fn card_count(cards: &str) -> usize {
    cards.lines().filter_map(parse_card_row).count()
}

pub fn card_count_for_topic(cards: &str, topic_kind: FlashcardsTopicKind) -> usize {
    cards
        .lines()
        .filter_map(parse_card_row)
        .filter(|card| card_matches_topic_kind(*card, topic_kind))
        .count()
}

pub fn card_at(cards: &str, selected: usize) -> Option<FlashcardRecord<'_>> {
    cards.lines().filter_map(parse_card_row).nth(selected)
}

pub fn card_at_for_topic(
    cards: &str,
    topic_kind: FlashcardsTopicKind,
    selected: usize,
) -> Option<FlashcardRecord<'_>> {
    cards
        .lines()
        .filter_map(parse_card_row)
        .filter(|card| card_matches_topic_kind(*card, topic_kind))
        .nth(selected)
}

pub fn is_safe_topic_folder(folder: &str) -> bool {
    let bytes = folder.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 8
        && bytes
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_' || *b == b'-')
}

pub fn is_safe_image_ref(value: &str) -> bool {
    let Some((dir, file)) = value.split_once('/') else {
        return false;
    };
    dir == "IMG" && is_safe_x4b_file(file)
}

fn is_safe_x4b_file(file: &str) -> bool {
    let Some((base, ext)) = file.rsplit_once('.') else {
        return false;
    };
    if !ext.eq_ignore_ascii_case("X4B") || base.is_empty() || base.len() > 8 {
        return false;
    }
    base.as_bytes()
        .iter()
        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_' || *b == b'-')
}

fn circular_topic_at(index: &str, selected: usize, delta: i8) -> Option<FlashcardsTopic<'_>> {
    let count = topic_count(index);
    if count == 0 {
        return None;
    }
    let selected = selected.min(count.saturating_sub(1));
    let target = if delta < 0 {
        if selected == 0 {
            count.saturating_sub(1)
        } else {
            selected.saturating_sub(1)
        }
    } else if selected + 1 >= count {
        0
    } else {
        selected + 1
    };
    topic_at(index, target)
}

fn effective_topic_kind(topic_kind: FlashcardsTopicKind, cards: &str) -> FlashcardsTopicKind {
    match topic_kind {
        FlashcardsTopicKind::Image => FlashcardsTopicKind::Image,
        FlashcardsTopicKind::Text => {
            let text_count = card_count_for_topic(cards, FlashcardsTopicKind::Text);
            let image_count = card_count_for_topic(cards, FlashcardsTopicKind::Image);
            if text_count == 0 && image_count > 0 {
                FlashcardsTopicKind::Image
            } else {
                FlashcardsTopicKind::Text
            }
        }
    }
}

fn card_matches_topic_kind(card: FlashcardRecord<'_>, topic_kind: FlashcardsTopicKind) -> bool {
    matches!(
        (card.kind, topic_kind),
        (FlashcardKind::Text, FlashcardsTopicKind::Text)
            | (FlashcardKind::Image, FlashcardsTopicKind::Image)
    )
}

fn parse_topic_row(line: &str) -> Option<FlashcardsTopic<'_>> {
    let line = line.split('#').next().unwrap_or("").trim();
    if line.is_empty() {
        return None;
    }
    let mut parts = line.split('|');
    let folder = parts.next()?.trim();
    let name = parts.next().unwrap_or(folder).trim();
    let kind_raw = parts.next().unwrap_or("TEXT").trim();
    let kind = parse_topic_kind(kind_raw).unwrap_or(FlashcardsTopicKind::Text);
    if !is_safe_topic_folder(folder) {
        return None;
    }
    Some(FlashcardsTopic { folder, name, kind })
}

fn parse_topic_kind(value: &str) -> Option<FlashcardsTopicKind> {
    if value.eq_ignore_ascii_case("IMAGE") || value.eq_ignore_ascii_case("IMG") {
        Some(FlashcardsTopicKind::Image)
    } else if value.eq_ignore_ascii_case("TEXT") || value.eq_ignore_ascii_case("TXT") {
        Some(FlashcardsTopicKind::Text)
    } else {
        None
    }
}

fn parse_card_row(line: &str) -> Option<FlashcardRecord<'_>> {
    let line = line.split('#').next().unwrap_or("").trim();
    if line.is_empty() {
        return None;
    }
    let mut parts = line.split('|');
    let first = parts.next()?.trim();
    if first.eq_ignore_ascii_case("IMG") || first.eq_ignore_ascii_case("IMAGE") {
        let front = parts.next()?.trim();
        let back = parts.next().unwrap_or(front).trim();
        let hint = parts.next().unwrap_or("").trim();
        if !is_safe_image_ref(front) || !is_safe_image_ref(back) {
            return None;
        }
        return Some(FlashcardRecord {
            kind: FlashcardKind::Image,
            front,
            back,
            hint,
        });
    }
    let back = parts.next()?.trim();
    let hint = parts.next().unwrap_or("").trim();
    if first.is_empty() || back.is_empty() {
        return None;
    }
    Some(FlashcardRecord {
        kind: FlashcardKind::Text,
        front: first,
        back,
        hint,
    })
}

fn manifest_declares_flashcards(manifest: &str) -> bool {
    for line in manifest.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if !line.starts_with("id") {
            continue;
        }
        let Some((key, raw)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "id" {
            continue;
        }
        let Some(value) = unquote(raw.trim()) else {
            continue;
        };
        if value == "FLASHCRD" || value == "flashcards" {
            return true;
        }
    }
    false
}

fn unquote(value: &str) -> Option<&str> {
    if value.len() < 2 {
        return None;
    }
    let bytes = value.as_bytes();
    let quote = bytes[0];
    if quote != b'\'' && quote != b'\"' {
        return None;
    }
    if bytes[value.len() - 1] != quote {
        return None;
    }
    Some(&value[1..value.len() - 1])
}

fn push_usize<const N: usize>(text: &mut FlashcardsText<N>, mut value: usize) {
    let mut buf = [0u8; 20];
    let mut len = 0usize;
    if value == 0 {
        text.push_char('0');
        return;
    }
    while value > 0 && len < buf.len() {
        buf[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        text.push_char(buf[len] as char);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_and_image_topics() {
        let index = "TEXTDEMO|Text Flashcards|TEXT\nIMGDEMO|Image Flashcards|IMAGE\n";
        assert_eq!(topic_count(index), 2);
        assert_eq!(topic_folder_at(index, 0), Some("TEXTDEMO"));
        assert_eq!(topic_folder_at(index, 1), Some("IMGDEMO"));
        assert_eq!(topic_kind_at(index, 0), Some(FlashcardsTopicKind::Text));
        assert_eq!(topic_kind_at(index, 1), Some(FlashcardsTopicKind::Image));
    }

    #[test]
    fn parses_text_and_image_cards() {
        let index = "TEXTDEMO|Text Flashcards|TEXT\nIMGDEMO|Image Flashcards|IMAGE\n";
        let text_cards = "Hola|Hello|Greeting\nGracias|Thank you|Polite\n";
        let image_cards = "IMG|IMG/IMG01F.X4B|IMG/IMG01B.X4B|Basics\n";
        assert_eq!(card_count(text_cards), 2);
        assert_eq!(
            card_count_for_topic(text_cards, FlashcardsTopicKind::Text),
            2
        );
        assert_eq!(
            card_count_for_topic(image_cards, FlashcardsTopicKind::Image),
            1
        );
        let screen =
            build_flashcards_screen("id = \"FLASHCRD\"", "", index, image_cards, 1, 0, false);
        assert!(screen.is_image_card());
        assert_eq!(screen.image_file(), "IMG/IMG01F.X4B");
    }

    #[test]
    fn rejects_unsafe_topic_and_image_paths() {
        assert!(!is_safe_topic_folder("../BAD"));
        assert!(is_safe_image_ref("IMG/IMG01F.X4B"));
        assert!(!is_safe_image_ref("../IMG01F.X4B"));
        assert!(!is_safe_image_ref("IMG/IMG01F.PNG"));
    }

    #[test]
    fn builds_topic_listing_screen() {
        let index = "TEXTDEMO|Text Flashcards|TEXT\nIMGDEMO|Image Flashcards|IMAGE\n";
        let manifest = "id = \"FLASHCRD\"\n";
        let screen = build_flashcards_topic_list_screen(manifest, "", index, 1);
        assert_eq!(screen.source, FlashcardsSource::SdLuaScript);
        assert_eq!(screen.topic(), "Choose a topic");
        assert_eq!(screen.topic_count, 2);
        assert!(screen.line1().contains("Image Flashcards"));
        assert!(screen.footer().contains("OK opens"));
    }
}
