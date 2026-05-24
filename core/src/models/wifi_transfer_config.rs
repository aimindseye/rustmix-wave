use heapless::String;
use serde::{Deserialize, Serialize};

pub const WIFI_TRANSFER_ORIGINAL_LABEL: &str = "Original Transfer";
pub const WIFI_TRANSFER_CHUNKED_LABEL: &str = "Chunked Resume";
pub const WIFI_TRANSFER_FCACHE_ROOT: &str = "/FCACHE";
pub const WIFI_TRANSFER_DEFAULT_FCACHE_BOOK_ID: &str = "15D1296A";
pub const WIFI_TRANSFER_DEFAULT_FCACHE_TARGET: &str = "/FCACHE/15D1296A";
pub const WIFI_TRANSFER_TARGET_MAX: usize = 64;
pub const WIFI_TRANSFER_BOOK_ID_LEN: usize = 8;

pub const WIFI_TRANSFER_MIN_CHUNK_SIZE: u16 = 128;
pub const WIFI_TRANSFER_DEFAULT_CHUNK_SIZE: u16 = 256;
pub const WIFI_TRANSFER_MAX_CHUNK_SIZE: u16 = 1536;

pub const WIFI_TRANSFER_MIN_CHUNK_DELAY_MS: u16 = 0;
pub const WIFI_TRANSFER_DEFAULT_CHUNK_DELAY_MS: u16 = 250;
pub const WIFI_TRANSFER_MAX_CHUNK_DELAY_MS: u16 = 2000;

pub const WIFI_TRANSFER_MIN_FILE_DELAY_MS: u16 = 0;
pub const WIFI_TRANSFER_DEFAULT_FILE_DELAY_MS: u16 = 600;
pub const WIFI_TRANSFER_MAX_FILE_DELAY_MS: u16 = 3000;

pub const WIFI_TRANSFER_MIN_RETRY_DELAY_MS: u16 = 0;
pub const WIFI_TRANSFER_DEFAULT_RETRY_DELAY_MS: u16 = 1000;
pub const WIFI_TRANSFER_MAX_RETRY_DELAY_MS: u16 = 5000;

pub const WIFI_TRANSFER_MIN_RETRIES: u8 = 0;
pub const WIFI_TRANSFER_DEFAULT_RETRIES: u8 = 5;
pub const WIFI_TRANSFER_MAX_RETRIES: u8 = 10;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WifiTransferModeModel {
    #[default]
    OriginalTransfer,
    ChunkedResume,
}

impl WifiTransferModeModel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::OriginalTransfer => WIFI_TRANSFER_ORIGINAL_LABEL,
            Self::ChunkedResume => WIFI_TRANSFER_CHUNKED_LABEL,
        }
    }

    pub const fn supports_resume(self) -> bool {
        matches!(self, Self::ChunkedResume)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WifiTransferTargetClassModel {
    Root,
    Folder,
    FcacheRoot,
    FcacheBook,
    #[default]
    Invalid,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WifiTransferFailureClassModel {
    #[default]
    None,
    InvalidTarget,
    NetworkUnavailable,
    ClientDisconnected,
    WriteFailed,
    Timeout,
    RetryExhausted,
    RequestMalformed,
    Unsupported,
    Unknown,
}

impl WifiTransferFailureClassModel {
    pub const fn retryable(self) -> bool {
        matches!(
            self,
            Self::NetworkUnavailable | Self::ClientDisconnected | Self::WriteFailed | Self::Timeout
        )
    }

    pub const fn clear_message(self) -> &'static str {
        match self {
            Self::None => "No failure.",
            Self::InvalidTarget => "Upload target is not allowed.",
            Self::NetworkUnavailable => "Network is unavailable; retry after reconnect.",
            Self::ClientDisconnected => "Client disconnected; resume the upload.",
            Self::WriteFailed => "File write failed; retry or check SD card.",
            Self::Timeout => "Upload timed out; retry or reduce chunk delay.",
            Self::RetryExhausted => "Upload retry limit reached.",
            Self::RequestMalformed => "Upload request is malformed.",
            Self::Unsupported => "Upload mode or request is unsupported.",
            Self::Unknown => "Unknown upload failure.",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiTransferTargetModel {
    pub path: String<WIFI_TRANSFER_TARGET_MAX>,
    pub class: WifiTransferTargetClassModel,
}

impl WifiTransferTargetModel {
    pub fn new(path: &str) -> Option<Self> {
        let normalized = normalize_transfer_target(path)?;
        let class = classify_transfer_target(normalized.as_str());
        if class == WifiTransferTargetClassModel::Invalid {
            return None;
        }
        Some(Self {
            path: normalized,
            class,
        })
    }

    pub fn default_fcache() -> Self {
        Self::new(WIFI_TRANSFER_DEFAULT_FCACHE_TARGET).expect("default FCACHE target is valid")
    }

    pub fn fcache_for_book_id(book_id: &str) -> Option<Self> {
        let book_id = normalize_transfer_book_id(book_id)?;
        let mut path: String<WIFI_TRANSFER_TARGET_MAX> = String::new();
        path.push_str(WIFI_TRANSFER_FCACHE_ROOT).ok()?;
        path.push('/').ok()?;
        path.push_str(book_id.as_str()).ok()?;
        Self::new(path.as_str())
    }

    pub const fn is_fcache_book(&self) -> bool {
        matches!(self.class, WifiTransferTargetClassModel::FcacheBook)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiTransferConfigModel {
    pub mode: WifiTransferModeModel,
    pub target: WifiTransferTargetModel,
    pub chunk_size: u16,
    pub chunk_delay_ms: u16,
    pub file_delay_ms: u16,
    pub retry_delay_ms: u16,
    pub max_retries: u8,
}

impl Default for WifiTransferConfigModel {
    fn default() -> Self {
        Self::chunked_resume_default()
    }
}

impl WifiTransferConfigModel {
    pub fn original_transfer_default() -> Self {
        Self {
            mode: WifiTransferModeModel::OriginalTransfer,
            target: WifiTransferTargetModel::new("/").expect("root target is valid"),
            chunk_size: WIFI_TRANSFER_DEFAULT_CHUNK_SIZE,
            chunk_delay_ms: WIFI_TRANSFER_DEFAULT_CHUNK_DELAY_MS,
            file_delay_ms: WIFI_TRANSFER_DEFAULT_FILE_DELAY_MS,
            retry_delay_ms: WIFI_TRANSFER_DEFAULT_RETRY_DELAY_MS,
            max_retries: WIFI_TRANSFER_DEFAULT_RETRIES,
        }
    }

    pub fn chunked_resume_default() -> Self {
        Self {
            mode: WifiTransferModeModel::ChunkedResume,
            target: WifiTransferTargetModel::default_fcache(),
            chunk_size: WIFI_TRANSFER_DEFAULT_CHUNK_SIZE,
            chunk_delay_ms: WIFI_TRANSFER_DEFAULT_CHUNK_DELAY_MS,
            file_delay_ms: WIFI_TRANSFER_DEFAULT_FILE_DELAY_MS,
            retry_delay_ms: WIFI_TRANSFER_DEFAULT_RETRY_DELAY_MS,
            max_retries: WIFI_TRANSFER_DEFAULT_RETRIES,
        }
    }

    pub fn sanitized(
        mode: WifiTransferModeModel,
        target: &str,
        chunk_size: u16,
        chunk_delay_ms: u16,
        file_delay_ms: u16,
        retry_delay_ms: u16,
        max_retries: u8,
    ) -> Self {
        let target = WifiTransferTargetModel::new(target)
            .unwrap_or_else(WifiTransferTargetModel::default_fcache);
        Self {
            mode,
            target,
            chunk_size: clamp_transfer_chunk_size(chunk_size),
            chunk_delay_ms: clamp_transfer_chunk_delay_ms(chunk_delay_ms),
            file_delay_ms: clamp_transfer_file_delay_ms(file_delay_ms),
            retry_delay_ms: clamp_transfer_retry_delay_ms(retry_delay_ms),
            max_retries: clamp_transfer_max_retries(max_retries),
        }
    }

    pub const fn stores_wifi_password(&self) -> bool {
        false
    }

    pub const fn exposes_wifi_password(&self) -> bool {
        false
    }

    pub const fn is_chunked_fcache_upload(&self) -> bool {
        self.mode.supports_resume() && self.target.is_fcache_book()
    }
}

pub fn clamp_transfer_chunk_size(value: u16) -> u16 {
    value.clamp(WIFI_TRANSFER_MIN_CHUNK_SIZE, WIFI_TRANSFER_MAX_CHUNK_SIZE)
}

pub fn clamp_transfer_chunk_delay_ms(value: u16) -> u16 {
    value.clamp(
        WIFI_TRANSFER_MIN_CHUNK_DELAY_MS,
        WIFI_TRANSFER_MAX_CHUNK_DELAY_MS,
    )
}

pub fn clamp_transfer_file_delay_ms(value: u16) -> u16 {
    value.clamp(
        WIFI_TRANSFER_MIN_FILE_DELAY_MS,
        WIFI_TRANSFER_MAX_FILE_DELAY_MS,
    )
}

pub fn clamp_transfer_retry_delay_ms(value: u16) -> u16 {
    value.clamp(
        WIFI_TRANSFER_MIN_RETRY_DELAY_MS,
        WIFI_TRANSFER_MAX_RETRY_DELAY_MS,
    )
}

pub fn clamp_transfer_max_retries(value: u8) -> u8 {
    value.clamp(WIFI_TRANSFER_MIN_RETRIES, WIFI_TRANSFER_MAX_RETRIES)
}

pub fn normalize_transfer_book_id(book_id: &str) -> Option<String<WIFI_TRANSFER_BOOK_ID_LEN>> {
    let trimmed = book_id.trim();
    if trimmed.len() != WIFI_TRANSFER_BOOK_ID_LEN {
        return None;
    }
    let mut out = String::new();
    for b in trimmed.bytes() {
        if !b.is_ascii_hexdigit() {
            return None;
        }
        out.push((b as char).to_ascii_uppercase()).ok()?;
    }
    Some(out)
}

pub fn is_valid_transfer_book_id(book_id: &str) -> bool {
    normalize_transfer_book_id(book_id).is_some()
}

pub fn normalize_transfer_target(path: &str) -> Option<String<WIFI_TRANSFER_TARGET_MAX>> {
    let trimmed = path.trim();
    if trimmed.is_empty() || has_transfer_path_traversal(trimmed) {
        return None;
    }

    let mut out = String::new();
    if !trimmed.starts_with('/') {
        out.push('/').ok()?;
    }

    let mut previous_slash = out.as_str().ends_with('/');
    for ch in trimmed.chars() {
        let ch = if ch == '\\' { '/' } else { ch };
        if ch == '/' {
            if !previous_slash {
                out.push('/').ok()?;
            }
            previous_slash = true;
            continue;
        }
        if !is_allowed_transfer_path_char(ch) {
            return None;
        }
        out.push(ch).ok()?;
        previous_slash = false;
    }

    while out.len() > 1 && out.ends_with('/') {
        out.pop();
    }
    Some(out)
}

pub fn classify_transfer_target(path: &str) -> WifiTransferTargetClassModel {
    let Some(normalized) = normalize_transfer_target(path) else {
        return WifiTransferTargetClassModel::Invalid;
    };
    let value = normalized.as_str();
    if value == "/" {
        return WifiTransferTargetClassModel::Root;
    }
    if value == WIFI_TRANSFER_FCACHE_ROOT {
        return WifiTransferTargetClassModel::FcacheRoot;
    }
    if let Some(id) = value.strip_prefix("/FCACHE/") {
        return if is_valid_transfer_book_id(id) {
            WifiTransferTargetClassModel::FcacheBook
        } else {
            WifiTransferTargetClassModel::Invalid
        };
    }
    WifiTransferTargetClassModel::Folder
}

pub fn is_fcache_upload_target(path: &str) -> bool {
    classify_transfer_target(path) == WifiTransferTargetClassModel::FcacheBook
}

pub fn classify_transfer_failure(code_or_message: &str) -> WifiTransferFailureClassModel {
    let mut lower: String<96> = String::new();
    for ch in code_or_message.chars() {
        if lower.push(ch.to_ascii_lowercase()).is_err() {
            break;
        }
    }
    let value = lower.as_str();
    if value.is_empty() || value == "ok" || value == "none" {
        WifiTransferFailureClassModel::None
    } else if value.contains("target") || value.contains("path") || value.contains("traversal") {
        WifiTransferFailureClassModel::InvalidTarget
    } else if value.contains("network") || value.contains("wifi") || value.contains("connect") {
        WifiTransferFailureClassModel::NetworkUnavailable
    } else if value.contains("disconnect") || value.contains("abort") || value.contains("stop") {
        WifiTransferFailureClassModel::ClientDisconnected
    } else if value.contains("write") || value.contains("sd") || value.contains("disk") {
        WifiTransferFailureClassModel::WriteFailed
    } else if value.contains("timeout") || value.contains("timed out") {
        WifiTransferFailureClassModel::Timeout
    } else if value.contains("retry") || value.contains("exhaust") {
        WifiTransferFailureClassModel::RetryExhausted
    } else if value.contains("malformed") || value.contains("bad request") {
        WifiTransferFailureClassModel::RequestMalformed
    } else if value.contains("unsupported") {
        WifiTransferFailureClassModel::Unsupported
    } else {
        WifiTransferFailureClassModel::Unknown
    }
}

pub fn is_path_traversal(path: &str) -> bool {
    has_transfer_path_traversal(path)
}

pub fn classify_target(path: &str) -> WifiTransferTargetClassModel {
    classify_transfer_target(path)
}

pub fn has_transfer_path_traversal(path: &str) -> bool {
    path.split(['/', '\\']).any(|part| part == "..")
}

fn is_allowed_transfer_path_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(
            ch,
            '/' | '_' | '-' | '.' | '~' | ' ' | '(' | ')' | '[' | ']' | '{' | '}'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_fcache_target_path_matches_large_cache_upload_note() {
        let cfg = WifiTransferConfigModel::chunked_resume_default();
        assert_eq!(cfg.mode, WifiTransferModeModel::ChunkedResume);
        assert_eq!(
            cfg.target.path.as_str(),
            WIFI_TRANSFER_DEFAULT_FCACHE_TARGET
        );
        assert!(cfg.is_chunked_fcache_upload());
    }

    #[test]
    fn chunk_size_is_clamped_to_browser_safe_bounds() {
        assert_eq!(clamp_transfer_chunk_size(1), WIFI_TRANSFER_MIN_CHUNK_SIZE);
        assert_eq!(clamp_transfer_chunk_size(256), 256);
        assert_eq!(
            clamp_transfer_chunk_size(9999),
            WIFI_TRANSFER_MAX_CHUNK_SIZE
        );
    }

    #[test]
    fn retry_delay_is_clamped_to_safe_bounds() {
        assert_eq!(
            clamp_transfer_retry_delay_ms(0),
            WIFI_TRANSFER_MIN_RETRY_DELAY_MS
        );
        assert_eq!(clamp_transfer_retry_delay_ms(1000), 1000);
        assert_eq!(
            clamp_transfer_retry_delay_ms(9999),
            WIFI_TRANSFER_MAX_RETRY_DELAY_MS
        );
    }

    #[test]
    fn chunk_and_file_delays_are_clamped() {
        assert_eq!(
            clamp_transfer_chunk_delay_ms(9999),
            WIFI_TRANSFER_MAX_CHUNK_DELAY_MS
        );
        assert_eq!(
            clamp_transfer_file_delay_ms(9999),
            WIFI_TRANSFER_MAX_FILE_DELAY_MS
        );
    }

    #[test]
    fn max_retries_are_clamped() {
        assert_eq!(clamp_transfer_max_retries(99), WIFI_TRANSFER_MAX_RETRIES);
    }

    #[test]
    fn transfer_config_does_not_represent_wifi_credentials() {
        let cfg = WifiTransferConfigModel::default();
        assert!(!cfg.stores_wifi_password());
        assert!(!cfg.exposes_wifi_password());
    }

    #[test]
    fn traversal_target_is_rejected() {
        assert!(WifiTransferTargetModel::new("/FCACHE/../SECRET").is_none());
        assert!(WifiTransferTargetModel::new("../FCACHE/15D1296A").is_none());
    }

    #[test]
    fn fcache_book_target_is_accepted_and_normalized() {
        let target = WifiTransferTargetModel::new("FCACHE/15d1296a").unwrap();
        assert_eq!(target.path.as_str(), "/FCACHE/15d1296a");
        assert_eq!(target.class, WifiTransferTargetClassModel::FcacheBook);
        assert!(is_fcache_upload_target("/FCACHE/15D1296A"));
    }

    #[test]
    fn fcache_target_builder_uppercases_book_id() {
        let target = WifiTransferTargetModel::fcache_for_book_id("15d1296a").unwrap();
        assert_eq!(target.path.as_str(), "/FCACHE/15D1296A");
    }

    #[test]
    fn invalid_fcache_book_id_is_rejected() {
        assert!(WifiTransferTargetModel::new("/FCACHE/NOTABOOK").is_none());
        assert!(WifiTransferTargetModel::fcache_for_book_id("not-book").is_none());
    }

    #[test]
    fn failure_classification_is_clear_and_retryable() {
        let timeout = classify_transfer_failure("upload timeout");
        assert_eq!(timeout, WifiTransferFailureClassModel::Timeout);
        assert!(timeout.retryable());
        assert_eq!(
            classify_transfer_failure("target traversal"),
            WifiTransferFailureClassModel::InvalidTarget
        );
    }
}
