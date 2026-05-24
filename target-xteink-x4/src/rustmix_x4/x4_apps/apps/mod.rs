// AppId definition and re-exports from kernel::app.
//
// Active X4 home/app-manager/Wi-Fi code has moved to target-xteink-x4/src/rustmix_x4.
// X4 apps left here are compatibility modules still being migrated.

pub mod files;
pub mod reader;
pub mod reader_state;
pub mod settings;
pub mod widgets;

use crate::rustmix_x4::x4_kernel::kernel::app::AppIdType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppId {
    Home,
    Files,
    Reader,
    Settings,
    // Special-mode ids are retained for the Rustmix-owned target app manager.
    BiscuitWifi,
    Upload,
    TimeSync,
    WifiScan,
}

impl AppIdType for AppId {
    const HOME: Self = Self::Home;
}

pub type Transition = crate::rustmix_x4::x4_kernel::kernel::app::Transition<AppId>;
pub type NavEvent = crate::rustmix_x4::x4_kernel::kernel::app::NavEvent<AppId>;
pub type Launcher = crate::rustmix_x4::x4_kernel::kernel::app::Launcher<AppId>;
pub use crate::rustmix_x4::x4_kernel::kernel::app::{
    App, AppContext, PendingSetting, RECENT_FILE, Redraw,
};
