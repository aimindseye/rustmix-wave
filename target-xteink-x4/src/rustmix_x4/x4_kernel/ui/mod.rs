// widget primitives for 1-bit e-paper displays
//
// font-independent: Region, Alignment, stack measurement, StackFmt
// font-dependent widgets (BitmapLabel, QuickMenu, ButtonFeedback)
// live in the distro's apps::widgets module

pub mod layout;
pub mod stack_fmt;
pub mod statusbar;
mod widget;

pub use layout::{
    CONTENT_TOP, FULL_CONTENT_W, HEADER_W, LARGE_MARGIN, SECTION_GAP, TITLE_Y_OFFSET,
};
pub use stack_fmt::{StackFmt, stack_fmt};
pub use statusbar::{free_stack_bytes, paint_stack, stack_high_water_mark};
pub use widget::{Alignment, Region, draw_loading_indicator, wrap_next, wrap_prev};
