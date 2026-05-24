#![allow(dead_code)]

/// Rustmix-owned storage boundary metadata for the Xteink X4 target.
///
/// The current implementation intentionally extracts only ownership metadata and typed helpers.
/// Physical SD/SPI initialization, volume mounting, file IO, EPUB cache IO,
/// bookmark persistence, progress persistence, and theme persistence still live
/// in the imported `target-xteink-x4/src/rustmix_x4` runtime.
pub struct RustmixStorageBoundary;

/// High-level storage resource groups that Rustmix must preserve when it later
/// takes over storage orchestration from the imported X4 runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustmixStorageResourceKind {
    StateDirectory,
    ProgressRecord,
    BookmarkRecord,
    BookmarkIndex,
    ThemeRecord,
    MetadataRecord,
    EpubCache,
    LibraryContent,
}

/// Static descriptor for a known storage path or naming convention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustmixStorageResource {
    pub kind: RustmixStorageResourceKind,
    pub directory: &'static str,
    pub pattern: &'static str,
    pub owner: &'static str,
}

impl RustmixStorageBoundary {
    pub const STORAGE_BOUNDARY_MARKER: &'static str = "x4-storage-boundary-ok";

    /// Current source of truth for behavior.
    pub const IMPLEMENTATION_OWNER: &'static str = "Rustmix-owned X4 runtime";

    /// The current implementation only extracts metadata/helpers. Runtime behavior is still imported.
    pub const BEHAVIOR_MOVED_TO_BOUNDARY: bool = false;
    pub const PHYSICAL_SD_INIT_MOVED_TO_BOUNDARY: bool = false;
    pub const FILE_IO_MOVED_TO_BOUNDARY: bool = false;
    pub const EPUB_CACHE_IO_MOVED_TO_BOUNDARY: bool = false;

    /// X4 storage pin facts preserved for future Rustmix-owned HAL extraction.
    pub const SD_CS_GPIO: u8 = 12;
    pub const SPI_SCLK_GPIO: u8 = 8;
    pub const SPI_MOSI_GPIO: u8 = 10;
    pub const SPI_MISO_GPIO: u8 = 7;
    pub const SHARES_DISPLAY_SPI_BUS: bool = true;

    /// Rustmix state layout conventions used by the current X4 reader path.
    pub const STATE_DIR: &'static str = "state";
    pub const BOOKMARK_INDEX_FILE: &'static str = "BMIDX.TXT";
    pub const PROGRESS_EXT: &'static str = "PRG";
    pub const BOOKMARK_EXT: &'static str = "BKM";
    pub const THEME_EXT: &'static str = "THM";
    pub const METADATA_EXT: &'static str = "MTA";

    /// EPUB cache remains owned by the imported X4/smol-epub path in The current implementation
    pub const EPUB_CACHE_OWNER: &'static str = "target-xteink-x4/src/rustmix_x4 + vendor/smol-epub";
    pub const EPUB_CACHE_MOVED_TO_BOUNDARY: bool = false;

    pub const RESOURCES: &'static [RustmixStorageResource] = &[
        RustmixStorageResource {
            kind: RustmixStorageResourceKind::StateDirectory,
            directory: "state",
            pattern: "state/",
            owner: "Rustmix boundary metadata; imported X4 runtime behavior",
        },
        RustmixStorageResource {
            kind: RustmixStorageResourceKind::ProgressRecord,
            directory: "state",
            pattern: "state/<BOOKID>.PRG",
            owner: "imported X4 reader progress behavior",
        },
        RustmixStorageResource {
            kind: RustmixStorageResourceKind::BookmarkRecord,
            directory: "state",
            pattern: "state/<BOOKID>.BKM",
            owner: "imported X4 reader bookmark behavior",
        },
        RustmixStorageResource {
            kind: RustmixStorageResourceKind::BookmarkIndex,
            directory: "state",
            pattern: "state/BMIDX.TXT",
            owner: "imported X4 reader bookmark index behavior",
        },
        RustmixStorageResource {
            kind: RustmixStorageResourceKind::ThemeRecord,
            directory: "state",
            pattern: "state/<BOOKID>.THM",
            owner: "imported X4 reader theme behavior",
        },
        RustmixStorageResource {
            kind: RustmixStorageResourceKind::MetadataRecord,
            directory: "state",
            pattern: "state/<BOOKID>.MTA",
            owner: "Rustmix typed metadata convention",
        },
        RustmixStorageResource {
            kind: RustmixStorageResourceKind::EpubCache,
            directory: "imported-runtime-managed",
            pattern: "smol-epub chapter/cache resources",
            owner: "target-xteink-x4/src/rustmix_x4 + vendor/smol-epub",
        },
    ];

    #[cfg(target_arch = "riscv32")]
    pub fn emit_boot_marker() {
        esp_println::println!("{}", Self::STORAGE_BOUNDARY_MARKER);
    }

    #[cfg(not(target_arch = "riscv32"))]
    pub fn emit_boot_marker() {}

    pub const fn implementation_owner() -> &'static str {
        Self::IMPLEMENTATION_OWNER
    }

    pub const fn state_dir() -> &'static str {
        Self::STATE_DIR
    }

    pub const fn bookmark_index_file() -> &'static str {
        Self::BOOKMARK_INDEX_FILE
    }

    pub const fn resources() -> &'static [RustmixStorageResource] {
        Self::RESOURCES
    }

    pub fn is_state_extension(ext: &[u8]) -> bool {
        eq_ignore_ascii_case(ext, Self::PROGRESS_EXT.as_bytes())
            || eq_ignore_ascii_case(ext, Self::BOOKMARK_EXT.as_bytes())
            || eq_ignore_ascii_case(ext, Self::THEME_EXT.as_bytes())
            || eq_ignore_ascii_case(ext, Self::METADATA_EXT.as_bytes())
    }

    pub fn is_reserved_state_file(name: &[u8]) -> bool {
        eq_ignore_ascii_case(name, Self::BOOKMARK_INDEX_FILE.as_bytes())
    }

    /// Validate the 8-character uppercase hex-ish book id base used by the current
    /// 8.3-safe state record convention. This helper is intentionally pure and
    /// allocation-free so it can later be reused by no_std storage code.
    pub fn is_valid_book_id_base(base: &[u8]) -> bool {
        if base.len() != 8 {
            return false;
        }

        let mut i = 0usize;
        while i < base.len() {
            let b = base[i];
            let is_digit = b.is_ascii_digit();
            let is_upper_hex = (b'A'..=b'F').contains(&b);
            let is_lower_hex = (b'a'..=b'f').contains(&b);
            if !(is_digit || is_upper_hex || is_lower_hex) {
                return false;
            }
            i += 1;
        }

        true
    }
}

fn eq_ignore_ascii_case(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut i = 0usize;
    while i < left.len() {
        if to_ascii_upper(left[i]) != to_ascii_upper(right[i]) {
            return false;
        }
        i += 1;
    }

    true
}

const fn to_ascii_upper(b: u8) -> u8 {
    if b >= b'a' && b <= b'z' { b - 32 } else { b }
}
