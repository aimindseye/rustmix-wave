//! Screen rendering boundary for the product shell.

use core::convert::Infallible;

use crate::orientation::OrientedFrameBuffer;

use super::{router::ScreenRoute, state::AppState};

pub mod alarms;
pub mod audio;
pub mod calendar;
pub mod category;
pub mod clock;
pub mod device_info;
pub mod dictionary;
pub mod display;
pub mod environment;
pub mod files;
pub mod home;
pub mod lua_game;
pub mod motion;
pub mod network;
pub mod placeholder;
pub mod power_key;
pub mod reader;
pub mod unit_converter;
pub mod voice_notes;
pub mod weather;

/// Draw the active screen selected by the router.
pub fn render_active_screen(
    display: &mut OrientedFrameBuffer<'_>,
    state: &AppState,
) -> Result<(), Infallible> {
    match state.active_route() {
        ScreenRoute::Home => home::render_home(display, state),
        route if route.is_category() => category::render_category(display, state),
        route if route.is_placeholder() => placeholder::render_placeholder(display, state),
        ScreenRoute::ContinueReading => reader::render_continue_reading(display, state),
        ScreenRoute::Library => reader::render_library(display, state),
        ScreenRoute::Bookmarks | ScreenRoute::ReaderBookmarks => {
            reader::render_bookmarks(display, state)
        }
        ScreenRoute::ReaderLoading => reader::render_loading(display, state),
        ScreenRoute::ReaderPage => reader::render_page(display, state),
        ScreenRoute::ReaderOptions => reader::render_options(display, state),
        ScreenRoute::ReaderPreferences => reader::render_preferences(display, state),
        ScreenRoute::ReaderToc => reader::render_toc(display, state),
        ScreenRoute::Calendar => calendar::render_calendar(display, state),
        ScreenRoute::CalendarAgenda => calendar::render_calendar_agenda(display, state),
        ScreenRoute::CalendarEventDetails => {
            calendar::render_calendar_event_details(display, state)
        }
        ScreenRoute::CalendarEventEditor => calendar::render_calendar_event_editor(display, state),
        ScreenRoute::CalendarDeleteConfirmation => {
            calendar::render_calendar_delete_confirmation(display, state)
        }
        ScreenRoute::VoiceNotes => voice_notes::render_voice_notes(display, state),
        ScreenRoute::VoiceNoteDetails => voice_notes::render_voice_note_details(display, state),
        ScreenRoute::VoiceNoteRecording => voice_notes::render_voice_note_recording(display, state),
        ScreenRoute::LuaApps => lua_game::render_lua_apps(display, state),
        ScreenRoute::LuaGame => lua_game::render_lua_game(display, state),
        ScreenRoute::LuaGameError => lua_game::render_lua_error(display, state),
        ScreenRoute::Dictionary => dictionary::render_dictionary(display, state),
        ScreenRoute::UnitConverter => unit_converter::render_unit_converter(display, state),
        ScreenRoute::Clock => clock::render_clock(display, state),
        ScreenRoute::ClockDetails => clock::render_clock_details(display, state),
        ScreenRoute::Environment => environment::render_environment(display, state),
        ScreenRoute::EnvironmentDetails => environment::render_environment_details(display, state),
        ScreenRoute::Motion => motion::render_motion(display, state),
        ScreenRoute::MotionEvents => motion::render_motion_events(display, state),
        ScreenRoute::MotionDetails => motion::render_motion_details(display, state),
        ScreenRoute::Network => network::render_network(display, state),
        ScreenRoute::NetworkDetails => network::render_network_details(display, state),
        ScreenRoute::WifiTransfer => network::render_wifi_transfer(display, state),
        ScreenRoute::Weather => weather::render_weather(display, state),
        ScreenRoute::WeatherDetails => weather::render_weather_details(display, state),
        ScreenRoute::Alarms => alarms::render_alarms(display, state),
        ScreenRoute::Audio => audio::render_audio(display, state),
        ScreenRoute::AudioDetails => audio::render_audio_details(display, state),
        ScreenRoute::Files => files::render_files(display, state),
        ScreenRoute::Display => display::render_display(display, state),
        ScreenRoute::PowerKeyMenu => power_key::render_power_key_menu(display, state),
        ScreenRoute::DeviceInfo => device_info::render_device_info(display, state),
        ScreenRoute::DeviceInfoBoard => device_info::render_device_info_board(display, state),
        ScreenRoute::DeviceInfoRuntime => device_info::render_device_info_runtime(display, state),
        ScreenRoute::Reader
        | ScreenRoute::Productivity
        | ScreenRoute::Games
        | ScreenRoute::Tools
        | ScreenRoute::Settings
        | ScreenRoute::GamesTbd => unreachable!("category and placeholder routes handled above"),
    }
}
