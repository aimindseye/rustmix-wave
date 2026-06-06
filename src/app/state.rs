//! Product UI state transitions independent of hardware wiring.

use crate::{
    alarm::AlarmSnapshot,
    audio::{AudioSnapshot, AudioUiRequest},
    board_services::BoardSnapshot,
    buttons::ButtonEvent,
    calendar::{CalendarEditorOutcome, CalendarUiRequest, CalendarUiState},
    dictionary::DictionaryUiState,
    imu::ImuReading,
    imu_events::{ImuControlOutcome, ImuDetectedEvent, ImuEventBridge},
    lua_runtime::LuaRuntimeUiState,
    network::NetworkSnapshot,
    orientation::DisplayOrientation,
    power_key_menu::{PowerKeyMenuOutcome, PowerKeyMenuUiState},
    reader::{ReaderOption, ReaderOrientation, ReaderTickOutcome, ReaderUiState},
    regional::RegionalPreferences,
    storage::StorageSnapshot,
    unit_converter::UnitConverterUiState,
    voice_notes::{VoiceNotesUiRequest, VoiceNotesUiState},
    weather::WeatherSnapshot,
    wifi_transfer::{WifiTransferSnapshot, WifiTransferState, WifiTransferUiRequest},
};

use super::{
    display::DisplayPreferences,
    menu::{category_entries, category_index, home_entries, CATEGORY_COUNT},
    router::{ScreenRoute, ScreenRouter},
};

/// Number of selectable rows in the playback overview screen.
pub const AUDIO_ACTION_COUNT: usize = 6;
/// Number of selectable rows in the Display settings screen.
pub const DISPLAY_ACTION_COUNT: usize = 2;
/// Number of selectable rows in the Weather overview screen.
pub const WEATHER_ACTION_COUNT: usize = 2;
/// Start/stop portal and provisioning-details rows on the Network screen.
pub const NETWORK_ACTION_COUNT: usize = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppState {
    pub home_selected: usize,
    category_selected: [usize; CATEGORY_COUNT],
    pub display_action_selected: usize,
    pub display: DisplayPreferences,
    /// Read-only monthly Calendar Foundation cursor and navigation mode.
    pub calendar: CalendarUiState,
    /// Offline X4-pack-compatible native Dictionary keyboard and lookup snapshot.
    pub dictionary: DictionaryUiState,
    /// Offline fixed-point Unit Converter cursor and editable field.
    pub unit_converter: UnitConverterUiState,
    /// TXT / reflowable EPUB Reader library, staged opening, RAM cache and options.
    pub reader: ReaderUiState,
    /// SD-loaded app catalog, bounded bootstrap executor and native canvas session.
    pub lua_runtime: LuaRuntimeUiState,
    /// Rust-owned debounced QMI8658 event bridge and diagnostics controls.
    pub imu_events: ImuEventBridge,
    pub partial_refreshes: u8,
    pub panel_awake: bool,
    pub select_presses: u32,
    pub orientation: DisplayOrientation,
    pub regional: RegionalPreferences,
    pub router: ScreenRouter,
    pub board: BoardSnapshot,
    pub storage: StorageSnapshot,
    /// Password-free snapshot owned by the networking boundary.
    pub network: NetworkSnapshot,
    /// Cached weather snapshot retained across transient HTTP failures.
    pub weather: WeatherSnapshot,
    /// SD-backed alarm schedules and active-alarm UI snapshot.
    pub alarms: AlarmSnapshot,
    /// Playback-only ES8311 diagnostics snapshot.
    pub audio: AudioSnapshot,
    /// Selected Audio-overview action.
    pub audio_action_selected: usize,
    /// Selected Weather-overview action: refresh or details.
    pub weather_action_selected: usize,
    /// Selected Network action: portal toggle or provisioning details.
    pub network_action_selected: usize,
    /// Compact LAN portal lifecycle snapshot.
    pub wifi_transfer: WifiTransferSnapshot,
    wifi_transfer_request: Option<WifiTransferUiRequest>,
    /// SD-backed PCM WAV voice-note catalog and recorder UI snapshot.
    pub voice_notes: VoiceNotesUiState,
    /// Global display-maintenance menu opened by a physical Power short press.
    pub power_key_menu: PowerKeyMenuUiState,
    power_key_menu_return_route: ScreenRoute,
    power_key_manual_refresh_requested: bool,
    weather_refresh_requested: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            home_selected: 0,
            category_selected: [0; CATEGORY_COUNT],
            display_action_selected: 0,
            display: DisplayPreferences::default(),
            calendar: CalendarUiState::default(),
            dictionary: DictionaryUiState::default(),
            unit_converter: UnitConverterUiState::default(),
            reader: ReaderUiState::default(),
            lua_runtime: LuaRuntimeUiState::default(),
            imu_events: ImuEventBridge::default(),
            partial_refreshes: 0,
            panel_awake: true,
            select_presses: 0,
            orientation: DisplayOrientation::default(),
            regional: RegionalPreferences::default(),
            router: ScreenRouter::default(),
            board: BoardSnapshot::default(),
            storage: StorageSnapshot::default(),
            network: NetworkSnapshot::default(),
            weather: WeatherSnapshot::default(),
            alarms: AlarmSnapshot::default(),
            audio: AudioSnapshot::default(),
            audio_action_selected: 0,
            weather_action_selected: 0,
            network_action_selected: 0,
            wifi_transfer: WifiTransferSnapshot::default(),
            wifi_transfer_request: None,
            voice_notes: VoiceNotesUiState::default(),
            power_key_menu: PowerKeyMenuUiState::default(),
            power_key_menu_return_route: ScreenRoute::Home,
            power_key_manual_refresh_requested: false,
            weather_refresh_requested: false,
        }
    }
}

impl AppState {
    /// Apply one debounced button event to routes whose behavior is fully
    /// hardware-independent. Files, Alarms and Audio remain delegated to their
    /// existing owners from main.rs.
    pub fn apply(&mut self, event: ButtonEvent) {
        let route = self.router.current();
        if route == ScreenRoute::Home {
            self.apply_home(event);
        } else if route.is_category() {
            self.apply_category(route, event);
        } else if route == ScreenRoute::Display {
            self.apply_display(event);
        } else if route == ScreenRoute::PowerKeyMenu {
            self.apply_power_key_menu(event);
        } else if route == ScreenRoute::Calendar {
            self.apply_calendar(event);
        } else if route == ScreenRoute::CalendarAgenda {
            self.apply_calendar_agenda(event);
        } else if route == ScreenRoute::CalendarEventDetails {
            self.apply_calendar_event_details(event);
        } else if route == ScreenRoute::CalendarEventEditor {
            self.apply_calendar_event_editor(event);
        } else if route == ScreenRoute::CalendarDeleteConfirmation {
            self.apply_calendar_delete_confirmation(event);
        } else if route == ScreenRoute::Dictionary {
            self.apply_dictionary(event);
        } else if route == ScreenRoute::UnitConverter {
            self.apply_unit_converter(event);
        } else if route == ScreenRoute::MotionEvents {
            self.apply_motion_events(event);
        } else if matches!(
            route,
            ScreenRoute::VoiceNotes
                | ScreenRoute::VoiceNoteDetails
                | ScreenRoute::VoiceNoteRecording
        ) {
            self.apply_voice_notes(event);
        } else if matches!(
            route,
            ScreenRoute::LuaApps | ScreenRoute::LuaGame | ScreenRoute::LuaGameError
        ) {
            self.apply_lua_runtime(event);
        } else if matches!(
            route,
            ScreenRoute::ContinueReading
                | ScreenRoute::Library
                | ScreenRoute::Bookmarks
                | ScreenRoute::ReaderBookmarks
                | ScreenRoute::ReaderLoading
                | ScreenRoute::ReaderPage
                | ScreenRoute::ReaderOptions
                | ScreenRoute::ReaderPreferences
                | ScreenRoute::ReaderToc
        ) {
            self.apply_reader(event);
        } else if route.is_placeholder() {
            // Placeholders are intentionally inert. Hierarchical navigation is
            // consistently handled by the dedicated GPIO0 BOOT long press.
        } else {
            match (route, event) {
                (ScreenRoute::Weather, ButtonEvent::Up) => {
                    self.weather_action_selected = self
                        .weather_action_selected
                        .checked_sub(1)
                        .unwrap_or(WEATHER_ACTION_COUNT - 1);
                }
                (ScreenRoute::Weather, ButtonEvent::Down) => {
                    self.weather_action_selected =
                        (self.weather_action_selected + 1) % WEATHER_ACTION_COUNT;
                }
                (ScreenRoute::Weather, ButtonEvent::Select) => {
                    self.note_select_press();
                    if self.weather_action_selected == 0 {
                        self.weather_refresh_requested = true;
                    } else {
                        self.router.navigate_to(ScreenRoute::WeatherDetails);
                    }
                }
                (ScreenRoute::Clock, ButtonEvent::Select) => {
                    self.note_select_press();
                    self.router.navigate_to(ScreenRoute::ClockDetails);
                }
                (ScreenRoute::Environment, ButtonEvent::Select) => {
                    self.note_select_press();
                    self.router.navigate_to(ScreenRoute::EnvironmentDetails);
                }
                (ScreenRoute::Motion, ButtonEvent::Select) => {
                    self.note_select_press();
                    self.router.navigate_to(ScreenRoute::MotionEvents);
                }
                (ScreenRoute::Network, ButtonEvent::Up) => {
                    self.network_action_selected = self
                        .network_action_selected
                        .checked_sub(1)
                        .unwrap_or(NETWORK_ACTION_COUNT - 1);
                }
                (ScreenRoute::Network, ButtonEvent::Down) => {
                    self.network_action_selected =
                        (self.network_action_selected + 1) % NETWORK_ACTION_COUNT;
                }
                (ScreenRoute::Network, ButtonEvent::Select) => {
                    self.note_select_press();
                    if self.network_action_selected == 0 {
                        self.wifi_transfer_request = Some(if self.wifi_transfer.is_active() {
                            WifiTransferUiRequest::Stop
                        } else {
                            WifiTransferUiRequest::Start
                        });
                        self.router.navigate_to(ScreenRoute::WifiTransfer);
                    } else {
                        self.router.navigate_to(ScreenRoute::NetworkDetails);
                    }
                }
                (ScreenRoute::WifiTransfer, ButtonEvent::Select) => {
                    self.note_select_press();
                    self.wifi_transfer_request = Some(WifiTransferUiRequest::Stop);
                    self.router.navigate_to(ScreenRoute::Network);
                }
                (ScreenRoute::DeviceInfo, ButtonEvent::Select) => {
                    self.note_select_press();
                    self.router.navigate_to(ScreenRoute::DeviceInfoBoard);
                }
                (ScreenRoute::DeviceInfoBoard, ButtonEvent::Select) => {
                    self.note_select_press();
                    self.router.navigate_to(ScreenRoute::DeviceInfoRuntime);
                }
                (
                    ScreenRoute::Clock
                    | ScreenRoute::Environment
                    | ScreenRoute::Motion
                    | ScreenRoute::DeviceInfo
                    | ScreenRoute::DeviceInfoBoard,
                    ButtonEvent::Up | ButtonEvent::Down,
                )
                | (
                    ScreenRoute::AudioDetails
                    | ScreenRoute::ClockDetails
                    | ScreenRoute::DeviceInfoRuntime
                    | ScreenRoute::EnvironmentDetails
                    | ScreenRoute::MotionDetails
                    | ScreenRoute::NetworkDetails
                    | ScreenRoute::WifiTransfer
                    | ScreenRoute::WeatherDetails,
                    _,
                )
                | (ScreenRoute::Files | ScreenRoute::Alarms | ScreenRoute::Audio, _) => {}
                _ => {}
            }
        }
        self.sync_reader_orientation_for_active_route();
    }

    fn apply_home(&mut self, event: ButtonEvent) {
        let count = home_entries().len();
        match event {
            ButtonEvent::Up => {
                self.home_selected = self.home_selected.checked_sub(1).unwrap_or(count - 1);
            }
            ButtonEvent::Down => self.home_selected = (self.home_selected + 1) % count,
            ButtonEvent::Select => {
                self.note_select_press();
                if let Some(entry) = home_entries().get(self.home_selected) {
                    self.router.navigate_to(entry.route);
                }
            }
        }
    }

    fn apply_category(&mut self, route: ScreenRoute, event: ButtonEvent) {
        let entries = category_entries(route);
        match event {
            ButtonEvent::Up => {
                let selected = self.category_selection_mut(route);
                *selected = selected.checked_sub(1).unwrap_or(entries.len() - 1);
            }
            ButtonEvent::Down => {
                let selected = self.category_selection_mut(route);
                *selected = (*selected + 1) % entries.len();
            }
            ButtonEvent::Select => {
                let target = entries[self.category_selection(route)].route;
                self.note_select_press();
                if target == ScreenRoute::Weather {
                    self.weather_action_selected = 0;
                }
                if target == ScreenRoute::Audio {
                    self.audio_action_selected = 0;
                }
                if target == ScreenRoute::Display {
                    self.display_action_selected = 0;
                }
                if target == ScreenRoute::Calendar {
                    self.initialize_calendar_if_needed();
                    self.calendar.refresh_events();
                }
                if target == ScreenRoute::Library {
                    self.reader.refresh_library();
                }
                if target == ScreenRoute::LuaApps {
                    self.lua_runtime.refresh_catalog(true);
                }
                if target == ScreenRoute::VoiceNotes {
                    self.voice_notes.refresh_catalog();
                }
                if target == ScreenRoute::Dictionary {
                    self.dictionary.refresh_pack_status();
                }
                if target == ScreenRoute::ContinueReading && self.reader.session.is_some() {
                    self.router.navigate_to(ScreenRoute::ReaderPage);
                } else {
                    self.router.navigate_to(target);
                }
            }
        }
    }

    fn apply_dictionary(&mut self, event: ButtonEvent) {
        if event == ButtonEvent::Select {
            self.note_select_press();
        }
        self.dictionary.apply_button(event);
    }

    fn apply_voice_notes(&mut self, event: ButtonEvent) {
        match self.router.current() {
            ScreenRoute::VoiceNotes => {
                if event == ButtonEvent::Select {
                    self.note_select_press();
                }
                let start = self.voice_notes.apply_list_button(event);
                if start {
                    self.router.navigate_to(ScreenRoute::VoiceNoteRecording);
                } else if event == ButtonEvent::Select && self.voice_notes.selected >= 2 {
                    self.voice_notes.clear_transient_details();
                    self.router.navigate_to(ScreenRoute::VoiceNoteDetails);
                }
            }
            ScreenRoute::VoiceNoteDetails => {
                if event == ButtonEvent::Select {
                    self.note_select_press();
                }
                let was_title_editing = self.voice_notes.title_editing;
                let was_delete_confirmation = self.voice_notes.delete_confirmation;
                self.voice_notes.apply_detail_button(event);
                if event == ButtonEvent::Select
                    && !was_title_editing
                    && !was_delete_confirmation
                    && self.voice_notes.detail_selected == 4
                {
                    self.voice_notes.request_stop_playback();
                    self.voice_notes.clear_transient_details();
                    self.router.navigate_to(ScreenRoute::VoiceNotes);
                }
            }
            ScreenRoute::VoiceNoteRecording => {
                if event == ButtonEvent::Select {
                    self.note_select_press();
                }
                self.voice_notes.apply_recording_button(event);
            }
            _ => {}
        }
    }

    fn apply_motion_events(&mut self, event: ButtonEvent) {
        match event {
            ButtonEvent::Up => self.imu_events.select_previous_control(),
            ButtonEvent::Down => self.imu_events.select_next_control(),
            ButtonEvent::Select => {
                self.note_select_press();
                if self.imu_events.apply_selected_control() == ImuControlOutcome::OpenDetails {
                    self.router.navigate_to(ScreenRoute::MotionDetails);
                }
            }
        }
    }

    /// Feed one native QMI8658 reading into the debounced event bridge while
    /// preserving the latest raw sample for diagnostics screens.
    pub fn update_imu_event_sample(
        &mut self,
        reading: ImuReading,
        now_ms: u64,
    ) -> Option<ImuDetectedEvent> {
        self.board.imu = Some(reading);
        self.imu_events.process(reading, now_ms)
    }

    fn initialize_calendar_if_needed(&mut self) {
        let local = self.board.rtc.map(|rtc| self.regional.localize_rtc(rtc));
        self.calendar.initialize_if_needed(local);
    }

    fn apply_calendar(&mut self, event: ButtonEvent) {
        self.initialize_calendar_if_needed();
        match event {
            ButtonEvent::Up => self.calendar.move_previous(),
            ButtonEvent::Down => self.calendar.move_next(),
            ButtonEvent::Select => {
                self.note_select_press();
                self.calendar.toggle_mode();
            }
        }
    }

    fn apply_calendar_agenda(&mut self, event: ButtonEvent) {
        match event {
            ButtonEvent::Up => self.calendar.agenda_previous(),
            ButtonEvent::Down => self.calendar.agenda_next(),
            ButtonEvent::Select => {
                self.note_select_press();
                if self.calendar.selected_agenda_event().is_some() {
                    self.router.navigate_to(ScreenRoute::CalendarEventDetails);
                }
            }
        }
    }

    fn apply_calendar_event_details(&mut self, event: ButtonEvent) {
        if !self.calendar.selected_event_is_personal() {
            return;
        }
        match event {
            ButtonEvent::Up => self.calendar.select_previous_details_action(),
            ButtonEvent::Down => self.calendar.select_next_details_action(),
            ButtonEvent::Select => {
                self.note_select_press();
                match self.calendar.details_action_selected {
                    0 => {
                        if self.calendar.begin_edit_selected_personal() {
                            self.router.navigate_to(ScreenRoute::CalendarEventEditor);
                        }
                    }
                    1 => {
                        if self.calendar.prepare_delete_confirmation() {
                            self.router
                                .navigate_to(ScreenRoute::CalendarDeleteConfirmation);
                        }
                    }
                    _ => self.router.navigate_to(ScreenRoute::CalendarAgenda),
                }
            }
        }
    }

    fn apply_calendar_event_editor(&mut self, event: ButtonEvent) {
        if event == ButtonEvent::Select {
            self.note_select_press();
        }
        match self.calendar.apply_editor_button(event) {
            CalendarEditorOutcome::None => {}
            CalendarEditorOutcome::Save(request) => self.calendar.queue_request(request),
            CalendarEditorOutcome::Cancel => {
                self.calendar.clear_editor();
                self.router.navigate_to(ScreenRoute::CalendarAgenda);
            }
        }
    }

    fn apply_calendar_delete_confirmation(&mut self, event: ButtonEvent) {
        match event {
            ButtonEvent::Up => self.calendar.select_previous_delete_confirmation(),
            ButtonEvent::Down => self.calendar.select_next_delete_confirmation(),
            ButtonEvent::Select => {
                self.note_select_press();
                if self.calendar.delete_confirmation_selected == 0 {
                    self.router.navigate_to(ScreenRoute::CalendarEventDetails);
                } else {
                    self.calendar.request_delete_selected_personal();
                }
            }
        }
    }

    /// Calendar keeps the accepted SELECT Day / Month toggle. BOOT short opens
    /// the selected-day agenda, then opens a create-personal editor from the
    /// agenda. BOOT long remains hierarchical Back.
    pub fn apply_calendar_boot_short_press(&mut self) -> bool {
        match self.router.current() {
            ScreenRoute::Calendar => {
                self.calendar.prepare_agenda();
                self.router.navigate_to(ScreenRoute::CalendarAgenda);
                true
            }
            ScreenRoute::CalendarAgenda => {
                self.calendar.begin_create_personal();
                self.router.navigate_to(ScreenRoute::CalendarEventEditor);
                true
            }
            _ => false,
        }
    }

    fn apply_unit_converter(&mut self, event: ButtonEvent) {
        match event {
            ButtonEvent::Up => self.unit_converter.increase_active(),
            ButtonEvent::Down => self.unit_converter.decrease_active(),
            ButtonEvent::Select => {
                self.note_select_press();
                self.unit_converter.select_next_field();
            }
        }
    }

    fn apply_lua_runtime(&mut self, event: ButtonEvent) {
        match self.router.current() {
            ScreenRoute::LuaApps => {
                if event == ButtonEvent::Select {
                    self.note_select_press();
                }
                if self.lua_runtime.apply_catalog_button(event) {
                    self.router.navigate_to(ScreenRoute::LuaGame);
                } else if self.lua_runtime.error.is_some() {
                    self.router.navigate_to(ScreenRoute::LuaGameError);
                }
            }
            ScreenRoute::LuaGame => {
                if event == ButtonEvent::Select {
                    self.note_select_press();
                }
                self.lua_runtime.apply_game_button(event);
                if self.lua_runtime.error.is_some() {
                    self.router.navigate_to(ScreenRoute::LuaGameError);
                }
            }
            ScreenRoute::LuaGameError => {}
            _ => {}
        }
    }

    /// Route BOOT short press into keyboard-style screens before game-specific
    /// contextual handlers. Future text-entry apps should compose the shared
    /// KeyboardGridNavigation helper and join this routing boundary.
    pub fn apply_keyboard_boot_short_press(&mut self) -> bool {
        if self.router.current() == ScreenRoute::CalendarEventEditor {
            self.calendar.toggle_editor_navigation_axis()
        } else if self.router.current() == ScreenRoute::VoiceNoteDetails
            && self.voice_notes.title_editing
        {
            self.voice_notes.toggle_title_editor_navigation_axis()
        } else if self.router.current() == ScreenRoute::Dictionary {
            self.dictionary.toggle_navigation_axis();
            true
        } else {
            false
        }
    }

    pub fn apply_lua_game_boot_short_press(&mut self) -> bool {
        self.router.current() == ScreenRoute::LuaGame
            && self.lua_runtime.apply_game_boot_short_press()
    }

    #[must_use]
    pub fn lua_game_needs_imu_events(&self) -> bool {
        self.router.current() == ScreenRoute::LuaGame && self.lua_runtime.needs_imu_events()
    }

    pub fn apply_lua_game_motion_event(&mut self, event: ImuDetectedEvent) -> bool {
        self.router.current() == ScreenRoute::LuaGame
            && self.lua_runtime.apply_game_motion_event(event)
    }

    pub fn refresh_lua_app_catalog(&mut self, mounted: bool) {
        self.lua_runtime.refresh_catalog(mounted);
    }

    pub fn take_lua_runtime_diagnostics(&mut self) -> Vec<String> {
        self.lua_runtime.take_diagnostics()
    }

    fn apply_reader(&mut self, event: ButtonEvent) {
        match self.router.current() {
            ScreenRoute::ContinueReading => {
                if event == ButtonEvent::Select {
                    self.note_select_press();
                    if self.reader.session.is_some() {
                        self.router.navigate_to(ScreenRoute::ReaderPage);
                    } else if self.reader.request_continue() {
                        self.router.navigate_to(ScreenRoute::ReaderLoading);
                    } else {
                        self.reader.refresh_library();
                        self.router.navigate_to(ScreenRoute::Library);
                    }
                }
            }
            ScreenRoute::Library => {
                if event == ButtonEvent::Select {
                    self.note_select_press();
                }
                if self.reader.apply_library_button(event) {
                    self.router.navigate_to(ScreenRoute::ReaderLoading);
                }
            }
            ScreenRoute::Bookmarks | ScreenRoute::ReaderBookmarks => {
                if event == ButtonEvent::Select {
                    self.note_select_press();
                }
                if self.reader.apply_bookmarks_button(event) {
                    self.router.navigate_to(ScreenRoute::ReaderLoading);
                }
            }
            ScreenRoute::ReaderToc => {
                if event == ButtonEvent::Select {
                    self.note_select_press();
                }
                if self.reader.apply_toc_button(event) {
                    self.router.navigate_to(ScreenRoute::ReaderPage);
                }
            }
            ScreenRoute::ReaderLoading => {}
            ScreenRoute::ReaderPage => match event {
                ButtonEvent::Up => self.reader.previous_page(),
                ButtonEvent::Down => self.reader.next_page(),
                ButtonEvent::Select => {
                    self.note_select_press();
                    self.reader.options_selected = 0;
                    self.router.navigate_to(ScreenRoute::ReaderOptions);
                }
            },
            ScreenRoute::ReaderOptions => match event {
                ButtonEvent::Up => self.reader.cycle_option_previous(),
                ButtonEvent::Down => self.reader.cycle_option_next(),
                ButtonEvent::Select => {
                    self.note_select_press();
                    match self.reader.selected_option() {
                        ReaderOption::Bookmark => self.reader.toggle_current_bookmark(),
                        ReaderOption::Bookmarks => {
                            self.reader.bookmarks_selected = 0;
                            self.router.navigate_to(ScreenRoute::ReaderBookmarks);
                        }
                        ReaderOption::TableOfContents => {
                            self.reader.toc_selected = 0;
                            self.router.navigate_to(ScreenRoute::ReaderToc)
                        }
                        ReaderOption::ReadingPreferences => {
                            self.reader.begin_preferences_edit();
                            self.router.navigate_to(ScreenRoute::ReaderPreferences);
                        }
                        ReaderOption::ClearGhosting => self.reader.request_clear_ghosting(),
                        ReaderOption::GoToLibrary => {
                            self.reader.refresh_library();
                            self.router.navigate_to(ScreenRoute::Library);
                        }
                        ReaderOption::GoHome => self.router.back_home(),
                    }
                }
            },
            ScreenRoute::ReaderPreferences => match event {
                ButtonEvent::Up => self.reader.cycle_preference_previous(),
                ButtonEvent::Down => self.reader.cycle_preference_next(),
                ButtonEvent::Select => {
                    self.note_select_press();
                    if self.reader.activate_selected_preference() {
                        self.router.navigate_to(ScreenRoute::ReaderLoading);
                    }
                }
            },
            _ => {}
        }
    }

    /// Advance one bounded Reader loading or nearby-cache stage. main.rs calls
    /// this from the event loop so the loading screen is visible before reads.
    pub fn tick_reader(&mut self) -> ReaderTickOutcome {
        let outcome = self.reader.tick();
        if outcome == ReaderTickOutcome::FirstPageReady {
            self.router.navigate_to(ScreenRoute::ReaderPage);
        }
        self.sync_reader_orientation_for_active_route();
        outcome
    }

    #[must_use]
    pub fn take_reader_clear_ghost_request(&mut self) -> bool {
        self.reader.take_clear_ghost_request()
    }

    /// Open the global display-maintenance menu from any awake product route.
    pub fn open_power_key_menu(&mut self) {
        if self.router.current() != ScreenRoute::PowerKeyMenu {
            self.power_key_menu_return_route = self.router.current();
        }
        self.power_key_menu.reset();
        self.router.navigate_to(ScreenRoute::PowerKeyMenu);
    }

    /// Return the route that long Power sleep should restore after wake. This
    /// unwraps the maintenance menu so a hold from that screen sleeps the
    /// underlying product route rather than restoring the transient menu.
    #[must_use]
    pub fn power_key_sleep_restore_route(&self) -> ScreenRoute {
        if self.router.current() == ScreenRoute::PowerKeyMenu {
            self.power_key_menu_return_route
        } else {
            self.router.current()
        }
    }

    #[must_use]
    pub fn take_power_key_manual_refresh_request(&mut self) -> bool {
        core::mem::take(&mut self.power_key_manual_refresh_requested)
    }

    fn close_power_key_menu(&mut self) {
        self.router.navigate_to(self.power_key_menu_return_route);
        self.sync_reader_orientation_for_active_route();
    }

    fn apply_power_key_menu(&mut self, event: ButtonEvent) {
        if event == ButtonEvent::Select {
            self.note_select_press();
        }
        match self.power_key_menu.apply_button(event) {
            PowerKeyMenuOutcome::None => {}
            PowerKeyMenuOutcome::ClearGhosting => {
                self.power_key_manual_refresh_requested = true;
                self.close_power_key_menu();
            }
            PowerKeyMenuOutcome::Cancel => self.close_power_key_menu(),
        }
    }

    fn apply_display(&mut self, event: ButtonEvent) {
        match event {
            ButtonEvent::Up => {
                self.display_action_selected = self
                    .display_action_selected
                    .checked_sub(1)
                    .unwrap_or(DISPLAY_ACTION_COUNT - 1);
            }
            ButtonEvent::Down => {
                self.display_action_selected =
                    (self.display_action_selected + 1) % DISPLAY_ACTION_COUNT;
            }
            ButtonEvent::Select => {
                self.note_select_press();
                match self.display_action_selected {
                    0 => self.display.cycle_font_family(),
                    _ => self.display.cycle_font_size(),
                }
            }
        }
    }

    /// Apply one Audio-overview event. Hardware requests are returned to
    /// main.rs so this product state remains independent of ESP-IDF handles.
    pub fn apply_audio_button(&mut self, event: ButtonEvent) -> Option<AudioUiRequest> {
        match event {
            ButtonEvent::Up => {
                self.audio_action_selected = self
                    .audio_action_selected
                    .checked_sub(1)
                    .unwrap_or(AUDIO_ACTION_COUNT - 1);
                None
            }
            ButtonEvent::Down => {
                self.audio_action_selected = (self.audio_action_selected + 1) % AUDIO_ACTION_COUNT;
                None
            }
            ButtonEvent::Select => {
                self.note_select_press();
                match self.audio_action_selected {
                    0 => Some(AudioUiRequest::PlayTestChime),
                    1 => Some(AudioUiRequest::StopPlayback),
                    2 => Some(AudioUiRequest::VolumeUp),
                    3 => Some(AudioUiRequest::VolumeDown),
                    4 => Some(AudioUiRequest::ToggleMute),
                    _ => {
                        self.router.navigate_to(ScreenRoute::AudioDetails);
                        None
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn category_selection(&self, route: ScreenRoute) -> usize {
        category_index(route)
            .map(|index| self.category_selected[index])
            .unwrap_or(0)
    }

    fn category_selection_mut(&mut self, route: ScreenRoute) -> &mut usize {
        let index = category_index(route).expect("category route required");
        &mut self.category_selected[index]
    }

    pub fn note_select_press(&mut self) {
        self.select_presses = self.select_presses.saturating_add(1);
    }

    /// Navigate one level toward Home. The hardware runtime calls this after a
    /// validated GPIO0 BOOT-button long press.
    pub fn back(&mut self) {
        if self.router.current() == ScreenRoute::PowerKeyMenu {
            self.close_power_key_menu();
            return;
        }
        if matches!(
            self.router.current(),
            ScreenRoute::LuaGame | ScreenRoute::LuaGameError
        ) {
            self.lua_runtime.close_session();
        }
        if self.router.current() == ScreenRoute::WifiTransfer {
            self.wifi_transfer_request = Some(WifiTransferUiRequest::Stop);
        }
        if self.router.current() == ScreenRoute::VoiceNoteRecording {
            self.voice_notes.request_cancel_recording();
        }
        if self.router.current() == ScreenRoute::VoiceNoteDetails {
            if self.voice_notes.title_editing {
                self.voice_notes.cancel_title_edit();
                return;
            }
            if self.voice_notes.delete_confirmation {
                self.voice_notes.clear_transient_details();
                return;
            }
            self.voice_notes.request_stop_playback();
            self.voice_notes.clear_transient_details();
        }
        if self.router.current() == ScreenRoute::CalendarEventEditor {
            self.calendar.clear_editor();
        }
        if self.router.current() == ScreenRoute::ReaderLoading {
            self.reader.cancel_loading();
        }
        if self.router.current() == ScreenRoute::ReaderPreferences {
            if self.reader.finish_preferences_edit() {
                self.router.navigate_to(ScreenRoute::ReaderLoading);
            } else {
                self.router.navigate_to(ScreenRoute::ReaderOptions);
            }
        } else {
            self.router.back();
        }
        self.sync_reader_orientation_for_active_route();
    }

    fn sync_reader_orientation_for_active_route(&mut self) {
        self.orientation = if self.router.current() == ScreenRoute::ReaderPage {
            match self.reader.preferences.orientation {
                ReaderOrientation::Portrait => DisplayOrientation::Portrait,
                ReaderOrientation::Landscape => DisplayOrientation::Landscape,
            }
        } else {
            DisplayOrientation::Portrait
        };
    }

    #[must_use]
    pub const fn active_route(&self) -> ScreenRoute {
        self.router.current()
    }

    pub fn update_board_snapshot(&mut self, board: BoardSnapshot) {
        self.board = board;
    }

    pub fn update_storage_snapshot(&mut self, storage: StorageSnapshot) {
        self.storage = storage;
    }

    pub fn update_network_snapshot(&mut self, network: NetworkSnapshot) {
        self.network = network;
    }

    pub fn update_weather_snapshot(&mut self, weather: WeatherSnapshot) {
        self.weather = weather;
    }

    pub fn update_wifi_transfer_snapshot(&mut self, snapshot: WifiTransferSnapshot) {
        self.wifi_transfer = snapshot;
    }

    #[must_use]
    pub fn take_wifi_transfer_request(&mut self) -> Option<WifiTransferUiRequest> {
        self.wifi_transfer_request.take()
    }

    /// Start the existing LAN portal from a feature shortcut without exposing
    /// HTTP-server ownership outside the main-loop dispatcher.
    pub fn request_wifi_transfer_start(&mut self) {
        if !self.wifi_transfer.is_active() {
            self.wifi_transfer_request = Some(WifiTransferUiRequest::Start);
        }
    }

    #[must_use]
    pub fn take_calendar_request(&mut self) -> Option<CalendarUiRequest> {
        self.calendar.take_request()
    }

    pub fn refresh_voice_notes_catalog(&mut self) {
        self.voice_notes.refresh_catalog();
    }

    #[must_use]
    pub fn take_voice_notes_request(&mut self) -> Option<VoiceNotesUiRequest> {
        self.voice_notes.take_request()
    }

    pub fn request_wifi_transfer_stop(&mut self) {
        if self.wifi_transfer.state != WifiTransferState::Off {
            self.wifi_transfer_request = Some(WifiTransferUiRequest::Stop);
        }
    }

    pub fn update_alarm_snapshot(&mut self, alarms: AlarmSnapshot) {
        self.alarms = alarms;
    }

    pub fn update_audio_snapshot(&mut self, audio: AudioSnapshot) {
        self.audio = audio;
    }

    #[must_use]
    pub fn take_weather_refresh_request(&mut self) -> bool {
        core::mem::take(&mut self.weather_refresh_requested)
    }

    pub fn set_orientation(&mut self, orientation: DisplayOrientation) {
        self.orientation = orientation;
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;
    use crate::{app::router::ScreenRoute, buttons::ButtonEvent};

    #[test]
    fn motion_event_screen_cycles_thresholds_and_opens_sensor_details() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Motion);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::MotionEvents);
        let original = state.imu_events.thresholds.tilt_enter_mg;
        state.apply(ButtonEvent::Select);
        assert_ne!(state.imu_events.thresholds.tilt_enter_mg, original);
        for _ in 0..6 {
            state.apply(ButtonEvent::Down);
        }
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::MotionDetails);
    }

    #[test]
    fn home_categories_wrap_and_open() {
        let mut state = AppState::default();
        state.apply(ButtonEvent::Up);
        assert_eq!(state.home_selected, 4);
        state.apply(ButtonEvent::Down);
        assert_eq!(state.home_selected, 0);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Reader);
    }

    #[test]
    fn productivity_calendar_opens_and_toggles_navigation_mode() {
        use crate::calendar::CalendarNavigationMode;

        let mut state = AppState::default();
        state.home_selected = 1;
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Productivity);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Calendar);
        assert_eq!(state.calendar.mode, CalendarNavigationMode::Day);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.calendar.mode, CalendarNavigationMode::Month);
    }

    #[test]
    fn calendar_boot_short_opens_daily_agenda_and_details_route_safely() {
        use crate::calendar::{
            CalendarCatalogSnapshot, CalendarDate, CalendarEvent, CalendarEventKind,
        };

        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Calendar);
        state.calendar.cursor = CalendarDate::new(2026, 6, 19).unwrap();
        state.calendar.catalog = CalendarCatalogSnapshot {
            events: vec![CalendarEvent {
                date: CalendarDate::new(2026, 6, 19).unwrap(),
                kind: CalendarEventKind::UsHoliday,
                title: "Juneteenth".into(),
                detail: "Federal holiday".into(),
                source_row: 0,
            }],
            personal_loaded: true,
            us_loaded: true,
            warning: None,
        };
        assert!(state.apply_calendar_boot_short_press());
        assert_eq!(state.active_route(), ScreenRoute::CalendarAgenda);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::CalendarEventDetails);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::CalendarAgenda);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Calendar);
    }

    #[test]
    fn tools_file_browser_returns_to_tools() {
        let mut state = AppState::default();
        state.home_selected = 3;
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Tools);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Files);
        state.router.back();
        assert_eq!(state.active_route(), ScreenRoute::Tools);
    }

    #[test]
    fn settings_display_changes_persistent_preferences_without_a_back_row() {
        let mut state = AppState::default();
        state.home_selected = 4;
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Settings);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Display);
        let original = state.display;
        state.apply(ButtonEvent::Select);
        assert_ne!(state.display.font_family, original.font_family);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_ne!(state.display.font_size, original.font_size);
        state.apply(ButtonEvent::Down);
        assert_eq!(state.display_action_selected, 0);
        assert_eq!(state.active_route(), ScreenRoute::Display);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Settings);
    }

    #[test]
    fn network_portal_is_explicitly_started_and_stopped_from_network_settings() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Network);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::WifiTransfer);
        assert_eq!(
            state.take_wifi_transfer_request(),
            Some(crate::wifi_transfer::WifiTransferUiRequest::Start)
        );
        state.update_wifi_transfer_snapshot(crate::wifi_transfer::WifiTransferSnapshot {
            state: crate::wifi_transfer::WifiTransferState::Ready,
            url: Some("http://192.168.1.2/".into()),
            code: Some("123456".into()),
            last_action: "Portal ready".into(),
            last_bytes: 0,
            error: None,
        });
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Network);
        assert_eq!(
            state.take_wifi_transfer_request(),
            Some(crate::wifi_transfer::WifiTransferUiRequest::Stop)
        );
    }

    #[test]
    fn weather_details_use_select_then_hierarchical_back() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Weather);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::WeatherDetails);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Weather);
    }

    #[test]
    fn clock_details_use_select_then_hierarchical_back() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Clock);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::ClockDetails);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Clock);
    }

    #[test]
    fn tools_dictionary_opens_native_screen_without_sd_pack() {
        let mut state = AppState::default();
        state.home_selected = 3;
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Tools);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Dictionary);
        assert!(!state.dictionary.pack_ready);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.dictionary.query, "B");
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Tools);
    }

    #[test]
    fn voice_note_title_editor_boot_short_toggles_axis_and_long_back_cancels() {
        let mut state = AppState::default();
        state
            .voice_notes
            .notes
            .push(crate::voice_notes::VoiceNoteEntry {
                file_name: "VOICE001.WAV".into(),
                title: "VOICE NOTE 001".into(),
                recorded_at: "2026-06-06  11:43:24".into(),
                wav_bytes: 44,
                pcm_bytes: 0,
                duration_seconds: 0,
            });
        state.voice_notes.selected = 2;
        state.voice_notes.begin_title_edit();
        state.router.navigate_to(ScreenRoute::VoiceNoteDetails);
        assert_eq!(
            state.voice_notes.title_editor_navigation_mode_label(),
            "NAV H"
        );
        assert!(state.apply_keyboard_boot_short_press());
        assert_eq!(
            state.voice_notes.title_editor_navigation_mode_label(),
            "NAV V"
        );
        state.back();
        assert!(!state.voice_notes.title_editing);
        assert_eq!(state.active_route(), ScreenRoute::VoiceNoteDetails);
    }

    #[test]
    fn dictionary_keyboard_boot_short_toggles_axis_preserves_key_and_long_back_route() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Dictionary);
        state.apply(ButtonEvent::Down);
        assert_eq!(state.dictionary.selected_key_label(), "B");
        assert!(state.apply_keyboard_boot_short_press());
        assert_eq!(state.dictionary.navigation_mode_label(), "NAV V");
        assert_eq!(state.dictionary.selected_key_label(), "B");
        state.apply(ButtonEvent::Down);
        assert_eq!(state.dictionary.selected_key_label(), "H");
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Tools);
    }

    #[test]
    fn tools_unit_converter_opens_and_edits_without_hardware() {
        use crate::unit_converter::{ConverterField, UnitCategory};

        let mut state = AppState::default();
        state.home_selected = 3;
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Tools);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::UnitConverter);
        assert_eq!(state.unit_converter.active_field, ConverterField::Category);
        state.apply(ButtonEvent::Up);
        assert_eq!(state.unit_converter.category, UnitCategory::Mass);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.unit_converter.active_field, ConverterField::FromUnit);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Tools);
    }

    #[test]
    fn games_route_opens_sd_lua_catalog_safely_without_sd_card() {
        let mut state = AppState::default();
        state.home_selected = 2;
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Games);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::LuaApps);
        assert!(state.lua_runtime.catalog.warning.is_some());
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Games);
    }

    #[test]
    fn reader_continue_shell_routes_to_library_when_no_session() {
        let mut state = AppState::default();
        state.apply(ButtonEvent::Select);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::ContinueReading);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Library);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Reader);
    }

    #[test]
    fn reader_preferences_use_settings_style_move_then_select_change() {
        use crate::reader::{ReadingPreference, ReadingTheme};

        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::ReaderPreferences);
        assert_eq!(
            state.reader.selected_preference(),
            ReadingPreference::ReadingTheme
        );
        let initial_theme = state.reader.preferences.theme;
        state.apply(ButtonEvent::Down);
        assert_eq!(
            state.reader.selected_preference(),
            ReadingPreference::Orientation
        );
        assert_eq!(state.reader.preferences.theme, initial_theme);
        state.apply(ButtonEvent::Up);
        assert_eq!(
            state.reader.selected_preference(),
            ReadingPreference::ReadingTheme
        );
        state.apply(ButtonEvent::Select);
        assert_eq!(state.reader.preferences.theme, ReadingTheme::HighContrast);
        assert_eq!(state.active_route(), ScreenRoute::ReaderPreferences);
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::ReaderOptions);
    }
    #[test]
    fn productivity_voice_notes_opens_recording_route_and_queues_start() {
        let mut state = AppState::default();
        state.home_selected = 1;
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Productivity);
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::VoiceNotes);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::VoiceNoteRecording);
        assert_eq!(
            state.take_voice_notes_request(),
            Some(crate::voice_notes::VoiceNotesUiRequest::StartRecording)
        );
    }

    #[test]
    fn calendar_personal_details_routes_edit_delete_and_boot_create_safely() {
        use crate::calendar::{
            CalendarCatalogSnapshot, CalendarDate, CalendarEvent, CalendarEventKind,
        };

        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::CalendarAgenda);
        state.calendar.cursor = CalendarDate::new(2026, 7, 4).unwrap();
        state.calendar.catalog = CalendarCatalogSnapshot {
            events: vec![CalendarEvent {
                date: CalendarDate::new(2026, 7, 4).unwrap(),
                kind: CalendarEventKind::Personal,
                title: "Picnic".into(),
                detail: "Bring snacks".into(),
                source_row: 0,
            }],
            personal_loaded: true,
            us_loaded: true,
            warning: None,
        };
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::CalendarEventDetails);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::CalendarEventEditor);
        assert!(state.apply_keyboard_boot_short_press());
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::CalendarAgenda);
        assert!(state.apply_calendar_boot_short_press());
        assert_eq!(state.active_route(), ScreenRoute::CalendarEventEditor);
    }

    #[test]
    fn power_key_menu_preserves_return_route_and_requests_manual_refresh() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Dictionary);
        state.open_power_key_menu();
        assert_eq!(state.active_route(), ScreenRoute::PowerKeyMenu);
        assert_eq!(
            state.power_key_sleep_restore_route(),
            ScreenRoute::Dictionary
        );
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Dictionary);
        assert!(state.take_power_key_manual_refresh_request());
        assert!(!state.take_power_key_manual_refresh_request());
    }

    #[test]
    fn power_key_menu_cancel_and_back_return_without_refresh() {
        let mut state = AppState::default();
        state.router.navigate_to(ScreenRoute::Calendar);
        state.open_power_key_menu();
        state.apply(ButtonEvent::Down);
        state.apply(ButtonEvent::Select);
        assert_eq!(state.active_route(), ScreenRoute::Calendar);
        assert!(!state.take_power_key_manual_refresh_request());
        state.open_power_key_menu();
        state.back();
        assert_eq!(state.active_route(), ScreenRoute::Calendar);
    }
}
