use core::fmt::Write;

use heapless::String;
use serde::{Deserialize, Serialize};

pub const FNV1A32_OFFSET: u32 = 0x811c_9dc5;
pub const FNV1A32_PRIME: u32 = 0x0100_0193;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BookIdScheme {
    /// Compatibility with the proving-ground path-derived identity.
    /// Do not use for future sync identity.
    PathFnv1a32LegacyV1,
    /// X4-safe early content identity: file size + streamed sample bytes.
    ContentSampleFnv1a32V1,
    /// Reserved for later once full-file hashing cost is measured on X4.
    ContentSha256V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookId {
    pub scheme: BookIdScheme,
    pub hex: String<64>,
}

impl BookId {
    pub fn new(scheme: BookIdScheme, hex: &str) -> Self {
        let mut out = String::new();
        for ch in hex.chars().take(out.capacity()) {
            let _ = out.push(ch.to_ascii_lowercase());
        }
        Self { scheme, hex: out }
    }

    pub fn from_legacy_path(path: &str) -> Self {
        Self::from_u32(BookIdScheme::PathFnv1a32LegacyV1, fnv1a32(path.as_bytes()))
    }

    pub fn from_content_sample(file_size_bytes: u64, sample: &[u8]) -> Self {
        let mut hash = FNV1A32_OFFSET;
        for b in file_size_bytes.to_le_bytes() {
            hash = fnv1a32_update(hash, b);
        }
        for &b in sample {
            hash = fnv1a32_update(hash, b);
        }
        Self::from_u32(BookIdScheme::ContentSampleFnv1a32V1, hash)
    }

    pub fn from_u32(scheme: BookIdScheme, value: u32) -> Self {
        let mut hex = String::new();
        let _ = write!(hex, "{value:08x}");
        Self { scheme, hex }
    }

    pub fn as_hex(&self) -> &str {
        self.hex.as_str()
    }

    pub fn compat_hex8_upper(&self) -> String<8> {
        let mut out = String::new();
        for ch in self.hex.chars().take(8) {
            let _ = out.push(ch.to_ascii_uppercase());
        }
        out
    }

    pub fn is_content_based(&self) -> bool {
        matches!(
            self.scheme,
            BookIdScheme::ContentSampleFnv1a32V1 | BookIdScheme::ContentSha256V1
        )
    }
}

pub fn fnv1a32(bytes: &[u8]) -> u32 {
    let mut hash = FNV1A32_OFFSET;
    for &b in bytes {
        hash = fnv1a32_update(hash, b);
    }
    hash
}

const fn fnv1a32_update(hash: u32, byte: u8) -> u32 {
    (hash ^ byte as u32).wrapping_mul(FNV1A32_PRIME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_known_value_for_empty() {
        assert_eq!(fnv1a32(&[]), FNV1A32_OFFSET);
    }

    #[test]
    fn legacy_path_id_formats_as_hex8() {
        let id = BookId::from_legacy_path("THEHO~26.EPU");
        assert_eq!(id.hex.len(), 8);
        assert_eq!(id.compat_hex8_upper().len(), 8);
        assert!(!id.is_content_based());
    }

    #[test]
    fn content_sample_id_includes_file_size() {
        let a = BookId::from_content_sample(100, b"same-sample");
        let b = BookId::from_content_sample(101, b"same-sample");
        assert_ne!(a, b);
        assert!(a.is_content_based());
    }
}
