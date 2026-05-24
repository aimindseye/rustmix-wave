//! Adapter contracts between Rustmix-owned pure models and the active Rustmix-owned X4 runtime.
//!
//! This module is intentionally behavior-neutral. It documents and exposes thin mapping
//! constants/helpers used to align Rustmix core ownership with the current runtime without
//! moving SD, SPI, display, Wi-Fi, input-scan, or refresh behavior.

pub const ACTIVE_RUNTIME_NAME: &str = "target-xteink-x4/src/rustmix_x4";
pub const ACTIVE_RUNTIME_OWNS_HARDWARE_BEHAVIOR: bool = true;
pub const RUSTMIX_ADAPTER_MOVES_HARDWARE_BEHAVIOR: bool = false;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct X4RuntimeAdapterContracts;

impl X4RuntimeAdapterContracts {
    pub const fn active_runtime_name(&self) -> &'static str {
        ACTIVE_RUNTIME_NAME
    }

    pub const fn hardware_behavior_moved(&self) -> bool {
        RUSTMIX_ADAPTER_MOVES_HARDWARE_BEHAVIOR
    }

    pub const fn storage_paths(&self) -> &'static [StoragePathRuntimeContract] {
        STORAGE_PATH_RUNTIME_CONTRACTS
    }

    pub const fn display_regions(&self) -> &'static [DisplayRegionRuntimeContract] {
        DISPLAY_REGION_RUNTIME_CONTRACTS
    }

    pub const fn wifi_transfer_defaults(&self) -> WifiTransferRuntimeDefaults {
        WifiTransferRuntimeDefaults::current()
    }
}

pub trait RuntimeAdapterContract {
    fn active_runtime_name(&self) -> &'static str;
    fn hardware_behavior_moved(&self) -> bool;
}

impl RuntimeAdapterContract for X4RuntimeAdapterContracts {
    fn active_runtime_name(&self) -> &'static str {
        X4RuntimeAdapterContracts::active_runtime_name(self)
    }

    fn hardware_behavior_moved(&self) -> bool {
        X4RuntimeAdapterContracts::hardware_behavior_moved(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoragePathContractKind {
    LibraryRootCurrent,
    LibraryRootBooks,
    StateDirectory,
    ProgressFileTemplate,
    BookmarkFileTemplate,
    BookmarkIndexFile,
    PreparedCacheRoot,
    PreparedCacheBookTemplate,
    SettingsFile,
    TitleCacheFile,
    SleepRoot,
    SleepDailyRoot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoragePathRuntimeContract {
    pub kind: StoragePathContractKind,
    pub core_owner: &'static str,
    pub runtime_path: &'static str,
    pub behavior_owner: &'static str,
}

pub const STORAGE_PATH_RUNTIME_CONTRACTS: &[StoragePathRuntimeContract] = &[
    StoragePathRuntimeContract {
        kind: StoragePathContractKind::LibraryRootCurrent,
        core_owner: "StoragePathModel::library_path_current",
        runtime_path: "/",
        behavior_owner: ACTIVE_RUNTIME_NAME,
    },
    StoragePathRuntimeContract {
        kind: StoragePathContractKind::LibraryRootBooks,
        core_owner: "StoragePathModel::library_path_books",
        runtime_path: "/books",
        behavior_owner: ACTIVE_RUNTIME_NAME,
    },
    StoragePathRuntimeContract {
        kind: StoragePathContractKind::StateDirectory,
        core_owner: "StoragePathModel::state_dir",
        runtime_path: "/state",
        behavior_owner: ACTIVE_RUNTIME_NAME,
    },
    StoragePathRuntimeContract {
        kind: StoragePathContractKind::ProgressFileTemplate,
        core_owner: "ReaderProgressModel path helper",
        runtime_path: "/state/<BOOKID>.PRG",
        behavior_owner: ACTIVE_RUNTIME_NAME,
    },
    StoragePathRuntimeContract {
        kind: StoragePathContractKind::BookmarkFileTemplate,
        core_owner: "BookmarkEntryModel path helper",
        runtime_path: "/state/<BOOKID>.BKM",
        behavior_owner: ACTIVE_RUNTIME_NAME,
    },
    StoragePathRuntimeContract {
        kind: StoragePathContractKind::BookmarkIndexFile,
        core_owner: "BookmarkIndexEntryModel path helper",
        runtime_path: "/state/BMIDX.TXT",
        behavior_owner: ACTIVE_RUNTIME_NAME,
    },
    StoragePathRuntimeContract {
        kind: StoragePathContractKind::PreparedCacheRoot,
        core_owner: "PreparedCachePathModel::root",
        runtime_path: "/FCACHE",
        behavior_owner: ACTIVE_RUNTIME_NAME,
    },
    StoragePathRuntimeContract {
        kind: StoragePathContractKind::PreparedCacheBookTemplate,
        core_owner: "PreparedCachePathModel::book",
        runtime_path: "/FCACHE/<BOOKID>",
        behavior_owner: ACTIVE_RUNTIME_NAME,
    },
    StoragePathRuntimeContract {
        kind: StoragePathContractKind::SettingsFile,
        core_owner: "Settings state model path",
        runtime_path: "/RUSTMIX/SETTINGS.TXT",
        behavior_owner: ACTIVE_RUNTIME_NAME,
    },
    StoragePathRuntimeContract {
        kind: StoragePathContractKind::TitleCacheFile,
        core_owner: "TitleCacheRecordModel path",
        runtime_path: "/RUSTMIX/TITLES.BIN",
        behavior_owner: ACTIVE_RUNTIME_NAME,
    },
    StoragePathRuntimeContract {
        kind: StoragePathContractKind::SleepRoot,
        core_owner: "SleepImageModeModel path",
        runtime_path: "/RUSTMIX/SLEEP",
        behavior_owner: ACTIVE_RUNTIME_NAME,
    },
    StoragePathRuntimeContract {
        kind: StoragePathContractKind::SleepDailyRoot,
        core_owner: "SleepImageModeModel random image path",
        runtime_path: "/RUSTMIX/SLEEP/SLEEPnn.BMP",
        behavior_owner: ACTIVE_RUNTIME_NAME,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreInputSemanticActionContract {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    Menu,
    Power,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X4ButtonEventContract {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    Menu,
    Power,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterAppContextContract {
    HomeCategoryDashboard,
    FilesLibrary,
    Reader,
    Settings,
    DateTime,
    WifiTransfer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReaderPageNavigationContract {
    Forward,
    Backward,
    ExitReader,
    OpenMenu,
    None,
}

pub const fn x4_button_for_semantic_action(
    action: CoreInputSemanticActionContract,
) -> X4ButtonEventContract {
    match action {
        CoreInputSemanticActionContract::Up => X4ButtonEventContract::Up,
        CoreInputSemanticActionContract::Down => X4ButtonEventContract::Down,
        CoreInputSemanticActionContract::Left => X4ButtonEventContract::Left,
        CoreInputSemanticActionContract::Right => X4ButtonEventContract::Right,
        CoreInputSemanticActionContract::Select => X4ButtonEventContract::Select,
        CoreInputSemanticActionContract::Back => X4ButtonEventContract::Back,
        CoreInputSemanticActionContract::Menu => X4ButtonEventContract::Menu,
        CoreInputSemanticActionContract::Power => X4ButtonEventContract::Power,
        CoreInputSemanticActionContract::Unknown => X4ButtonEventContract::Unknown,
    }
}

pub const fn input_action_is_repeatable(
    context: AdapterAppContextContract,
    action: CoreInputSemanticActionContract,
) -> bool {
    match context {
        AdapterAppContextContract::Reader => matches!(
            action,
            CoreInputSemanticActionContract::Up
                | CoreInputSemanticActionContract::Down
                | CoreInputSemanticActionContract::Left
                | CoreInputSemanticActionContract::Right
        ),
        AdapterAppContextContract::HomeCategoryDashboard
        | AdapterAppContextContract::FilesLibrary
        | AdapterAppContextContract::Settings => matches!(
            action,
            CoreInputSemanticActionContract::Up
                | CoreInputSemanticActionContract::Down
                | CoreInputSemanticActionContract::Left
                | CoreInputSemanticActionContract::Right
        ),
        AdapterAppContextContract::DateTime | AdapterAppContextContract::WifiTransfer => false,
    }
}

pub const fn safe_back_action(context: AdapterAppContextContract) -> bool {
    match context {
        AdapterAppContextContract::HomeCategoryDashboard => false,
        AdapterAppContextContract::FilesLibrary
        | AdapterAppContextContract::Reader
        | AdapterAppContextContract::Settings
        | AdapterAppContextContract::DateTime
        | AdapterAppContextContract::WifiTransfer => true,
    }
}

pub const fn reader_page_navigation_for_action(
    action: CoreInputSemanticActionContract,
) -> ReaderPageNavigationContract {
    match action {
        CoreInputSemanticActionContract::Down | CoreInputSemanticActionContract::Right => {
            ReaderPageNavigationContract::Forward
        }
        CoreInputSemanticActionContract::Up | CoreInputSemanticActionContract::Left => {
            ReaderPageNavigationContract::Backward
        }
        CoreInputSemanticActionContract::Back => ReaderPageNavigationContract::ExitReader,
        CoreInputSemanticActionContract::Menu => ReaderPageNavigationContract::OpenMenu,
        _ => ReaderPageNavigationContract::None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayRuntimeRegionKind {
    Screen,
    Header,
    Body,
    FooterStatus,
    BatteryStatus,
    ReaderProgressStatus,
    PopupMessage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayRegionRuntimeContract {
    pub kind: DisplayRuntimeRegionKind,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub core_owner: &'static str,
    pub runtime_role: &'static str,
}

impl DisplayRegionRuntimeContract {
    pub const fn right(self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub const fn bottom(self) -> u16 {
        self.y.saturating_add(self.height)
    }

    pub const fn is_within_screen(self) -> bool {
        self.right() <= X4_SCREEN_WIDTH && self.bottom() <= X4_SCREEN_HEIGHT
    }

    pub const fn intersects(self, other: Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }
}

pub const X4_SCREEN_WIDTH: u16 = 800;
pub const X4_SCREEN_HEIGHT: u16 = 480;
pub const X4_LOGICAL_ROTATION_DEGREES: u16 = 270;
pub const X4_DISPLAY_ORIENTATION: &str = "portrait-rotated-270";

pub const DISPLAY_REGION_RUNTIME_CONTRACTS: &[DisplayRegionRuntimeContract] = &[
    DisplayRegionRuntimeContract {
        kind: DisplayRuntimeRegionKind::Screen,
        x: 0,
        y: 0,
        width: X4_SCREEN_WIDTH,
        height: X4_SCREEN_HEIGHT,
        core_owner: "DisplaySizeModel",
        runtime_role: "full logical screen",
    },
    DisplayRegionRuntimeContract {
        kind: DisplayRuntimeRegionKind::Header,
        x: 0,
        y: 0,
        width: X4_SCREEN_WIDTH,
        height: 28,
        core_owner: "DisplayChromeLayoutModel::header",
        runtime_role: "top chrome/header",
    },
    DisplayRegionRuntimeContract {
        kind: DisplayRuntimeRegionKind::Body,
        x: 0,
        y: 28,
        width: X4_SCREEN_WIDTH,
        height: 424,
        core_owner: "DisplayChromeLayoutModel::body",
        runtime_role: "reader/list/body drawing area",
    },
    DisplayRegionRuntimeContract {
        kind: DisplayRuntimeRegionKind::FooterStatus,
        x: 0,
        y: 452,
        width: X4_SCREEN_WIDTH,
        height: 28,
        core_owner: "DisplayChromeLayoutModel::footer",
        runtime_role: "bottom status/footer chrome",
    },
    DisplayRegionRuntimeContract {
        kind: DisplayRuntimeRegionKind::BatteryStatus,
        x: 704,
        y: 0,
        width: 96,
        height: 28,
        core_owner: "DisplayChromeLayoutModel::battery_status",
        runtime_role: "right header battery/status area",
    },
    DisplayRegionRuntimeContract {
        kind: DisplayRuntimeRegionKind::ReaderProgressStatus,
        x: 0,
        y: 452,
        width: X4_SCREEN_WIDTH,
        height: 28,
        core_owner: "DisplayChromeLayoutModel::reader_progress_status",
        runtime_role: "reader progress/footer status",
    },
    DisplayRegionRuntimeContract {
        kind: DisplayRuntimeRegionKind::PopupMessage,
        x: 16,
        y: 48,
        width: 768,
        height: 96,
        core_owner: "DisplayDiagnosticPlacementModel::popup_message",
        runtime_role: "body popup/diagnostic notice",
    },
];

pub const fn display_region_for_kind(
    kind: DisplayRuntimeRegionKind,
) -> Option<DisplayRegionRuntimeContract> {
    let mut idx = 0;
    while idx < DISPLAY_REGION_RUNTIME_CONTRACTS.len() {
        let region = DISPLAY_REGION_RUNTIME_CONTRACTS[idx];
        if region.kind as u8 == kind as u8 {
            return Some(region);
        }
        idx += 1;
    }
    None
}

pub const fn cache_diagnostic_region_is_header_safe() -> bool {
    let Some(header) = display_region_for_kind(DisplayRuntimeRegionKind::Header) else {
        return false;
    };
    let Some(popup) = display_region_for_kind(DisplayRuntimeRegionKind::PopupMessage) else {
        return false;
    };
    !popup.intersects(header)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiTransferRuntimeModeContract {
    OriginalTransfer,
    ChunkedResume,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WifiTransferRuntimeDefaults {
    pub original_transfer_label: &'static str,
    pub chunked_resume_label: &'static str,
    pub default_target_folder: &'static str,
    pub default_chunk_size: u16,
    pub min_chunk_size: u16,
    pub max_chunk_size: u16,
    pub default_chunk_delay_ms: u16,
    pub default_file_delay_ms: u16,
    pub large_fcache_note: &'static str,
}

impl WifiTransferRuntimeDefaults {
    pub const fn current() -> Self {
        Self {
            original_transfer_label: "Original Transfer",
            chunked_resume_label: "Chunked Resume",
            default_target_folder: "/FCACHE/15D1296A",
            default_chunk_size: 256,
            min_chunk_size: 128,
            max_chunk_size: 1536,
            default_chunk_delay_ms: 250,
            default_file_delay_ms: 600,
            large_fcache_note: "large prepared cache folders such as /FCACHE/15D1296A",
        }
    }

    pub const fn clamp_chunk_size(self, value: u16) -> u16 {
        if value < self.min_chunk_size {
            self.min_chunk_size
        } else if value > self.max_chunk_size {
            self.max_chunk_size
        } else {
            value
        }
    }
}

pub const fn wifi_transfer_mode_label(mode: WifiTransferRuntimeModeContract) -> &'static str {
    match mode {
        WifiTransferRuntimeModeContract::OriginalTransfer => "Original Transfer",
        WifiTransferRuntimeModeContract::ChunkedResume => "Chunked Resume",
    }
}
