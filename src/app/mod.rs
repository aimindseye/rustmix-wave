//! Modular portrait product UI shell.
//!
//! Hardware-independent screen state and drawing code live below this module.
//! `main.rs` wires peripherals, forwards debounced events, captures optional
//! board-service snapshots and asks this shell to render the active route.

use core::convert::Infallible;

use crate::{framebuffer::FrameBuffer, orientation::OrientedFrameBuffer};

pub mod display;
pub mod menu;
pub mod reader_atkinson_next_assets;
pub mod reader_literata_assets;
pub mod reader_serif_assets;
pub mod reader_typography;
pub mod router;
pub mod screens;
pub mod state;
pub mod typography;
pub mod widgets;

pub use router::ScreenRoute;
pub use state::AppState;

/// Idle interval before the panel controller and ALDO3 rail enter sleep.
pub const PANEL_IDLE_SLEEP_SECONDS: u64 = 60;
/// Detail-screen status cadence inherited from the sample-app clock use case.
pub const SAMPLE_LIVE_REFRESH_SECONDS: u64 = 30;
/// Motion diagnostics refresh at a slower e-paper-safe cadence.
pub const MOTION_LIVE_REFRESH_SECONDS: u64 = 10;
/// Motion-event diagnostics refresh slowly unless an event arrives sooner.
pub const IMU_EVENT_SCREEN_REFRESH_SECONDS: u64 =
    crate::imu_events::IMU_EVENT_SCREEN_REFRESH_SECONDS;
/// Network diagnostics refresh while visible.
pub const NETWORK_LIVE_REFRESH_SECONDS: u64 = 10;
/// Concise network serial heartbeat; UI refresh remains independent.
pub const NETWORK_LOG_HEARTBEAT_SECONDS: u64 = 30;
/// Voice-recording e-paper timer updates stay deliberately coarse.
pub const VOICE_RECORD_SCREEN_REFRESH_SECONDS: u64 =
    crate::voice_notes::VOICE_RECORD_SCREEN_REFRESH_SECONDS;
/// Poll the PCF85063 alarm flag and domain schedule once per second.
pub const ALARM_POLL_SECONDS: u64 = 1;

/// Clear the native frame and render the active product screen through the
/// orientation adapter.
pub fn render_current_screen(frame: &mut FrameBuffer, state: &AppState) -> Result<(), Infallible> {
    frame.clear_white();
    let mut display = OrientedFrameBuffer::new(frame, state.orientation);
    screens::render_active_screen(&mut display, state)
}

#[cfg(test)]
mod tests {
    use embedded_graphics::prelude::Point;

    use super::{render_current_screen, AppState, ScreenRoute};
    use crate::{buttons::ButtonEvent, framebuffer::FrameBuffer};

    #[test]
    fn home_renderer_places_black_ink_in_rotated_native_dashboard_chrome() {
        let mut frame = FrameBuffer::new_white();
        render_current_screen(&mut frame, &AppState::default()).unwrap();
        // Portrait logical header y=0 maps to the native left edge.
        assert_eq!(frame.is_black(Point::new(0, 479)), Some(true));
        // The v0.13.5 fixed dark footer maps to the native right edge.
        assert_eq!(frame.is_black(Point::new(799, 479)), Some(true));
        // The outer margin beside the category cards remains white.
        assert_eq!(frame.is_black(Point::new(400, 0)), Some(false));
    }

    #[test]
    fn settings_display_renderer_is_reachable_from_home() {
        let mut frame = FrameBuffer::new_white();
        let mut state = AppState::default();
        state.home_selected = 4;
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Settings);
        for _ in 0..3 {
            state.apply(ButtonEvent::Down);
        }
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Display);
        render_current_screen(&mut frame, &state).unwrap();
        assert_eq!(frame.is_black(Point::new(0, 479)), Some(true));
    }

    #[test]
    fn tools_file_browser_route_is_reachable() {
        let mut state = AppState::default();
        state.home_selected = 3;
        state.apply(ButtonEvent::Select);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Files);
    }

    #[test]
    fn tools_dictionary_route_renders_offline_without_sd_pack() {
        let mut frame = FrameBuffer::new_white();
        let mut state = AppState::default();
        state.home_selected = 3;
        state.apply(ButtonEvent::Select);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Dictionary);
        render_current_screen(&mut frame, &state).unwrap();
    }

    #[test]
    fn tools_unit_converter_route_renders_offline() {
        let mut frame = FrameBuffer::new_white();
        let mut state = AppState::default();
        state.home_selected = 3;
        state.apply(ButtonEvent::Select);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::UnitConverter);
        render_current_screen(&mut frame, &state).unwrap();
    }

    #[test]
    fn power_key_short_menu_route_renders_and_returns_to_previous_screen() {
        let mut frame = FrameBuffer::new_white();
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Dictionary);
        state.open_power_key_menu();
        assert_eq!(state.active_route(), ScreenRoute::PowerKeyMenu);
        render_current_screen(&mut frame, &state).unwrap();
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Dictionary);
        assert!(state.take_power_key_manual_refresh_request());
    }
}
