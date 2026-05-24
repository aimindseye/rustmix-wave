// ui re-exports: kernel primitives + app-side font-dependent widgets
//
// kernel ui (Region, Alignment, StackFmt, statusbar constants) is
// re-exported from the Rustmix-owned X4 kernel module; font-dependent widgets (BitmapLabel,
// QuickMenu, ButtonFeedback) come from apps::widgets

// kernel-side primitives
pub use crate::rustmix_x4::x4_kernel::ui::stack_fmt;
pub use crate::rustmix_x4::x4_kernel::ui::*;

// app-side font-dependent widgets
pub use crate::rustmix_x4::x4_apps::apps::widgets::bitmap_label::{
    BitmapDynLabel, BitmapLabel, BitmapTextWeight,
};
pub use crate::rustmix_x4::x4_apps::apps::widgets::button_feedback::{
    BUTTON_BAR_H, ButtonFeedback,
};
