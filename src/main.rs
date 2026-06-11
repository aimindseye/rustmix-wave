#[cfg(target_os = "espidf")]
mod firmware {
    use std::{
        ffi::CString,
        time::{Duration, Instant},
    };

    use anyhow::Result;
    use embedded_hal::delay::DelayNs;
    use esp_idf_svc::{
        fs::fatfs::Fatfs,
        hal::{
            delay::FreeRtos,
            gpio::{AnyIOPin, PinDriver, Pull},
            i2c::{I2cConfig, I2cDriver},
            i2s::{
                config::{
                    ClockSource, Config as I2sChannelConfig, DataBitWidth, MclkMultiple, SlotMode,
                    StdClkConfig, StdConfig, StdGpioConfig, StdSlotConfig,
                },
                I2sBiDir, I2sDriver,
            },
            peripherals::Peripherals,
            sd::{
                mmc::{SdMmcHostConfiguration, SdMmcHostDriver},
                SdCardConfiguration, SdCardDriver,
            },
            spi::{config::Config as SpiConfig, Dma, SpiBusDriver, SpiDriver, SpiDriverConfig},
            units::*,
        },
        io::vfs::MountedFatfs,
        log::EspLogger,
        sys,
    };
    use log::{info, warn};
    use waveshare_epd397_rust_app::{
        alarm::{AlarmEngine, AlarmSnapshot, AlarmUiOutcome, ALARMS_CONFIG_PATH},
        app::{
            display::{DisplayPreferences, DISPLAY_CONFIG_PATH},
            render_current_screen, AppState, ScreenRoute, ALARM_POLL_SECONDS,
            IMU_EVENT_SCREEN_REFRESH_SECONDS, MOTION_LIVE_REFRESH_SECONDS,
            NETWORK_LIVE_REFRESH_SECONDS, NETWORK_LOG_HEARTBEAT_SECONDS, PANEL_IDLE_SLEEP_SECONDS,
            SAMPLE_LIVE_REFRESH_SECONDS, VOICE_RECORD_SCREEN_REFRESH_SECONDS,
        },
        audio::{
            espidf::AudioRuntime, AudioSnapshot, AudioUiRequest, AUDIO_MCLK_HZ,
            AUDIO_SAMPLE_RATE_HZ, DEFAULT_AUDIO_VOLUME_PERCENT,
        },
        board_services::{BoardServices, BoardSnapshot},
        build_info::{FIRMWARE_VERSION, PRODUCT_SLUG, UI_SHELL_MILESTONE},
        buttons::{
            BootButtonEvent, ButtonEvent, Buttons, LongPressBackButton, BOOT_BACK_LONG_PRESS_MS,
        },
        calendar::{
            create_personal_event, delete_personal_event, update_personal_event, CalendarUiRequest,
            CALENDAR_EVENTS_FILE, CALENDAR_ROOT, CALENDAR_US_EVENTS_FILE,
        },
        dictionary::{DICTIONARY_ROOT, DICTIONARY_SHARD_MAX_BYTES},
        epaper::Epaper397,
        framebuffer::FrameBuffer,
        games::dirty_regions::MAX_DIRTY_REGIONS,
        imu_events::IMU_EVENT_SAMPLE_INTERVAL_MS,
        lua_runtime::{catalog::LUA_APPS_DIRECTORY, loader::LUA_LOADER_WORKER_STACK_BYTES},
        network::{
            espidf::NetworkRuntime, NetworkLogFingerprint, NetworkSnapshot, WifiConnectionState,
        },
        network_config::{NetworkConfig, WIFI_CONFIG_PATH},
        panel_refresh::{
            PanelGlobalReason, PanelRefreshCoordinator, PanelRefreshPlan, PanelRefreshRequest,
            PANEL_PARTIAL_REFRESH_LIMIT,
        },
        power::Axp2101,
        power_key::{
            PowerKeyEvent, SleepWakeGuard, SleepWakeGuardDecision, POWER_KEY_POLL_MS,
            POWER_KEY_WAKE_GUARD_QUIET_MS,
        },
        reader::ReaderTickOutcome,
        regional::RegionalPreferences,
        rtc::RtcDateTime,
        rtc_alarm_interrupt::{espidf::RtcAlarmInterruptMonitor, RTC_ALARM_INTERRUPT_GPIO},
        runtime_memory::log_runtime_memory,
        shared_i2c::SharedI2cBus,
        sleep_images::{SleepImageCatalog, SleepImageSelection, SLEEP_IMAGE_DIRECTORY},
        sleep_mode::{SleepModeState, SleepWakeCause},
        sleep_network::SleepNetworkState,
        storage::{
            StorageBrowser, StorageSnapshot, StorageUiOutcome, SDMMC_COMMAND_TIMEOUT_MS,
            SDMMC_STABLE_SPEED_KHZ, SD_MOUNT_POINT, STORAGE_IO_RETRY_ATTEMPTS,
        },
        voice_note_metadata::{
            load_voice_notes_preferences, save_voice_notes_preferences, VoiceNotesPreferences,
            VOICE_UNKNOWN_RECORDED_AT,
        },
        voice_notes::{
            cleanup_stale_voice_tmp, delete_voice_note, save_voice_note_title, VoiceNotesUiRequest,
            VoicePlaybackSession, VoiceRecordingSession, VOICE_NOTES_ROOT,
            VOICE_PCM_MONO_CHUNK_BYTES, VOICE_PCM_STEREO_CAPTURE_BYTES,
        },
        weather::{
            espidf::fetch_open_meteo_on_worker, WeatherFetchError, WeatherSnapshot,
            WEATHER_RETRY_DELAYS_SECONDS, WEATHER_RETRY_LIMIT,
        },
        weather_config::{WeatherConfig, WEATHER_CONFIG_PATH},
        wifi_transfer::{
            espidf::WifiTransferServer, WifiTransferSnapshot, WifiTransferUiRequest,
            WIFI_TRANSFER_INACTIVITY_SECONDS, WIFI_TRANSFER_ROOT, WIFI_TRANSFER_SERVER_STACK_BYTES,
        },
    };

    pub fn run() -> Result<()> {
        sys::link_patches();
        EspLogger::initialize_default();
        info!("rustmix-wave=epd397-rust-app-start");
        info!(
            "rustmix-wave=product-ui-shell-start product={PRODUCT_SLUG} version={FIRMWARE_VERSION} milestone={UI_SHELL_MILESTONE}"
        );

        let peripherals = Peripherals::take()?;

        // The uploaded Waveshare sample uses SDMMC in 4-bit mode:
        // CMD GPIO17, CLK GPIO16, D0 GPIO15, D1 GPIO7, D2 GPIO8, D3 GPIO18.
        // Mount failure is non-fatal so the verified product shell still boots
        // when no card is inserted.
        let mounted_sd = (|| {
            let host = SdMmcHostDriver::new_4bits(
                peripherals.sdmmc1,
                peripherals.pins.gpio17,
                peripherals.pins.gpio16,
                peripherals.pins.gpio15,
                peripherals.pins.gpio7,
                peripherals.pins.gpio8,
                peripherals.pins.gpio18,
                None::<AnyIOPin>,
                None::<AnyIOPin>,
                &SdMmcHostConfiguration::new(),
            )?;
            let mut card_config = SdCardConfiguration::new();
            card_config.speed_khz = SDMMC_STABLE_SPEED_KHZ;
            card_config.command_timeout_ms = SDMMC_COMMAND_TIMEOUT_MS;
            let card = SdCardDriver::new_mmc(host, &card_config)?;
            let fatfs = Fatfs::new_sdcard(0, card)?;
            MountedFatfs::mount(fatfs, SD_MOUNT_POINT, 5)
        })();
        let mounted_sd = match mounted_sd {
            Ok(mounted) => {
                info!(
                    "rustmix-wave=sdmmc-mount status=ready mount={SD_MOUNT_POINT} mode=4bit-fat access=ui-readonly speed-khz={SDMMC_STABLE_SPEED_KHZ} timeout-ms={SDMMC_COMMAND_TIMEOUT_MS} retry-attempts={STORAGE_IO_RETRY_ATTEMPTS}"
                );
                Some(mounted)
            }
            Err(error) => {
                warn!(
                    "rustmix-wave=sdmmc-mount status=unavailable mount={SD_MOUNT_POINT} mode=4bit-fat access=ui-readonly speed-khz={SDMMC_STABLE_SPEED_KHZ} timeout-ms={SDMMC_COMMAND_TIMEOUT_MS} retry-attempts={STORAGE_IO_RETRY_ATTEMPTS} error={error:#}"
                );
                None
            }
        };
        let mut storage_browser = StorageBrowser::new(SD_MOUNT_POINT, mounted_sd.is_some());
        let _mounted_sd = mounted_sd;
        let display_preferences = match DisplayPreferences::load_from_path(DISPLAY_CONFIG_PATH) {
            Ok(preferences) => {
                info!(
                    "rustmix-wave=display-config status=ready path={DISPLAY_CONFIG_PATH} font-family={} font-size={}",
                    preferences.font_family.marker(),
                    preferences.font_size.marker()
                );
                preferences
            }
            Err(error) => {
                let preferences = DisplayPreferences::default();
                warn!(
                    "rustmix-wave=display-config status=default path={DISPLAY_CONFIG_PATH} font-family={} font-size={} error={error:#}",
                    preferences.font_family.marker(),
                    preferences.font_size.marker()
                );
                preferences
            }
        };

        // Credentials are read from removable storage. Never log the password.
        let network_config = match NetworkConfig::load_from_path(WIFI_CONFIG_PATH) {
            Ok(config) => {
                info!(
                    "rustmix-wave=wifi-config status=ready path={WIFI_CONFIG_PATH} ssid={} timezone={} ntp-server={}",
                    config.ssid, config.timezone, config.ntp_server
                );
                Some(config)
            }
            Err(error) => {
                warn!(
                    "rustmix-wave=wifi-config status=unavailable path={WIFI_CONFIG_PATH} error={error:#}"
                );
                None
            }
        };

        let weather_config = match WeatherConfig::load_from_path(WEATHER_CONFIG_PATH) {
            Ok(config) => {
                info!(
                    "rustmix-wave=weather-config status=ready path={WEATHER_CONFIG_PATH} provider={} location={} latitude={:.4} longitude={:.4} timezone={} refresh-minutes={}",
                    config.provider,
                    config.location,
                    config.latitude,
                    config.longitude,
                    config.timezone,
                    config.refresh_minutes
                );
                Some(config)
            }
            Err(error) => {
                warn!(
                    "rustmix-wave=weather-config status=unavailable path={WEATHER_CONFIG_PATH} error={error:#}"
                );
                None
            }
        };

        let mut alarm_engine = match AlarmEngine::load_from_path(ALARMS_CONFIG_PATH) {
            Ok(engine) => {
                let snapshot = engine.snapshot();
                info!(
                    "rustmix-wave=alarm-config status=ready path={ALARMS_CONFIG_PATH} schedules={} snooze-minutes={}",
                    snapshot.alarms.len(), snapshot.snooze_minutes
                );
                engine
            }
            Err(error) => {
                warn!(
                    "rustmix-wave=alarm-config status=unavailable path={ALARMS_CONFIG_PATH} error={error:#}"
                );
                AlarmEngine::unavailable(format!("{error:#}"))
            }
        };

        let i2c_config = I2cConfig::new().baudrate(400.kHz().into());
        let i2c = I2cDriver::new(
            peripherals.i2c0,
            peripherals.pins.gpio41,
            peripherals.pins.gpio42,
            &i2c_config,
        )?;
        let shared_i2c = SharedI2cBus::new(i2c);
        let panel_power = Axp2101::new(shared_i2c.clone());
        let mut board_services = BoardServices::new(shared_i2c.clone());

        // Bidirectional ES8311 Voice Notes milestone. The uploaded BSP uses I2S0 with
        // MCLK GPIO13, BCLK GPIO14, WS GPIO47, ESP-to-codec DOUT GPIO48,
        // codec-to-ESP DIN GPIO21 and amplifier GPIO39. Start muted with the
        // amplifier disabled; audio failure remains non-fatal.
        info!("rustmix-wave=audio-init status=starting codec=es8311 address=0x18 wire-write=0x30");
        let audio_attempt = (|| -> Result<_> {
            let i2s_config = StdConfig::new(
                I2sChannelConfig::new().auto_clear(true),
                StdClkConfig::new(
                    AUDIO_SAMPLE_RATE_HZ,
                    ClockSource::default(),
                    MclkMultiple::M384,
                ),
                StdSlotConfig::philips_slot_default(DataBitWidth::Bits16, SlotMode::Stereo),
                StdGpioConfig::default(),
            );
            let mut i2s = I2sDriver::<I2sBiDir>::new_std_bidir(
                peripherals.i2s0,
                &i2s_config,
                peripherals.pins.gpio14,
                peripherals.pins.gpio21,
                peripherals.pins.gpio48,
                Some(peripherals.pins.gpio13),
                peripherals.pins.gpio47,
            )?;
            i2s.tx_enable()?;
            i2s.rx_enable()?;
            let amplifier = PinDriver::output(peripherals.pins.gpio39)?;
            AudioRuntime::initialize(shared_i2c.clone(), i2s, amplifier, &mut FreeRtosDelay)
        })();
        let (mut audio_runtime, initial_audio_snapshot) = match audio_attempt {
            Ok(runtime) => {
                let snapshot = runtime.snapshot();
                info!(
                    "rustmix-wave=audio-codec status=ready codec=es8311 address={} wire-write={} mclk-hz={AUDIO_MCLK_HZ}",
                    snapshot.codec_address_label(),
                    snapshot.codec_address.map_or_else(|| "--".into(), |address| format!("0x{:02X}", address << 1))
                );
                let profile = runtime.profile();
                info!(
                    "rustmix-wave=audio-codec-profile status=ready source=waveshare-esp-codec-dev-parity gpio44=0x{:02X} dac-reference=ready system14=0x{:02X} adc15=0x{:02X} adc17=0x{:02X} gp45=0x{:02X}",
                    profile.gpio44,
                    profile.system14,
                    profile.adc15,
                    profile.adc17,
                    profile.gp45
                );
                info!("rustmix-wave=audio-i2s status=ready direction=bidir sample-rate={AUDIO_SAMPLE_RATE_HZ} bits=16 tx-channels=2 rx-channels=2 voice-wav-channels=1 mclk-gpio=13 bclk-gpio=14 ws-gpio=47 dout-gpio=48 din-gpio=21");
                info!("rustmix-wave=audio-amp status=ready gpio=39 default=off");
                info!("rustmix-wave=audio-subsystem-ready mute=true volume={DEFAULT_AUDIO_VOLUME_PERCENT}");
                (Some(runtime), snapshot)
            }
            Err(error) => {
                warn!("rustmix-wave=audio-init status=unavailable codec=es8311 error={error:#}");
                (None, AudioSnapshot::unavailable(format!("{error:#}")))
            }
        };

        let spi_driver_config = SpiDriverConfig::new().dma(Dma::Auto(4096));
        let spi_driver = SpiDriver::new(
            peripherals.spi3,
            peripherals.pins.gpio11,
            peripherals.pins.gpio12,
            None::<AnyIOPin>,
            &spi_driver_config,
        )?;
        let spi_config = SpiConfig::new().baudrate(20.MHz().into()).write_only(true);
        let spi = SpiBusDriver::new(spi_driver, &spi_config)?;

        let dc = PinDriver::output(peripherals.pins.gpio9)?;
        let reset = PinDriver::output(peripherals.pins.gpio46)?;
        let cs = PinDriver::output(peripherals.pins.gpio10)?;
        // GPIO3 is display busy. Do not reuse it for rotary or app input.
        let busy = PinDriver::input(peripherals.pins.gpio3, Pull::Up)?;

        let mut panel = Epaper397::new(spi, dc, reset, cs, busy, FreeRtosDelay, panel_power)?;
        let mut buttons = Buttons::new(
            PinDriver::input(peripherals.pins.gpio4, Pull::Up)?,
            PinDriver::input(peripherals.pins.gpio5, Pull::Up)?,
            PinDriver::input(peripherals.pins.gpio6, Pull::Up)?,
        );
        let mut back_button =
            LongPressBackButton::new(PinDriver::input(peripherals.pins.gpio0, Pull::Up)?);
        info!(
            "rustmix-wave=boot-button-back status=ready gpio=0 active-low=true short-press=contextual-navigation hold-ms={BOOT_BACK_LONG_PRESS_MS}"
        );
        // The uploaded BSP routes the PCF85063 active-low alarm output to
        // GPIO45. Validate that board-level line before introducing MCU
        // deep-sleep entry in the following isolated power milestone.
        let mut rtc_alarm_interrupt =
            RtcAlarmInterruptMonitor::new(PinDriver::input(peripherals.pins.gpio45, Pull::Up)?);
        info!(
            "rustmix-wave=rtc-alarm-int status=ready gpio={RTC_ALARM_INTERRUPT_GPIO} active-low=true wake-policy=active-loop-readiness"
        );
        let mut button_delay = FreeRtosDelay;
        let mut service_delay = FreeRtosDelay;
        let mut frame = FrameBuffer::new_white();
        // Keep the growing product UI state off the firmware main-task stack.
        // HTTPS weather retrieval and display refreshes still execute from the
        // same orchestrator, but their stack budget is no longer reduced by a
        // long-lived inline AppState allocation.
        let mut state = Box::new(AppState::default());
        let mut panel_refresh = PanelRefreshCoordinator::default();
        sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
        state.display = display_preferences;
        let reader_persistence = state.reader.load_persistent_state();
        state.reader.refresh_library();
        if _mounted_sd.is_some() {
            match cleanup_stale_voice_tmp(std::path::Path::new(VOICE_NOTES_ROOT)) {
                Ok(removed) => info!(
                    "rustmix-wave=voice-note-stale-tmp-cleanup status=completed removed={removed} root={VOICE_NOTES_ROOT}"
                ),
                Err(error) => warn!(
                    "rustmix-wave=voice-note-stale-tmp-cleanup status=failed root={VOICE_NOTES_ROOT} error={error:#}"
                ),
            }
        }
        if _mounted_sd.is_some() {
            match load_voice_notes_preferences(std::path::Path::new(VOICE_NOTES_ROOT)) {
                Ok(preferences) => {
                    state.voice_notes.mic_gain = preferences.mic_gain;
                    info!(
                        "rustmix-wave=voice-note-settings-load status=completed mic-gain={} path={VOICE_NOTES_ROOT}/SETTINGS.TXT",
                        preferences.mic_gain.marker()
                    );
                }
                Err(error) => warn!(
                    "rustmix-wave=voice-note-settings-load status=failed path={VOICE_NOTES_ROOT}/SETTINGS.TXT error={error:#}"
                ),
            }
        }
        refresh_voice_note_storage_available(&mut state, _mounted_sd.is_some());
        state.refresh_voice_notes_catalog();
        state.refresh_lua_app_catalog(_mounted_sd.is_some());
        log_lua_runtime_events(&mut state);
        info!(
            "rustmix-wave=reader-persistence-load state-loaded={} preferences-loaded={} positions={} recent={} bookmarks={} warning={}",
            reader_persistence.state_loaded,
            reader_persistence.preferences_loaded,
            reader_persistence.position_count,
            reader_persistence.recent_count,
            reader_persistence.bookmark_count,
            reader_persistence.warning.as_deref().unwrap_or("none")
        );
        let mut sleep_images = SleepImageCatalog::default();
        let mut sleep_mode = SleepModeState::default();
        let mut sleep_wake_guard = SleepWakeGuard::default();
        let mut sleep_wake_guard_started_at: Option<Instant> = None;
        let mut sleep_network = SleepNetworkState::default();
        state.update_audio_snapshot(initial_audio_snapshot);
        log_audio_snapshot(&state.audio);
        if let Some(config) = network_config.as_ref() {
            state.regional = state.regional.with_timezone_name(&config.timezone)?;
            state.update_network_snapshot(NetworkSnapshot::provisioned(config));
        }
        if let Some(config) = weather_config.as_ref() {
            state.update_weather_snapshot(WeatherSnapshot::provisioned(config));
        }
        state.update_alarm_snapshot(alarm_engine.snapshot());
        state.update_storage_snapshot(storage_browser.snapshot());
        log_storage_snapshot(&state.storage);
        info!(
            "rustmix-wave=regional-profile timezone={} display-offset={} rtc-storage-offset={} temperature-unit={}",
            state.regional.timezone_name(),
            state.regional.timezone_label_for_rtc(state.board.rtc),
            state.regional.rtc_storage_label(),
            state.regional.temperature_unit.marker()
        );

        let init = board_services.initialize(&mut service_delay);
        info!(
            "rustmix-wave=sample-board-services-init rtc={} environment={} power={} imu={} rtc-integrity-lost={} shtc3-id={} qmi8658-address={} qmi8658-revision={}",
            init.rtc_available,
            init.environment_available,
            init.power_monitoring_available,
            init.imu_available,
            init.rtc_clock_integrity_was_lost,
            init.environment_sensor_id
                .map_or_else(|| "unavailable".into(), |id| format!("0x{id:04X}")),
            init.imu_address
                .map_or_else(|| "unavailable".into(), |value| format!("0x{value:02X}")),
            init.imu_revision
                .map_or_else(|| "unavailable".into(), |value| format!("0x{value:02X}"))
        );
        let mut power_key_available = match board_services.initialize_power_key_events() {
            Ok(()) => {
                info!("rustmix-wave=power-key status=ready source=axp2101-pek events=short-menu,long-sleep poll-ms={POWER_KEY_POLL_MS}");
                true
            }
            Err(error) => {
                warn!(
                    "rustmix-wave=power-key status=unavailable source=axp2101-pek error={error:#}"
                );
                false
            }
        };
        state.update_board_snapshot(board_services.read_snapshot(&mut service_delay));
        log_board_snapshot(state.board, state.regional);
        if let Some(rtc) = state.board.rtc {
            alarm_engine.recompute_next(state.regional.localize_rtc(rtc));
        }
        sync_alarm_hardware(&mut alarm_engine, &mut board_services, state.regional);
        state.update_alarm_snapshot(alarm_engine.snapshot());
        log_alarm_snapshot(&state.alarms);

        panel.initialize()?;
        render_current_screen(&mut frame, &state)?;
        panel.show_base(frame.as_bytes())?;
        panel_refresh.reset_after_external_global(PanelGlobalReason::InitialBoot);
        sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
        info!(
            "rustmix-wave=panel-refresh plan=global-base reason=initial-boot transport=global-base"
        );
        info!("rustmix-wave=epd397-rust-display-ready");

        // Start optional networking only after the first e-paper frame is
        // visible. A missing config or failed association never blocks shell
        // startup. Keep the runtime alive so Wi-Fi and SNTP remain active.
        let mut network_runtime = if let Some(config) = network_config.as_ref() {
            info!(
                "rustmix-wave=wifi-connect status=starting ssid={}",
                config.ssid
            );
            match NetworkRuntime::connect(peripherals.modem, config) {
                Ok(runtime) => {
                    info!(
                        "rustmix-wave=wifi-connect status=connected ssid={}",
                        config.ssid
                    );
                    runtime
                }
                Err(error) => {
                    warn!(
                        "rustmix-wave=wifi-connect status=failed ssid={} error={error:#}",
                        config.ssid
                    );
                    NetworkRuntime::failed(config, format!("{error:#}"))
                }
            }
        } else {
            NetworkRuntime::configuration_missing()
        };
        state.update_network_snapshot(network_runtime.snapshot());
        log_network_snapshot(&state.network);
        let mut last_network_log = Instant::now();
        let mut last_network_fingerprint = state.network.log_fingerprint();
        // Explicitly activated only.  Normal boot never starts the portal.
        let mut wifi_transfer_server: Option<WifiTransferServer> = None;
        state.update_wifi_transfer_snapshot(WifiTransferSnapshot::default());
        let mut voice_recording: Option<VoiceRecordingSession> = None;
        let mut voice_playback: Option<VoicePlaybackSession> = None;
        let mut voice_stereo_buffer = vec![0_u8; VOICE_PCM_STEREO_CAPTURE_BYTES];
        let mut voice_mono_buffer = vec![0_u8; VOICE_PCM_MONO_CHUNK_BYTES];
        info!("rustmix-wave=open-meteo-weather-forecast-ready");
        info!("rustmix-wave=open-meteo-fixed-point-json-parser-ready");
        info!("rustmix-wave=open-meteo-whitespace-parser-repair-ready");
        info!("rustmix-wave=rtc-alarm-scheduling-ui-ready");
        info!("rustmix-wave=es8311-audio-diagnostics-audible-alarm-ready");
        info!("rustmix-wave=es8311-pindriver-output-mode-repair-ready");
        info!("rustmix-wave=es8311-i2s-data-route-repair-ready");
        info!("rustmix-wave=es8311-waveshare-codec-profile-repair-ready");
        info!(
            "rustmix-wave=wifi-monitor-log-quieting-ready policy=state-change-or-heartbeat heartbeat-seconds={NETWORK_LOG_HEARTBEAT_SECONDS} rssi-immediate=false"
        );
        info!("rustmix-wave=rtc-alarm-int-readiness-ready gpio={RTC_ALARM_INTERRUPT_GPIO} active-low=true");
        info!("rustmix-wave=power-key-sleep-image-mode-ready path={SLEEP_IMAGE_DIRECTORY} format=native-800x480-1bpp-bmp mcu-sleep=false");
        info!("rustmix-wave=power-key-short-menu-long-sleep-ready short-press=display-maintenance-menu long-press=sleep-image wake=power-key menu-action=manual-global-refresh");
        info!("rustmix-wave=release-flash-workflow-safety-ready docs=consolidated workflow=ci release-artifact=elf supported-flash=espflash-flash factory-image=deferred");
        info!("rustmix-wave=text-editor-layout-alignment-ready voice-title-editor=shared-grid-keyboard calendar-editor-status=compact-date keyboard=boot-hv-axis footer=width-safe");
        info!("rustmix-wave=sd-unicode-indic-epub-reader-ready fonts=/sdcard/RUSTMIX/FONTS manifest=FONTS.TXT packs=RWF1 scripts=devanagari,gujarati family=noto-sans shaping=browser-cluster-raster byte-anchors=preserved");
        info!("rustmix-wave=reader-epub-large-archive-file-backed-ready archive-storage=file-backed archive-max-bytes=67108864 zip-entry-limit=4096 central-dir-max-bytes=2097152 manifest-limit=4096 spine-limit=4096 reflow-max-bytes=7340032 page-anchor-limit=16384 invalid-spine=skip nav-spine=skip");
        info!("rustmix-wave=reader-epub-parser-fragmentation-aware-stack-ready worker=epub-parser preferred-stack-bytes=49152 fallback-stack-bytes=32768 guard-bytes=4096 preflight=largest-internal-block telemetry=before,after");
        info!("rustmix-wave=reader-library-scroll-epub-title-defer-ready visible-rows=7 window=selection-following titles=fat-filename-first opf-title=after-open parser-heap=preserved");
        info!("rustmix-wave=reader-epub-first-page-unicode-subset-ready opening=first-page-first pagination=lazy unicode-fonts=visible-page-subset sd-scan=per-page rwf-memory=bounded alpha-threshold=builder-selectable");
        info!("rustmix-wave=sleep-image-directory-classification-fix-ready policy=fat-metadata-fallback");
        info!("rustmix-wave=network-suspended-sleep-image-mode-ready wifi=stop-on-sleep sntp=paused weather=paused mcu-sleep=false");
        info!("rustmix-wave=random-sleep-image-selection-ready source=esp-random policy=avoid-immediate-repeat-when-multiple");
        info!("rustmix-wave=main-category-navigation-ready categories=5");
        info!("rustmix-wave=reader-category-ready entries=3");
        info!("rustmix-wave=productivity-category-ready entries=2");
        info!("rustmix-wave=games-category-ready entries=1 status=sd-lua-catalog");
        info!("rustmix-wave=tools-category-ready entries=3");
        info!("rustmix-wave=settings-category-ready entries=9 display=true");
        info!("rustmix-wave=display-settings-ready default-family=inter alternate-family=atkinson-hyperlegible default-size=standard profiles=compact,standard,large persistence={DISPLAY_CONFIG_PATH} scope=all-user-facing-screens");
        info!("rustmix-wave=global-ui-typography-ready default-family=inter alternate-family=atkinson-hyperlegible default-size=standard profiles=compact,standard,large persistence={DISPLAY_CONFIG_PATH} scope=all-user-facing-screens");
        info!("rustmix-wave=boot-button-hierarchical-back-ready gpio=0 active-low=true short-press=contextual-navigation hold-ms={BOOT_BACK_LONG_PRESS_MS} policy=long-press-back");
        info!("rustmix-wave=category-back-row-removal-ready policy=boot-long-press");
        info!("rustmix-wave=global-typography-scale-increase-ready shift=two-raster-steps settings-page-size=6 display-copy=compact default-family=inter default-size=standard");
        info!("rustmix-wave=secondary-screen-readability-reflow-ready detail-role=technical-tokens-only pagination=device-info-3-pages details=weather,audio,rtc,environment,motion,network synthetic-back-rows=removed");
        info!("rustmix-wave=weather-fetch-resilience-ready retries=3 backoff-seconds=2,5,15 cache=last-known-good-in-memory retryable=tls-eof,http-connect,timeout,http-429,http-500,http-502,http-503,http-504");
        info!("rustmix-wave=home-dashboard-redesign-ready header=simplified-dark date-time-row=true summary-strip=weather,battery,wifi cards=high-contrast footer=fixed categories=5 developer-notes=removed");
        info!("rustmix-wave=calendar-foundation-ready mode=read-only monthly-view=true selected-day-summary=true range=2000-2099");
        info!(
            "rustmix-wave=calendar-local-date-ready timezone=regional-profile source=rtc-localized"
        );
        info!("rustmix-wave=calendar-navigation-ready modes=day,month select=toggle-mode boot-short=agenda back=boot-long-press");
        info!("rustmix-wave=calendar-us-events-daily-agenda-ready root={CALENDAR_ROOT} personal={CALENDAR_EVENTS_FILE} us={CALENDAR_US_EVENTS_FILE} hindu=excluded markers=month-grid agenda=scrollable details=personal-editor missing-files=safe alarms=separate");
        info!("rustmix-wave=calendar-personal-event-editor-ready writable=EVENTS.TXT temp=EVENTS.TMP backup=EVENTS.BAK operations=create,edit,delete us-holidays=read-only keyboard=boot-hv-axis alarms=separate");
        info!("rustmix-wave=power-key-sleep-entry-wake-guard-ready source=axp2101-pek minimum-quiet-ms={POWER_KEY_WAKE_GUARD_QUIET_MS} policy=suppress-stale-until-quiet-window");
        info!("rustmix-wave=unit-converter-foundation-ready categories=length,mass,temperature,volume mode=offline fixed-point=true precision=thousandths");
        info!("rustmix-wave=unit-converter-navigation-ready fields=category,from-unit,value,to-unit,step-size back=boot-long-press");
        info!("rustmix-wave=unit-converter-host-tests-ready coverage=length,mass,temperature,volume,bounds");
        info!("rustmix-wave=reader-library-txt-foundation-ready path=/sdcard/RUSTMIX/BOOKS formats=txt,epub encoding=utf8,bom,windows-1252 opening=staged-first-page-first cache=ram-nearby-pages");
        info!("rustmix-wave=reader-state-persistence-ready path=/sdcard/RUSTMIX/READER files=STATE.TXT,POSITS.TXT,RECENT.TXT,MARKS.TXT cache=CACHE atomic-replace=tmp-primary-backup fallback=corrupt-record-safe");
        info!("rustmix-wave=reader-bookmarks-ready add-remove=true list=true recent=true continue-reading=true cache-fingerprint=path,size,modified,format,layout");
        info!("rustmix-wave=reader-loading-ui-ready stages=open,encoding,resume,first-page,cache cancel=boot-long-press refresh=coarse-stage-boundaries");
        info!("rustmix-wave=reader-options-shell-ready toc=none-for-txt,list-for-epub bookmarks=persistent clear-ghosting=manual-global-refresh");
        info!("rustmix-wave=reader-ux-repair-ready menu=continue,library,bookmarks-ready normalization=utf8-punctuation,latin1,underscore-emphasis byte-offsets=preserved");
        info!("rustmix-wave=reader-preferences-ready path=/sdcard/RUSTMIX/READER/PREFS.TXT theme=classic,high-contrast orientation=portrait,landscape font-size=small,medium,large,xlarge book-font=inter,atkinson-hyperlegible,serif,literata paragraph-alignment=justified,left,center,right show-progress=on,off atomic-replace=tmp-primary-backup");
        info!("rustmix-wave=reader-high-contrast-layout-ready viewport=shared border=outside-text top-padding=true clip=right,bottom theme-change=redraw-only ghost-refresh=global-base");
        info!("rustmix-wave=reader-txt-emphasis-cleanup-ready multiline-gutenberg=true word-internal-underscores=preserved repeated-separators=preserved byte-offsets=preserved");
        info!("rustmix-wave=reader-per-book-resume-ready path=/sdcard/RUSTMIX/READER/POSITS.TXT records=64 fingerprint=path,size,modified,format atomic-replace=tmp-primary-backup routes=continue,books,files,bookmark");
        info!("rustmix-wave=reader-controls-alignment-ready navigation=up-down-move-select-activate preferences=up-down-move-select-change back=boot-long-press");
        info!("rustmix-wave=reader-options-split-ready actions=bookmark,toc,preferences,clear-ghosting,library,home editor=theme,orientation,font-size,font,paragraph-alignment,show-progress");
        info!("rustmix-wave=reader-preferences-settings-navigation-ready move=up-down change=select back=boot-long-press persistence=immediate rows=theme,orientation,font-size,font,paragraph-alignment,show-progress");
        info!("rustmix-wave=reader-fat83-persistence-ready positions=POSITS.TXT legacy-read=POSITIONS.TXT cache-basename=8hex extensions=CCH,TMP,BAK atomic-replace=true");
        info!("rustmix-wave=reader-fat83-runtime-ready positions-write=POSITS.TXT legacy-read=POSITIONS.TXT cache-write=8hex-no-prefix extensions=CCH,TMP,BAK duplicate-degraded-log=suppressed");
        info!("rustmix-wave=reader-bookmark-page-labels-ready anchor=byte-offset display=page-number layout-aware=true fallback=stored-page");
        info!("rustmix-wave=library-bookmark-tab-rendering-ready status=saved-marks source=MARKS.TXT rows=title,page-number anchors=byte-offset page-label=layout-aware-fallback-stored books-files=txt-open preserved=true");
        info!("rustmix-wave=reader-epub-reflowable-foundation-ready archive=zip-central-directory compression=stored,deflate package=container-xml,opf spine=xhtml reflow=bounded-utf8 cache=ram-nearby-pages");
        info!("rustmix-wave=reader-epub-toc-ready sources=epub3-nav,epub2-ncx,fallback-spine route=reader-toc selection=byte-offset");
        info!("rustmix-wave=reader-epub-parser-stack-isolation-ready worker=epub-parser preferred-stack-bytes=49152 fallback-stack-bytes=32768 guard-bytes=4096 main-task-stack-bytes=16384 policy=short-lived-worker-join");
        info!("rustmix-wave=reader-epub-chapter-aware-presentation-ready page-label=chapter,page-of-total bookmarks=chapter,page-of-total library-title=fat-filename-first session-title=opf-metadata-after-open txt-path=preserved");
        info!("rustmix-wave=reader-epub-watchdog-memory-pressure-repair-ready index-yield-every-pages=4 index-yield-ms=1 session-release=before-book-open layout-rebuild=move-document toc-jump=no-document-clone parser-worker-preferred-stack-bytes=49152 parser-worker-fallback-stack-bytes=32768 parser-worker-guard-bytes=4096 library-title-worker=deferred-to-open");
        info!("rustmix-wave=reader-eink-font-pack-ready fonts=inter,atkinson-hyperlegible,serif,literata atkinson-source=atkinson-hyperlegible-next-medium literata-source=literata-medium glyphs=printable-ascii persisted-keys=serif,atkinson-hyperlegible cache-fingerprint=book-font epub-repagination=layout-rebuild bookmarks=byte-offset txt-epub-aligned=true");
        info!("rustmix-wave=lua-runtime-foundation-ready mode=bootstrap-static,event-bridge root={LUA_APPS_DIRECTORY} manifest=APP.TOM entry=MAIN.LUA script-max-bytes=65536 vm-callbacks=sudoku,minesweeper,tilt-maze,motion-2048,sokoban-tilt-bounded-native");
        info!("rustmix-wave=lua-native-dirty-region-canvas-ready commands=256 text-bytes=160 dirty-regions={MAX_DIRTY_REGIONS} partial-limit={PANEL_PARTIAL_REFRESH_LIMIT} transport=existing-fullscreen-partial panel-api=rust-owned");
        info!("rustmix-wave=panel-refresh-coordinator-ready partial-limit={PANEL_PARTIAL_REFRESH_LIMIT} transport=existing-fullscreen-partial state=main-loop-owned lua-route-global-refresh=false");
        info!("rustmix-wave=runtime-worker-boundary-ready workers=weather-fetch,lua-loader policy=short-lived-named-stack panel-spi=main-task-only");
        info!("rustmix-wave=lua-loader-stack-isolation-ready worker=lua-loader stack-bytes={LUA_LOADER_WORKER_STACK_BYTES} main-task-stack-bytes=16384 policy=short-lived-worker-join");
        info!("rustmix-wave=lua-sudoku-event-bridge-ready sample=SUDOKU input=up,down,select,boot-short-context board=native dirty=old-cell,new-cell,status refresh=shared-panel-coordinator transport=existing-fullscreen-partial panel-api=rust-owned");
        info!("rustmix-wave=lua-sudoku-boot-axis-navigation-ready short-press=boot nav=axis-toggle edit=cancel default-axis=horizontal long-press=hierarchical-back dirty=status-or-cell refresh=shared-panel-coordinator");
        info!("rustmix-wave=lua-sudoku-boot-mode-ux-repair-ready nav=boot-short-axis-toggle edit=boot-short-cancel long-press=hierarchical-back dirty=axis-status-or-edit-cell-status refresh=shared-panel-coordinator");
        info!("rustmix-wave=lua-minesweeper-event-bridge-ready sample=MINES board=beginner-9x9 mines=10 first-reveal=safe input=up,down,select,boot-short-context action=reveal,flag dirty=old-cell,new-cell,status-or-board refresh=shared-panel-coordinator transport=existing-fullscreen-partial panel-api=rust-owned");
        info!("rustmix-wave=imu-event-bridge-ready events=tilt,shake,rotate,level sampling=motion-events-or-motion-game sample-ms={IMU_EVENT_SAMPLE_INTERVAL_MS} diagnostics=thresholds,debounce,counters redraw=event-or-{IMU_EVENT_SCREEN_REFRESH_SECONDS}s-heartbeat raw-i2c=rust-owned lua-api=none");
        info!("rustmix-wave=imu-event-thresholds tilt-mg={} shake-delta-mg={} rotate-dps={} level-tolerance-mg={} debounce-ms={}", state.imu_events.thresholds.tilt_enter_mg, state.imu_events.thresholds.shake_delta_mg, state.imu_events.thresholds.rotate_dps, state.imu_events.thresholds.level_tolerance_mg, state.imu_events.thresholds.debounce_ms);
        info!("rustmix-wave=imu-event-discrete-latching-ready tilt=release-to-neutral rotate=release-to-neutral level=edge-only shake=cooldown raw-i2c=rust-owned");
        info!("rustmix-wave=lua-tilt-maze-event-bridge-ready sample=TILTMAZE board=9x9 motion=debounced-tilt-only dirty=old-cell,new-cell,status-or-board refresh=shared-panel-coordinator transport=existing-fullscreen-partial panel-api=rust-owned");
        info!("rustmix-wave=lua-tilt-maze-portrait-axis-repair-ready logical=portrait mapping=raw:+x->down,-x->up,+y->left,-y->right diagnostics=logical-direction,raw-axis");
        info!("rustmix-wave=lua-motion-2048-event-bridge-ready sample=M2048 board=4x4 motion=debounced-tilt-swipe dirty=board,status refresh=shared-panel-coordinator transport=existing-fullscreen-partial panel-api=rust-owned");
        info!("rustmix-wave=lua-sokoban-tilt-event-bridge-ready sample=SOKOBAN board=9x9 motion=debounced-tilt-only dirty=old-cell,new-cell,status-or-board refresh=shared-panel-coordinator transport=existing-fullscreen-partial panel-api=rust-owned");
        info!("rustmix-wave=weather-fetch-stack-isolation-ready worker=weather-fetch stack-bytes=65536 main-task-stack-bytes=16384 response-max-bytes=8192 state=heap-boxed policy=short-lived-worker-join");
        info!("rustmix-wave=wifi-transfer-web-portal-ready activation=settings-network-explicit-toggle auto-start=false root={WIFI_TRANSFER_ROOT} transport=http-lan-only token=required server-stack-bytes={WIFI_TRANSFER_SERVER_STACK_BYTES} main-task-stack-bytes=16384 upload=streamed-atomic-tmp fat83=true protected-config=true inactivity-seconds={WIFI_TRANSFER_INACTIVITY_SECONDS}");
        log_runtime_memory("boot-complete");
        info!("rustmix-wave=hierarchical-router-ready policy=category-subcategory-feature-details");
        info!("rustmix-wave=wifi-transfer-lifecycle-ready state=off-until-settings-network-toggle server=temporary-http-task sd-root=/sdcard/RUSTMIX stop=switch-off,back,sleep,wifi-loss,inactivity");
        info!("rustmix-wave=wifi-transfer-immediate-start-redraw-repair-ready dispatch=ordinary-button-event-before-refresh snapshot=ready-url-code refresh=single-normal-partial");
        info!("rustmix-wave=voice-notes-foundation-ready root={VOICE_NOTES_ROOT} format=wav-pcm16-mono-16khz storage=streamed-tmp-rename capture=cooperative-bounded-i2s-rx chunk-bytes={VOICE_PCM_MONO_CHUNK_BYTES} main-task-stack-bytes=16384 audio-owner=native");
        info!("rustmix-wave=voice-notes-microphone-gain-ready profiles=low,normal,high,boost default=high multipliers=1x,2x,3x,4x clipping=per-recording-saturated-sample-count wav-format=unchanged");
        info!("rustmix-wave=voice-notes-fat-metadata-catalog-repair-ready policy=stat-metadata-final-classification overwrite=refuse-existing-target");
        info!("rustmix-wave=voice-notes-catalog-scrolling-saved-wav-playback-ready visible-rows=6 format=wav-pcm16-mono-16khz playback=bounded-sd-stream mono-to-stereo=true volume=existing-codec-setting audio-owner=native stale-tmp-cleanup=boot alarms=interrupt");
        info!("rustmix-wave=voice-notes-organizer-controls-export-ready gain-persistence=SETTINGS.TXT metadata=META.TXT titles=friendly-sidecar filenames=fat83-wav recording-date-time=rtc-local storage=esp-vfs-fat-info delete-confirmation=true pause-resume=rx-discard export=wifi-transfer-shortcut");
        info!("rustmix-wave=offline-dictionary-x4-pack-native-foundation-ready root={DICTIONARY_ROOT} index=INDEX.TXT shards=DATA/*.JSN shard-max-bytes={DICTIONARY_SHARD_MAX_BYTES} lookup=exact-prefix-fallback wildcard=true ui=native-rust");
        info!("rustmix-wave=dictionary-keyboard-boot-axis-navigation-ready short-press=boot toggle=horizontal,vertical default-axis=horizontal selected-key=preserved long-press=hierarchical-back helper=keyboard-grid-navigation");
        info!(
            "rustmix-wave=voice-notes-catalog status=completed notes={} root={VOICE_NOTES_ROOT}",
            state.voice_notes.notes.len()
        );

        let mut last_activity = Instant::now();
        let mut last_status_refresh = Instant::now();
        let mut last_alarm_poll = Instant::now();
        let mut last_power_key_poll = Instant::now();
        let mut last_weather_attempt: Option<Instant> = None;
        let mut last_reader_tick = Instant::now();
        let imu_event_started_at = Instant::now();
        let mut last_imu_event_sample = Instant::now();
        let mut last_imu_event_screen_refresh = Instant::now();
        let mut weather_retry = WeatherRetryState::default();
        let mut last_voice_record_refresh = Instant::now();
        loop {
            maintain_wifi_transfer_server(
                &mut wifi_transfer_server,
                &mut state,
                &mut storage_browser,
                _mounted_sd.is_some(),
            );
            if state.panel_awake
                && last_activity.elapsed() >= Duration::from_secs(PANEL_IDLE_SLEEP_SECONDS)
            {
                panel.sleep()?;
                state.panel_awake = false;
                info!("rustmix-wave=epd397-panel-sleep");
            }

            let mut voice_capture_failure = None;
            if let Some(session) = voice_recording.as_mut() {
                if state.voice_notes.recording_paused {
                    let discard = audio_runtime
                        .as_mut()
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "audio runtime unavailable during paused voice recording"
                            )
                        })
                        .and_then(|runtime| runtime.discard_voice_pcm(&mut voice_stereo_buffer));
                    if let Err(error) = discard {
                        voice_capture_failure = Some(format!("{error:#}"));
                    }
                } else {
                    let capture = audio_runtime
                        .as_mut()
                        .ok_or_else(|| {
                            anyhow::anyhow!("audio runtime unavailable during voice recording")
                        })
                        .and_then(|runtime| {
                            runtime.read_voice_pcm_mono(
                                &mut voice_stereo_buffer,
                                &mut voice_mono_buffer,
                                state.voice_notes.mic_gain,
                            )
                        });
                    match capture {
                        Ok(metrics) if metrics.bytes > 0 => {
                            session.add_clipped_samples(metrics.clipped_samples);
                            if let Err(error) =
                                session.append_pcm16_mono(&voice_mono_buffer[..metrics.bytes])
                            {
                                voice_capture_failure = Some(format!("{error:#}"));
                            } else {
                                state.voice_notes.update_recording_progress(
                                    session.pcm_bytes(),
                                    session.peak(),
                                    session.clipped_samples(),
                                );
                            }
                        }
                        Ok(_) => {}
                        Err(error) => voice_capture_failure = Some(format!("{error:#}")),
                    }
                }
                if voice_capture_failure.is_none()
                    && state.panel_awake
                    && state.active_route() == ScreenRoute::VoiceNoteRecording
                    && last_voice_record_refresh.elapsed()
                        >= Duration::from_secs(VOICE_RECORD_SCREEN_REFRESH_SECONDS)
                {
                    if state.voice_notes.recording_paused {
                        info!("rustmix-wave=voice-record status=paused file={} elapsed-seconds={} pcm-bytes={} peak={} clipped-samples={} mic-gain={}", session.file_name(), state.voice_notes.elapsed_seconds, session.pcm_bytes(), session.peak(), session.clipped_samples(), state.voice_notes.mic_gain.marker());
                    } else {
                        info!("rustmix-wave=voice-record status=active file={} elapsed-seconds={} pcm-bytes={} peak={} clipped-samples={} mic-gain={}", session.file_name(), state.voice_notes.elapsed_seconds, session.pcm_bytes(), session.peak(), session.clipped_samples(), state.voice_notes.mic_gain.marker());
                    }
                    refresh_screen(
                        &mut panel,
                        &mut frame,
                        &mut state,
                        &mut panel_refresh,
                        RefreshRequest::Normal,
                    )?;
                    last_voice_record_refresh = Instant::now();
                }
            }
            if let Some(error) = voice_capture_failure {
                warn!("rustmix-wave=voice-record status=failed stage=capture error={error}");
                if let Some(active) = voice_recording.take() {
                    let _ = active.cancel();
                }
                if let Some(runtime) = audio_runtime.as_mut() {
                    let _ = runtime.finish_voice_recording();
                    state.update_audio_snapshot(runtime.snapshot());
                }
                state.voice_notes.fail(error);
                log_runtime_memory("after-voice-record-stop");
            }

            let mut voice_playback_finished = None;
            let mut voice_playback_failure = None;
            if voice_recording.is_none() {
                if let Some(session) = voice_playback.as_mut() {
                    match session.read_pcm16_mono(&mut voice_mono_buffer) {
                        Ok(0) => voice_playback_finished = Some(session.file_name().to_string()),
                        Ok(bytes) => {
                            let output = audio_runtime
                                .as_mut()
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "audio runtime unavailable during voice-note playback"
                                    )
                                })
                                .and_then(|runtime| {
                                    runtime.write_voice_pcm16_mono(
                                        &voice_mono_buffer[..bytes],
                                        &mut voice_stereo_buffer,
                                    )
                                });
                            match output {
                                Ok(()) => {
                                    state.voice_notes.update_playback_progress(
                                        session.played_pcm_bytes(),
                                        session.total_pcm_bytes(),
                                    );
                                    if session.is_complete() {
                                        voice_playback_finished =
                                            Some(session.file_name().to_string());
                                    }
                                }
                                Err(error) => {
                                    voice_playback_failure = Some(format!("{error:#}"));
                                }
                            }
                        }
                        Err(error) => voice_playback_failure = Some(format!("{error:#}")),
                    }
                }
            }
            if let Some(file_name) = voice_playback_finished {
                stop_voice_note_playback(
                    &mut voice_playback,
                    &mut audio_runtime,
                    &mut state,
                    "completed",
                );
                info!("rustmix-wave=voice-note-playback status=completed file={file_name}");
                if state.panel_awake && state.active_route() == ScreenRoute::VoiceNoteDetails {
                    refresh_screen(
                        &mut panel,
                        &mut frame,
                        &mut state,
                        &mut panel_refresh,
                        RefreshRequest::Normal,
                    )?;
                }
            }
            if let Some(error) = voice_playback_failure {
                warn!("rustmix-wave=voice-note-playback status=failed error={error}");
                stop_voice_note_playback(
                    &mut voice_playback,
                    &mut audio_runtime,
                    &mut state,
                    "stream-error",
                );
                state.voice_notes.fail(format!("Playback failed: {error}"));
                if state.panel_awake && state.active_route() == ScreenRoute::VoiceNoteDetails {
                    refresh_screen(
                        &mut panel,
                        &mut frame,
                        &mut state,
                        &mut panel_refresh,
                        RefreshRequest::Normal,
                    )?;
                }
            }

            if voice_recording.is_none() && voice_playback.is_none() {
                if let Some(runtime) = audio_runtime.as_mut() {
                    match runtime.tick() {
                        Ok(changed) => {
                            let latest = runtime.snapshot();
                            if latest != state.audio {
                                state.update_audio_snapshot(latest);
                                log_audio_snapshot(&state.audio);
                                if changed
                                    && state.panel_awake
                                    && matches!(
                                        state.active_route(),
                                        ScreenRoute::Audio
                                            | ScreenRoute::AudioDetails
                                            | ScreenRoute::Alarms
                                    )
                                {
                                    refresh_screen(
                                        &mut panel,
                                        &mut frame,
                                        &mut state,
                                        &mut panel_refresh,
                                        RefreshRequest::Normal,
                                    )?;
                                }
                            }
                        }
                        Err(error) => {
                            warn!(
                                "rustmix-wave=audio-event outcome=playback-error error={error:#}"
                            );
                            runtime.record_failure(format!("{error:#}"));
                            state.update_audio_snapshot(runtime.snapshot());
                            log_audio_snapshot(&state.audio);
                        }
                    }
                }
            }

            if !sleep_network.is_suspended() {
                if let Some(utc) = network_runtime.tick() {
                    info!(
                        "rustmix-wave=sntp-sync status=completed utc={}",
                        utc.date_time()
                    );
                    match board_services.sync_rtc_from_utc(utc) {
                        Ok(stored) => info!(
                            "rustmix-wave=rtc-sync status=updated storage-basis={} stored={}",
                            state.regional.rtc_storage_label(),
                            stored.date_time()
                        ),
                        Err(error) => warn!("rustmix-wave=rtc-sync status=failed error={error:#}"),
                    }
                    state.update_board_snapshot(board_services.read_snapshot(&mut service_delay));
                    log_board_snapshot(state.board, state.regional);
                    if let Some(rtc) = state.board.rtc {
                        alarm_engine.recompute_next(state.regional.localize_rtc(rtc));
                        sync_alarm_hardware(&mut alarm_engine, &mut board_services, state.regional);
                        state.update_alarm_snapshot(alarm_engine.snapshot());
                        log_alarm_snapshot(&state.alarms);
                    }
                }
                let latest_network = network_runtime.snapshot();
                if latest_network != state.network {
                    state.update_network_snapshot(latest_network);
                }
                let latest_fingerprint = state.network.log_fingerprint();
                if latest_fingerprint != last_network_fingerprint
                    || last_network_log.elapsed()
                        >= Duration::from_secs(NETWORK_LOG_HEARTBEAT_SECONDS)
                {
                    log_network_snapshot(&state.network);
                    last_network_fingerprint = latest_fingerprint;
                    last_network_log = Instant::now();
                }
            }

            if alarm_engine.should_poll()
                && last_alarm_poll.elapsed() >= Duration::from_secs(ALARM_POLL_SECONDS)
            {
                match board_services.read_rtc() {
                    Ok(rtc) => {
                        let local = state.regional.localize_rtc(rtc);
                        let interrupt_sample = rtc_alarm_interrupt.sample();
                        if interrupt_sample.changed {
                            info!(
                                "rustmix-wave=rtc-alarm-int status={} gpio={RTC_ALARM_INTERRUPT_GPIO} level={}",
                                interrupt_sample.level.marker(),
                                interrupt_sample.level.raw_level_marker()
                            );
                        }
                        let hardware_flag = match board_services.take_rtc_alarm_flag() {
                            Ok(flag) => flag,
                            Err(error) => {
                                warn!("rustmix-wave=rtc-alarm-flag status=unavailable error={error:#}");
                                false
                            }
                        };
                        let outcome =
                            alarm_engine.poll(local, hardware_flag || interrupt_sample.asserted());
                        if outcome.schedule_changed {
                            sync_alarm_hardware(
                                &mut alarm_engine,
                                &mut board_services,
                                state.regional,
                            );
                        }
                        state.update_alarm_snapshot(alarm_engine.snapshot());
                        if outcome.triggered {
                            if let Some(active) = voice_recording.take() {
                                let _ = active.cancel();
                                if let Some(runtime) = audio_runtime.as_mut() {
                                    let _ = runtime.finish_voice_recording();
                                    state.update_audio_snapshot(runtime.snapshot());
                                }
                                state.voice_notes.cancel_recording();
                                info!("rustmix-wave=voice-record status=cancelled reason=alarm-trigger");
                            }
                            if voice_playback.is_some() {
                                stop_voice_note_playback(
                                    &mut voice_playback,
                                    &mut audio_runtime,
                                    &mut state,
                                    "alarm-trigger",
                                );
                            }
                            info!(
                                "rustmix-wave=alarm-triggered active={} local={} hardware-flag={hardware_flag} interrupt-low={}",
                                state.alarms.active.as_ref().map_or("alarm", |active| active.name.as_str()),
                                local.date_time(),
                                interrupt_sample.asserted()
                            );
                            if let Some(runtime) = audio_runtime.as_mut() {
                                match runtime.start_alarm_chime() {
                                    Ok(()) => {
                                        info!("rustmix-wave=audio-event outcome=alarm-chime-start")
                                    }
                                    Err(error) => {
                                        warn!("rustmix-wave=audio-event outcome=alarm-chime-failed error={error:#}");
                                        runtime.record_failure(format!("{error:#}"));
                                    }
                                }
                                state.update_audio_snapshot(runtime.snapshot());
                                log_audio_snapshot(&state.audio);
                            } else {
                                warn!("rustmix-wave=audio-event outcome=alarm-chime-unavailable fallback=visual-only");
                            }
                            let woke_from_sleep = !state.panel_awake;
                            if sleep_mode.is_sleeping() {
                                let _ = sleep_mode.exit(SleepWakeCause::RtcAlarm);
                                sleep_wake_guard.reset_after_wake();
                                sleep_wake_guard_started_at = None;
                                info!("rustmix-wave=sleep-mode-exit cause=rtc-alarm restore-route=alarms");
                            }
                            if woke_from_sleep {
                                panel.initialize()?;
                                state.panel_awake = true;
                                panel_refresh
                                    .reset_after_external_global(PanelGlobalReason::AfterWake);
                                sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                            }
                            state.router.navigate_to(ScreenRoute::Alarms);
                            info!("rustmix-wave=screen-route route=alarms cause=alarm-trigger");
                            if woke_from_sleep {
                                info!(
                                    "rustmix-wave=wake-global-refresh reason=rtc-alarm-sleep-image"
                                );
                            }
                            refresh_screen(
                                &mut panel,
                                &mut frame,
                                &mut state,
                                &mut panel_refresh,
                                if woke_from_sleep {
                                    RefreshRequest::ForceGlobalAfterWake
                                } else {
                                    RefreshRequest::Normal
                                },
                            )?;
                            if sleep_network.is_suspended() {
                                resume_network_after_sleep(
                                    &mut network_runtime,
                                    network_config.as_ref(),
                                    &mut state,
                                    &mut sleep_network,
                                    &mut last_network_fingerprint,
                                    &mut last_network_log,
                                    &mut last_weather_attempt,
                                    &mut weather_retry,
                                );
                            }
                            last_activity = Instant::now();
                            last_status_refresh = Instant::now();
                        }
                    }
                    Err(error) => {
                        warn!("rustmix-wave=rtc-alarm-poll status=unavailable error={error:#}")
                    }
                }
                last_alarm_poll = Instant::now();
            }

            if sleep_mode.is_sleeping() {
                if let Some(started_at) = sleep_wake_guard_started_at.as_ref() {
                    let elapsed_ms = started_at.elapsed().as_millis() as u64;
                    if sleep_wake_guard.arm_after_quiet_window(elapsed_ms) {
                        info!(
                            "rustmix-wave=sleep-wake-guard status=ready-for-new-wake-press minimum-quiet-ms={POWER_KEY_WAKE_GUARD_QUIET_MS}"
                        );
                    }
                }
            }

            if power_key_available
                && last_power_key_poll.elapsed() >= Duration::from_millis(POWER_KEY_POLL_MS)
            {
                match board_services.take_power_key_event() {
                    Ok(Some(event)) => {
                        info!(
                            "rustmix-wave=power-key event={} source=axp2101-pek",
                            event.marker()
                        );
                        if sleep_mode.is_sleeping() {
                            let elapsed_ms = sleep_wake_guard_started_at
                                .as_ref()
                                .map_or(0, |started_at| started_at.elapsed().as_millis() as u64);
                            if sleep_wake_guard.on_power_press(elapsed_ms)
                                == SleepWakeGuardDecision::SuppressStalePress
                            {
                                info!(
                                    "rustmix-wave=sleep-wake-guard event=stale-power-key-suppressed source=axp2101-pek elapsed-ms={elapsed_ms} minimum-quiet-ms={POWER_KEY_WAKE_GUARD_QUIET_MS}"
                                );
                                last_power_key_poll = Instant::now();
                                continue;
                            }
                            sleep_wake_guard.reset_after_wake();
                            sleep_wake_guard_started_at = None;
                            let restore_route = sleep_mode.exit(SleepWakeCause::PowerKey);
                            panel.initialize()?;
                            state.panel_awake = true;
                            state.router.navigate_to(restore_route);
                            render_current_screen(&mut frame, &state)?;
                            panel.show_base(frame.as_bytes())?;
                            panel_refresh.reset_after_external_global(PanelGlobalReason::AfterWake);
                            sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                            info!("rustmix-wave=panel-refresh plan=global-base reason=after-wake transport=global-base");
                            info!(
                                "rustmix-wave=sleep-mode-exit cause=power-key restore-route={}",
                                restore_route.marker()
                            );
                            info!("rustmix-wave=wake-global-refresh reason=power-key-sleep-image");
                            if sleep_network.is_suspended() {
                                resume_network_after_sleep(
                                    &mut network_runtime,
                                    network_config.as_ref(),
                                    &mut state,
                                    &mut sleep_network,
                                    &mut last_network_fingerprint,
                                    &mut last_network_log,
                                    &mut last_weather_attempt,
                                    &mut weather_retry,
                                );
                            }
                            last_activity = Instant::now();
                            last_status_refresh = Instant::now();
                        } else if event == PowerKeyEvent::ShortPress {
                            if state.alarms.active.is_some() {
                                warn!(
                                    "rustmix-wave=power-key-menu outcome=rejected reason=active-alarm"
                                );
                            } else {
                                state.open_power_key_menu();
                                refresh_screen(
                                    &mut panel,
                                    &mut frame,
                                    &mut state,
                                    &mut panel_refresh,
                                    RefreshRequest::Normal,
                                )?;
                                info!(
                                    "rustmix-wave=power-key-menu outcome=opened return-route={}",
                                    state.power_key_sleep_restore_route().marker()
                                );
                                last_activity = Instant::now();
                                last_status_refresh = Instant::now();
                            }
                        } else if state.alarms.active.is_some() {
                            warn!(
                                "rustmix-wave=sleep-mode-enter status=rejected reason=active-alarm"
                            );
                        } else {
                            stop_wifi_transfer_server(
                                &mut wifi_transfer_server,
                                &mut state,
                                &mut storage_browser,
                                _mounted_sd.is_some(),
                                "sleep-entry",
                            );
                            if let Some(active) = voice_recording.take() {
                                let _ = active.cancel();
                                if let Some(runtime) = audio_runtime.as_mut() {
                                    let _ = runtime.finish_voice_recording();
                                    state.update_audio_snapshot(runtime.snapshot());
                                }
                                state.voice_notes.cancel_recording();
                                info!(
                                    "rustmix-wave=voice-record status=cancelled reason=sleep-entry"
                                );
                            }
                            if voice_playback.is_some() {
                                stop_voice_note_playback(
                                    &mut voice_playback,
                                    &mut audio_runtime,
                                    &mut state,
                                    "sleep-entry",
                                );
                            }
                            if let Some(runtime) = audio_runtime.as_mut() {
                                match runtime.stop_playback() {
                                    Ok(()) => info!("rustmix-wave=audio-event outcome=playback-stop reason=sleep-mode"),
                                    Err(error) => {
                                        warn!("rustmix-wave=audio-event outcome=playback-stop-failed reason=sleep-mode error={error:#}");
                                        runtime.record_failure(format!("{error:#}"));
                                    }
                                }
                                state.update_audio_snapshot(runtime.snapshot());
                                log_audio_snapshot(&state.audio);
                            }
                            let selection =
                                sleep_images.select_random(unsafe { sys::esp_random() });
                            log_sleep_image_selection(&selection);
                            if !suspend_network_for_sleep(
                                &mut network_runtime,
                                &mut state,
                                &mut sleep_network,
                                &mut last_network_fingerprint,
                                &mut last_network_log,
                            ) {
                                warn!("rustmix-wave=sleep-mode-enter status=rejected reason=network-suspend-failed");
                                last_activity = Instant::now();
                                continue;
                            }
                            if !state.panel_awake {
                                panel.initialize()?;
                                state.panel_awake = true;
                            }
                            let restore_route = state.power_key_sleep_restore_route();
                            frame = selection.frame;
                            panel.show_base(frame.as_bytes())?;
                            panel_refresh
                                .reset_after_external_global(PanelGlobalReason::SleepImage);
                            sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                            info!("rustmix-wave=panel-refresh plan=global-base reason=sleep-image transport=global-base");
                            sleep_mode.enter(restore_route, selection.file_name.clone());
                            sleep_wake_guard.begin_sleep_entry();
                            sleep_wake_guard_started_at = Some(Instant::now());
                            info!(
                                "rustmix-wave=sleep-wake-guard status=waiting-for-quiet-window minimum-quiet-ms={POWER_KEY_WAKE_GUARD_QUIET_MS} policy=suppress-stale-power-key"
                            );
                            panel.sleep()?;
                            state.panel_awake = false;
                            info!(
                                "rustmix-wave=sleep-mode-enter image={} restore-route={} display=global-refresh panel=deep-sleep aldo3=off wifi=off network-services=paused mcu-sleep=false",
                                selection.file_name,
                                restore_route.marker()
                            );
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        power_key_available = false;
                        warn!("rustmix-wave=power-key status=unavailable source=axp2101-pek error={error:#}");
                    }
                }
                last_power_key_poll = Instant::now();
            }

            let manual_weather_refresh = state.take_weather_refresh_request();
            if !sleep_network.is_suspended() {
                if let Some(config) = weather_config.as_ref() {
                    if manual_weather_refresh {
                        weather_retry.clear();
                    }
                    let wifi_connected = state.network.wifi_state == WifiConnectionState::Connected;
                    let interval_due = last_weather_attempt.map_or(true, |last| {
                        last.elapsed()
                            >= Duration::from_secs(config.refresh_minutes.saturating_mul(60))
                    });
                    let scheduled_retry = if wifi_connected {
                        weather_retry.take_due()
                    } else {
                        None
                    };
                    let attempt = if manual_weather_refresh {
                        Some(WeatherFetchAttempt::initial("manual"))
                    } else if let Some(retry) = scheduled_retry {
                        Some(retry)
                    } else if interval_due && !weather_retry.is_pending() {
                        Some(WeatherFetchAttempt::initial(
                            if last_weather_attempt.is_none() {
                                "network-ready"
                            } else {
                                "periodic"
                            },
                        ))
                    } else {
                        None
                    };

                    if let Some(attempt) = attempt {
                        if wifi_connected {
                            run_weather_fetch_attempt(
                                config,
                                attempt,
                                &mut weather_retry,
                                &mut state,
                            );
                            last_weather_attempt = Some(Instant::now());
                            if state.panel_awake
                                && matches!(
                                    state.active_route(),
                                    ScreenRoute::Home
                                        | ScreenRoute::Weather
                                        | ScreenRoute::WeatherDetails
                                )
                            {
                                refresh_screen(
                                    &mut panel,
                                    &mut frame,
                                    &mut state,
                                    &mut panel_refresh,
                                    RefreshRequest::Normal,
                                )?;
                            }
                        } else if manual_weather_refresh {
                            state.weather.record_failure("Wi-Fi is not connected");
                            warn!("rustmix-wave=weather-fetch status=deferred cause=manual error=wifi-not-connected");
                            if state.panel_awake
                                && matches!(
                                    state.active_route(),
                                    ScreenRoute::Weather | ScreenRoute::WeatherDetails
                                )
                            {
                                refresh_screen(
                                    &mut panel,
                                    &mut frame,
                                    &mut state,
                                    &mut panel_refresh,
                                    RefreshRequest::Normal,
                                )?;
                            }
                        }
                    }
                } else if manual_weather_refresh {
                    state
                        .weather
                        .record_failure("weather configuration is missing");
                    warn!("rustmix-wave=weather-fetch status=deferred cause=manual error=weather-config-missing");
                    if state.panel_awake
                        && matches!(
                            state.active_route(),
                            ScreenRoute::Weather | ScreenRoute::WeatherDetails
                        )
                    {
                        refresh_screen(
                            &mut panel,
                            &mut frame,
                            &mut state,
                            &mut panel_refresh,
                            RefreshRequest::Normal,
                        )?;
                    }
                }
            }

            if !sleep_mode.is_sleeping()
                && matches!(
                    state.active_route(),
                    ScreenRoute::ReaderLoading | ScreenRoute::ReaderPage
                )
                && last_reader_tick.elapsed() >= Duration::from_millis(250)
            {
                let previous_route = state.active_route();
                let outcome = state.tick_reader();
                match outcome {
                    ReaderTickOutcome::LoadingStageChanged => {
                        info!(
                            "rustmix-wave=reader-cache-stage route={} stage={}",
                            state.active_route().marker(),
                            state
                                .reader
                                .loading_stage()
                                .map_or("none", |stage| stage.label())
                        );
                    }
                    ReaderTickOutcome::FirstPageReady => {
                        info!("rustmix-wave=reader-first-page-ready route={} cache-policy=lazy-nearby-pages", state.active_route().marker());
                    }
                    ReaderTickOutcome::BackgroundCacheAdvanced => {
                        if let Some(session) = state.reader.session.as_ref() {
                            info!("rustmix-wave=reader-background-cache indexed-percent={} pages={} complete={}", session.progress_percent(), session.page_offsets.len(), session.index_complete);
                        }
                    }
                    ReaderTickOutcome::Failed => {
                        warn!(
                            "rustmix-wave=reader-cache-stage status=failed route={}",
                            state.active_route().marker()
                        );
                    }
                    ReaderTickOutcome::None => {}
                }
                apply_wifi_transfer_ui_request(
                    &mut wifi_transfer_server,
                    &mut state,
                    &mut storage_browser,
                    _mounted_sd.is_some(),
                    voice_recording.is_some(),
                    voice_playback.is_some(),
                );
                log_reader_persistence_event(&mut state);
                if state.panel_awake
                    && (outcome == ReaderTickOutcome::LoadingStageChanged
                        || outcome == ReaderTickOutcome::FirstPageReady
                        || outcome == ReaderTickOutcome::Failed
                        || state.active_route() != previous_route)
                {
                    refresh_screen(
                        &mut panel,
                        &mut frame,
                        &mut state,
                        &mut panel_refresh,
                        RefreshRequest::Normal,
                    )?;
                    last_activity = Instant::now();
                }
                last_reader_tick = Instant::now();
            }

            if !sleep_mode.is_sleeping()
                && state.panel_awake
                && (state.active_route() == ScreenRoute::MotionEvents
                    || state.lua_game_needs_imu_events())
                && last_imu_event_sample.elapsed()
                    >= Duration::from_millis(IMU_EVENT_SAMPLE_INTERVAL_MS)
            {
                match board_services.read_imu_motion() {
                    Ok(reading) => {
                        let now_ms = imu_event_started_at.elapsed().as_millis() as u64;
                        let event = state.update_imu_event_sample(reading, now_ms);
                        if let Some(event) = event {
                            info!("rustmix-wave=imu-event type={} detail={} at-ms={} samples={} counts=tilt:{},shake:{},rotate:{},level:{} thresholds=tilt:{}mg,shake:{}mg,rotate:{}dps,level:{}mg,debounce:{}ms", event.kind.marker(), event.kind.detail_marker(), event.at_ms, state.imu_events.samples, state.imu_events.counters.tilt, state.imu_events.counters.shake, state.imu_events.counters.rotate, state.imu_events.counters.level, state.imu_events.thresholds.tilt_enter_mg, state.imu_events.thresholds.shake_delta_mg, state.imu_events.thresholds.rotate_dps, state.imu_events.thresholds.level_tolerance_mg, state.imu_events.thresholds.debounce_ms);
                        }
                        let game_motion_changed =
                            event.is_some_and(|event| state.apply_lua_game_motion_event(event));
                        if game_motion_changed {
                            log_lua_runtime_events(&mut state);
                        }
                        let diagnostic_refresh = state.active_route() == ScreenRoute::MotionEvents
                            && (event.is_some()
                                || last_imu_event_screen_refresh.elapsed()
                                    >= Duration::from_secs(IMU_EVENT_SCREEN_REFRESH_SECONDS));
                        if game_motion_changed || diagnostic_refresh {
                            refresh_screen(
                                &mut panel,
                                &mut frame,
                                &mut state,
                                &mut panel_refresh,
                                RefreshRequest::Normal,
                            )?;
                            last_imu_event_screen_refresh = Instant::now();
                        }
                    }
                    Err(error) => {
                        warn!("rustmix-wave=imu-event-sample status=unavailable error={error:#}")
                    }
                }
                last_imu_event_sample = Instant::now();
            }

            let live_refresh_seconds = match state.active_route() {
                ScreenRoute::Motion | ScreenRoute::MotionDetails => MOTION_LIVE_REFRESH_SECONDS,
                ScreenRoute::Network | ScreenRoute::NetworkDetails => NETWORK_LIVE_REFRESH_SECONDS,
                _ => SAMPLE_LIVE_REFRESH_SECONDS,
            };
            if state.panel_awake
                && state.active_route().uses_live_status()
                && last_status_refresh.elapsed() >= Duration::from_secs(live_refresh_seconds)
            {
                state.update_board_snapshot(board_services.read_snapshot(&mut service_delay));
                log_board_snapshot(state.board, state.regional);
                refresh_screen(
                    &mut panel,
                    &mut frame,
                    &mut state,
                    &mut panel_refresh,
                    RefreshRequest::Normal,
                )?;
                info!(
                    "rustmix-wave=sample-board-status-auto-refresh route={}",
                    state.active_route().marker()
                );
                last_status_refresh = Instant::now();
            }

            match back_button.poll(&mut button_delay)? {
                Some(BootButtonEvent::LongPress) => {
                    info!(
                        "rustmix-wave=boot-button event=long-press action=back hold-ms={BOOT_BACK_LONG_PRESS_MS}"
                    );
                    if sleep_mode.is_sleeping() {
                        info!(
                            "rustmix-wave=sleep-mode-input-suppressed event=boot-long-press-back"
                        );
                        FreeRtos::delay_ms(20);
                        continue;
                    }
                    let woke_from_sleep = !state.panel_awake;
                    if woke_from_sleep {
                        panel.initialize()?;
                        state.panel_awake = true;
                        panel_refresh.reset_after_external_global(PanelGlobalReason::AfterWake);
                        sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                    }
                    state.update_board_snapshot(board_services.read_snapshot(&mut service_delay));
                    log_board_snapshot(state.board, state.regional);
                    let previous_route = state.active_route();
                    if previous_route == ScreenRoute::Home {
                        info!("rustmix-wave=hierarchical-back outcome=ignored route=home");
                    } else {
                        state.back();
                        apply_voice_notes_ui_request(
                            &mut voice_recording,
                            &mut voice_playback,
                            &mut audio_runtime,
                            &mut state,
                            _mounted_sd.is_some(),
                        );
                        apply_wifi_transfer_ui_request(
                            &mut wifi_transfer_server,
                            &mut state,
                            &mut storage_browser,
                            _mounted_sd.is_some(),
                            voice_recording.is_some(),
                            voice_playback.is_some(),
                        );
                        log_lua_runtime_events(&mut state);
                        info!(
                            "rustmix-wave=hierarchical-back outcome=navigated from={} to={}",
                            previous_route.marker(),
                            state.active_route().marker()
                        );
                        info!(
                            "rustmix-wave=screen-route route={}",
                            state.active_route().marker()
                        );
                    }
                    if woke_from_sleep || state.active_route() != previous_route {
                        let request = if woke_from_sleep {
                            RefreshRequest::ForceGlobalAfterWake
                        } else {
                            RefreshRequest::Normal
                        };
                        refresh_screen(
                            &mut panel,
                            &mut frame,
                            &mut state,
                            &mut panel_refresh,
                            request,
                        )?;
                    }
                    last_activity = Instant::now();
                    last_status_refresh = Instant::now();
                }
                Some(BootButtonEvent::ShortPress) => {
                    info!(
                        "rustmix-wave=boot-button event=short-press action=contextual-navigation"
                    );
                    if sleep_mode.is_sleeping() {
                        info!("rustmix-wave=sleep-mode-input-suppressed event=boot-short-press-contextual-navigation");
                        FreeRtos::delay_ms(20);
                        continue;
                    }
                    let calendar_agenda_context = state.apply_calendar_boot_short_press();
                    let keyboard_context = if calendar_agenda_context {
                        false
                    } else {
                        state.apply_keyboard_boot_short_press()
                    };
                    let lua_game_context = if calendar_agenda_context || keyboard_context {
                        false
                    } else {
                        state.apply_lua_game_boot_short_press()
                    };
                    if calendar_agenda_context || keyboard_context || lua_game_context {
                        if calendar_agenda_context {
                            info!("rustmix-wave=calendar-agenda route=selected-day outcome=opened");
                        }
                        if keyboard_context {
                            if state.active_route() == ScreenRoute::CalendarEventEditor {
                                if let Some(editor) = state.calendar.editor.as_ref() {
                                    info!(
                                        "rustmix-wave=calendar-editor-keyboard-nav axis={} outcome=toggled",
                                        editor.navigation_mode_label()
                                    );
                                }
                            } else if state.active_route() == ScreenRoute::VoiceNoteDetails
                                && state.voice_notes.title_editing
                            {
                                info!(
                                    "rustmix-wave=voice-note-title-keyboard-nav axis={} outcome=toggled",
                                    state.voice_notes.title_editor_navigation_mode_label()
                                );
                            } else {
                                info!(
                                    "rustmix-wave=dictionary-keyboard-nav axis={} outcome=toggled",
                                    state.dictionary.navigation_mode_label()
                                );
                            }
                        }
                        let woke_from_sleep = !state.panel_awake;
                        if woke_from_sleep {
                            panel.initialize()?;
                            state.panel_awake = true;
                            panel_refresh.reset_after_external_global(PanelGlobalReason::AfterWake);
                            sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                        }
                        state.update_board_snapshot(
                            board_services.read_snapshot(&mut service_delay),
                        );
                        log_board_snapshot(state.board, state.regional);
                        log_lua_runtime_events(&mut state);
                        let request = if woke_from_sleep {
                            RefreshRequest::ForceGlobalAfterWake
                        } else {
                            RefreshRequest::Normal
                        };
                        refresh_screen(
                            &mut panel,
                            &mut frame,
                            &mut state,
                            &mut panel_refresh,
                            request,
                        )?;
                        last_activity = Instant::now();
                        last_status_refresh = Instant::now();
                    } else {
                        info!(
                            "rustmix-wave=boot-button event=short-press action=ignored route={}",
                            state.active_route().marker()
                        );
                    }
                }
                None => {}
            }

            if let Some(event) = buttons.poll(&mut button_delay)? {
                info!("rustmix-wave=button-event event={event:?}");
                if sleep_mode.is_sleeping() {
                    info!("rustmix-wave=sleep-mode-input-suppressed event={event:?}");
                    FreeRtos::delay_ms(20);
                    continue;
                }
                let woke_from_sleep = !state.panel_awake;
                if woke_from_sleep {
                    panel.initialize()?;
                    state.panel_awake = true;
                    panel_refresh.reset_after_external_global(PanelGlobalReason::AfterWake);
                    sync_panel_refresh_diagnostics(&mut state, &panel_refresh);
                }

                state.update_board_snapshot(board_services.read_snapshot(&mut service_delay));
                log_board_snapshot(state.board, state.regional);
                let previous_route = state.active_route();
                let previous_display = state.display;
                if previous_route == ScreenRoute::Files {
                    apply_storage_event(&mut storage_browser, &mut state, event);
                } else if previous_route == ScreenRoute::Alarms {
                    let local = state
                        .board
                        .rtc
                        .map_or_else(fallback_local_time, |rtc| state.regional.localize_rtc(rtc));
                    let outcome = apply_alarm_event(&mut alarm_engine, &mut state, event, local);
                    if matches!(
                        outcome,
                        AlarmUiOutcome::Saved | AlarmUiOutcome::Snoozed | AlarmUiOutcome::Dismissed
                    ) {
                        sync_alarm_hardware(&mut alarm_engine, &mut board_services, state.regional);
                        state.update_alarm_snapshot(alarm_engine.snapshot());
                        log_alarm_snapshot(&state.alarms);
                    }
                    if matches!(outcome, AlarmUiOutcome::Snoozed | AlarmUiOutcome::Dismissed) {
                        if let Some(runtime) = audio_runtime.as_mut() {
                            match runtime.stop_playback() {
                                Ok(()) => info!("rustmix-wave=audio-event outcome=alarm-chime-stop reason={outcome:?}"),
                                Err(error) => {
                                    warn!("rustmix-wave=audio-event outcome=alarm-chime-stop-failed error={error:#}");
                                    runtime.record_failure(format!("{error:#}"));
                                }
                            }
                            state.update_audio_snapshot(runtime.snapshot());
                            log_audio_snapshot(&state.audio);
                        }
                    }
                } else if previous_route == ScreenRoute::Audio {
                    if let Some(request) = state.apply_audio_button(event) {
                        apply_audio_request(&mut audio_runtime, &mut state, request);
                    }
                } else {
                    state.apply(event);
                    log_lua_runtime_events(&mut state);
                    if state.active_route() == ScreenRoute::Files {
                        storage_browser.refresh();
                        state.update_storage_snapshot(storage_browser.snapshot());
                        log_storage_snapshot(&state.storage);
                    }
                }
                // Consume Settings > Network transfer start/stop intents before
                // rendering the next frame.  This guarantees that the transfer
                // route shows READY plus its LAN URL and code on the same normal
                // partial refresh that follows the SELECT event.
                apply_calendar_ui_request(&mut state, _mounted_sd.is_some());
                apply_voice_notes_ui_request(
                    &mut voice_recording,
                    &mut voice_playback,
                    &mut audio_runtime,
                    &mut state,
                    _mounted_sd.is_some(),
                );
                apply_wifi_transfer_ui_request(
                    &mut wifi_transfer_server,
                    &mut state,
                    &mut storage_browser,
                    _mounted_sd.is_some(),
                    voice_recording.is_some(),
                    voice_playback.is_some(),
                );
                log_reader_persistence_event(&mut state);
                if state.display != previous_display {
                    match state.display.save_to_path(DISPLAY_CONFIG_PATH) {
                        Ok(()) => info!(
                            "rustmix-wave=display-config-write status=saved path={DISPLAY_CONFIG_PATH}"
                        ),
                        Err(error) => warn!(
                            "rustmix-wave=display-config-write status=failed path={DISPLAY_CONFIG_PATH} error={error:#}"
                        ),
                    }
                    info!(
                        "rustmix-wave=display-settings-updated font-family={} font-size={} persistence=sd-file path={DISPLAY_CONFIG_PATH}",
                        state.display.font_family.marker(),
                        state.display.font_size.marker()
                    );
                }
                if state.active_route() != previous_route {
                    info!(
                        "rustmix-wave=screen-route route={}",
                        state.active_route().marker()
                    );
                }
                let reader_clear_ghost = state.take_reader_clear_ghost_request();
                let power_key_clear_ghost = state.take_power_key_manual_refresh_request();
                let request = if woke_from_sleep {
                    RefreshRequest::ForceGlobalAfterWake
                } else if reader_clear_ghost || power_key_clear_ghost {
                    RefreshRequest::ForceGlobalManual
                } else {
                    RefreshRequest::Normal
                };
                refresh_screen(
                    &mut panel,
                    &mut frame,
                    &mut state,
                    &mut panel_refresh,
                    request,
                )?;
                last_activity = Instant::now();
                last_status_refresh = Instant::now();
            }

            FreeRtos::delay_ms(20);
        }
    }

    fn apply_calendar_ui_request(state: &mut AppState, mounted: bool) {
        let Some(request) = state.take_calendar_request() else {
            return;
        };
        if !mounted {
            state.calendar.fail("SD card unavailable");
            warn!(
                "rustmix-wave=calendar-personal-event-write status=rejected reason=sd-unavailable"
            );
            return;
        }
        let root = std::path::Path::new(CALENDAR_ROOT);
        let outcome = match request {
            CalendarUiRequest::CreatePersonal { date, title, detail } => {
                create_personal_event(root, date, &title, &detail).map(|()| {
                    info!("rustmix-wave=calendar-personal-event-write status=completed operation=create title={title}");
                    "Personal event created"
                })
            }
            CalendarUiRequest::UpdatePersonal {
                source_row,
                title,
                detail,
            } => update_personal_event(root, source_row, &title, &detail).map(|()| {
                info!("rustmix-wave=calendar-personal-event-write status=completed operation=edit source-row={source_row} title={title}");
                "Personal event updated"
            }),
            CalendarUiRequest::DeletePersonal { source_row } => {
                delete_personal_event(root, source_row).map(|()| {
                    info!("rustmix-wave=calendar-personal-event-write status=completed operation=delete source-row={source_row}");
                    "Personal event deleted"
                })
            }
        };
        match outcome {
            Ok(message) => {
                state.calendar.refresh_events();
                state.calendar.mark_persistence_completed(message);
                state.router.navigate_to(ScreenRoute::CalendarAgenda);
            }
            Err(error) => {
                state.calendar.fail(format!("{error:#}"));
                warn!("rustmix-wave=calendar-personal-event-write status=failed error={error:#}");
            }
        }
    }

    fn maintain_wifi_transfer_server(
        server: &mut Option<WifiTransferServer>,
        state: &mut AppState,
        storage_browser: &mut StorageBrowser,
        mounted: bool,
    ) {
        let stop_reason = server.as_ref().and_then(|active| {
            if state.network.wifi_state != WifiConnectionState::Connected {
                Some("wifi-loss")
            } else if active.is_expired() {
                Some("inactivity-timeout")
            } else {
                None
            }
        });
        if let Some(reason) = stop_reason {
            stop_wifi_transfer_server(server, state, storage_browser, mounted, reason);
        } else if let Some(active) = server.as_ref() {
            let snapshot = active.snapshot();
            if snapshot != state.wifi_transfer {
                state.update_wifi_transfer_snapshot(snapshot);
            }
        }
    }

    fn apply_wifi_transfer_ui_request(
        server: &mut Option<WifiTransferServer>,
        state: &mut AppState,
        storage_browser: &mut StorageBrowser,
        mounted: bool,
        voice_recording_active: bool,
        voice_playback_active: bool,
    ) {
        let Some(request) = state.take_wifi_transfer_request() else {
            return;
        };
        match request {
            WifiTransferUiRequest::Start => {
                info!(
                    "rustmix-wave=wifi-transfer-ui-request request=start dispatch=before-refresh"
                );
                if voice_recording_active {
                    state.update_wifi_transfer_snapshot(WifiTransferSnapshot::failed(
                        "Voice recording is active; stop recording before Wi-Fi transfer",
                    ));
                    warn!("rustmix-wave=wifi-transfer-server status=rejected reason=voice-recording-active");
                    return;
                }
                if voice_playback_active {
                    state.update_wifi_transfer_snapshot(WifiTransferSnapshot::failed(
                        "Voice-note playback is active; stop playback before Wi-Fi transfer",
                    ));
                    warn!("rustmix-wave=wifi-transfer-server status=rejected reason=voice-note-playback-active");
                    return;
                }
                if server.is_some() {
                    return;
                }
                state.update_wifi_transfer_snapshot(WifiTransferSnapshot::starting());
                let Some(ipv4) = state.network.ipv4_address.as_deref() else {
                    state.update_wifi_transfer_snapshot(WifiTransferSnapshot::failed(
                        "Connect Wi-Fi before starting transfer",
                    ));
                    warn!("rustmix-wave=wifi-transfer-server status=start-rejected reason=wifi-not-connected");
                    return;
                };
                let code = format!("{:06}", unsafe { sys::esp_random() } % 1_000_000);
                info!("rustmix-wave=wifi-transfer-server status=starting ipv4={ipv4} port=80 root={WIFI_TRANSFER_ROOT} stack-bytes={WIFI_TRANSFER_SERVER_STACK_BYTES}");
                log_runtime_memory("before-wifi-transfer-start");
                match WifiTransferServer::start(ipv4, code) {
                    Ok(active) => {
                        state.update_wifi_transfer_snapshot(active.snapshot());
                        *server = Some(active);
                        log_runtime_memory("after-wifi-transfer-start");
                    }
                    Err(error) => {
                        warn!(
                            "rustmix-wave=wifi-transfer-server status=start-failed error={error:#}"
                        );
                        state.update_wifi_transfer_snapshot(WifiTransferSnapshot::failed(format!(
                            "{error:#}"
                        )));
                    }
                }
            }
            WifiTransferUiRequest::Stop => {
                info!("rustmix-wave=wifi-transfer-ui-request request=stop dispatch=before-refresh");
                stop_wifi_transfer_server(
                    server,
                    state,
                    storage_browser,
                    mounted,
                    "settings-toggle",
                );
            }
        }
    }

    fn stop_wifi_transfer_server(
        server: &mut Option<WifiTransferServer>,
        state: &mut AppState,
        storage_browser: &mut StorageBrowser,
        mounted: bool,
        reason: &'static str,
    ) {
        if server.take().is_some() {
            info!("rustmix-wave=wifi-transfer-server status=stopped reason={reason}");
            log_runtime_memory("after-wifi-transfer-stop");
        }
        state.update_wifi_transfer_snapshot(WifiTransferSnapshot::default());
        state.refresh_lua_app_catalog(mounted);
        state.reader.refresh_library();
        state.calendar.refresh_events();
        storage_browser.refresh();
        state.update_storage_snapshot(storage_browser.snapshot());
    }

    #[derive(Clone, Copy, Debug)]
    struct WeatherFetchAttempt {
        cause: &'static str,
        retry_attempt: usize,
    }

    impl WeatherFetchAttempt {
        const fn initial(cause: &'static str) -> Self {
            Self {
                cause,
                retry_attempt: 0,
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct PendingWeatherRetry {
        due: Instant,
        attempt: WeatherFetchAttempt,
    }

    #[derive(Clone, Copy, Debug, Default)]
    struct WeatherRetryState {
        pending: Option<PendingWeatherRetry>,
    }

    impl WeatherRetryState {
        fn clear(&mut self) {
            self.pending = None;
        }

        #[must_use]
        const fn is_pending(&self) -> bool {
            self.pending.is_some()
        }

        fn take_due(&mut self) -> Option<WeatherFetchAttempt> {
            if self
                .pending
                .as_ref()
                .is_some_and(|pending| Instant::now() >= pending.due)
            {
                self.pending.take().map(|pending| pending.attempt)
            } else {
                None
            }
        }

        fn schedule_next(&mut self, failed_attempt: WeatherFetchAttempt) -> Option<(usize, u64)> {
            let retry_attempt = failed_attempt.retry_attempt.saturating_add(1);
            let delay_seconds = *WEATHER_RETRY_DELAYS_SECONDS.get(retry_attempt - 1)?;
            self.pending = Some(PendingWeatherRetry {
                due: Instant::now() + Duration::from_secs(delay_seconds),
                attempt: WeatherFetchAttempt {
                    cause: failed_attempt.cause,
                    retry_attempt,
                },
            });
            Some((retry_attempt, delay_seconds))
        }
    }

    fn run_weather_fetch_attempt(
        config: &WeatherConfig,
        attempt: WeatherFetchAttempt,
        retry: &mut WeatherRetryState,
        state: &mut AppState,
    ) {
        let attempt_label = if attempt.retry_attempt == 0 {
            "initial".into()
        } else {
            format!("{}/{}", attempt.retry_attempt, WEATHER_RETRY_LIMIT)
        };
        info!(
            "rustmix-wave=weather-fetch status=starting cause={} attempt={} provider={} location={}",
            attempt.cause, attempt_label, config.provider, config.location
        );
        state.weather.mark_fetching();
        match fetch_open_meteo_on_worker(config) {
            Ok(data) => {
                retry.clear();
                state.weather.record_success(data);
                log_weather_snapshot(&state.weather);
                info!(
                    "rustmix-wave=weather-fetch status=completed cause={} attempt={} forecast-days={}",
                    attempt.cause,
                    attempt_label,
                    state.weather.forecast.len()
                );
            }
            Err(error) => {
                handle_weather_fetch_failure(attempt, error, retry, state);
                log_weather_snapshot(&state.weather);
            }
        }
    }

    fn handle_weather_fetch_failure(
        attempt: WeatherFetchAttempt,
        error: WeatherFetchError,
        retry: &mut WeatherRetryState,
        state: &mut AppState,
    ) {
        let message = error.to_string();
        if error.is_retryable() {
            if let Some((retry_attempt, delay_seconds)) = retry.schedule_next(attempt) {
                state.weather.mark_retrying(message.clone());
                warn!(
                    "rustmix-wave=weather-fetch-retry-scheduled cause={} attempt={}/{} delay-seconds={} classification={} error={}",
                    attempt.cause,
                    retry_attempt,
                    WEATHER_RETRY_LIMIT,
                    delay_seconds,
                    error.category(),
                    message
                );
                if state.weather.current.is_some() {
                    info!(
                        "rustmix-wave=weather-fetch outcome=stale-cache-retained state=retrying last-success={} error={}",
                        state.weather.last_success_label(),
                        message
                    );
                }
                return;
            }
        }

        retry.clear();
        state.weather.record_failure(message.clone());
        warn!(
            "rustmix-wave=weather-fetch status=failed cause={} retryable={} retries-exhausted={} classification={} error={}",
            attempt.cause,
            error.is_retryable(),
            error.is_retryable(),
            error.category(),
            message
        );
        if state.weather.current.is_some() {
            info!(
                "rustmix-wave=weather-fetch outcome=stale-cache-retained state=stale last-success={} error={}",
                state.weather.last_success_label(),
                message
            );
        }
    }

    fn suspend_network_for_sleep(
        runtime: &mut NetworkRuntime,
        state: &mut AppState,
        sleep_network: &mut SleepNetworkState,
        last_network_fingerprint: &mut NetworkLogFingerprint,
        last_network_log: &mut Instant,
    ) -> bool {
        info!("rustmix-wave=sleep-network-suspend status=starting");
        match runtime.suspend() {
            Ok(()) => {
                let _ = sleep_network.suspend();
                state.update_network_snapshot(runtime.snapshot());
                *last_network_fingerprint = state.network.log_fingerprint();
                *last_network_log = Instant::now();
                info!("rustmix-wave=sntp-suspend status=stopped");
                info!("rustmix-wave=wifi-suspend status=disconnected");
                info!("rustmix-wave=wifi-suspend status=stopped");
                info!("rustmix-wave=weather-suspend status=paused");
                true
            }
            Err(error) => {
                warn!("rustmix-wave=sleep-network-suspend status=failed error={error:#}");
                false
            }
        }
    }

    fn resume_network_after_sleep(
        runtime: &mut NetworkRuntime,
        config: Option<&NetworkConfig>,
        state: &mut AppState,
        sleep_network: &mut SleepNetworkState,
        last_network_fingerprint: &mut NetworkLogFingerprint,
        last_network_log: &mut Instant,
        last_weather_attempt: &mut Option<Instant>,
        weather_retry: &mut WeatherRetryState,
    ) {
        info!("rustmix-wave=sleep-network-resume status=starting");
        if let Some(config) = config {
            info!(
                "rustmix-wave=wifi-resume status=starting ssid={}",
                config.ssid
            );
            match runtime.resume(config) {
                Ok(()) => {
                    info!(
                        "rustmix-wave=wifi-resume status=connected ssid={}",
                        config.ssid
                    );
                    info!("rustmix-wave=sntp-resume status=started");
                    info!("rustmix-wave=weather-resume status=pending-network-ready");
                }
                Err(error) => {
                    warn!(
                        "rustmix-wave=wifi-resume status=failed ssid={} error={error:#}",
                        config.ssid
                    );
                    runtime.record_resume_failure(format!("{error:#}"));
                }
            }
        } else {
            runtime.record_configuration_missing();
            info!("rustmix-wave=wifi-resume status=skipped reason=configuration-missing");
        }
        let _ = sleep_network.resume();
        state.update_network_snapshot(runtime.snapshot());
        log_network_snapshot(&state.network);
        *last_network_fingerprint = state.network.log_fingerprint();
        *last_network_log = Instant::now();
        *last_weather_attempt = None;
        weather_retry.clear();
    }

    fn apply_alarm_event(
        engine: &mut AlarmEngine,
        state: &mut AppState,
        event: ButtonEvent,
        now_local: RtcDateTime,
    ) -> AlarmUiOutcome {
        if event == ButtonEvent::Select {
            state.note_select_press();
        }
        let outcome = engine.apply_button(event, now_local);
        if outcome == AlarmUiOutcome::ReturnHome {
            state.router.back();
        }
        state.update_alarm_snapshot(engine.snapshot());
        info!(
            "rustmix-wave=alarm-ui-event outcome={outcome:?} active={} schedules={} selected={} next={} hardware-programmed={}",
            state.alarms.active.is_some(),
            state.alarms.alarms.len(),
            state.alarms.selected,
            state.alarms.next_label(),
            state.alarms.hardware_programmed
        );
        outcome
    }

    fn apply_audio_request<'d, I2C>(
        runtime: &mut Option<AudioRuntime<'d, I2C>>,
        state: &mut AppState,
        request: AudioUiRequest,
    ) where
        I2C: embedded_hal::i2c::I2c,
        I2C::Error: core::fmt::Debug,
    {
        let Some(runtime) = runtime.as_mut() else {
            warn!("rustmix-wave=audio-event outcome=unavailable request={request:?}");
            return;
        };
        match runtime.apply_request(request) {
            Ok(outcome) => info!("rustmix-wave=audio-event outcome={outcome}"),
            Err(error) => {
                warn!("rustmix-wave=audio-event outcome=request-failed request={request:?} error={error:#}");
                runtime.record_failure(format!("{error:#}"));
            }
        }
        state.update_audio_snapshot(runtime.snapshot());
        log_audio_snapshot(&state.audio);
    }

    fn sd_available_bytes(path: &str) -> Option<u64> {
        let path = CString::new(path).ok()?;
        let mut total_bytes = 0_u64;
        let mut free_bytes = 0_u64;
        if unsafe { sys::esp_vfs_fat_info(path.as_ptr(), &mut total_bytes, &mut free_bytes) }
            != sys::ESP_OK
        {
            return None;
        }
        Some(free_bytes)
    }

    fn refresh_voice_note_storage_available(state: &mut AppState, mounted: bool) {
        let available = mounted
            .then(|| sd_available_bytes(SD_MOUNT_POINT))
            .flatten();
        state.voice_notes.set_available_storage_bytes(available);
    }

    fn stop_voice_note_playback<'d, I2C>(
        session: &mut Option<VoicePlaybackSession>,
        audio_runtime: &mut Option<AudioRuntime<'d, I2C>>,
        state: &mut AppState,
        reason: &str,
    ) where
        I2C: embedded_hal::i2c::I2c,
        I2C::Error: core::fmt::Debug,
    {
        let Some(active) = session.take() else {
            return;
        };
        let file_name = active.file_name().to_string();
        if let Some(runtime) = audio_runtime.as_mut() {
            if let Err(error) = runtime.finish_voice_note_playback() {
                warn!("rustmix-wave=voice-note-playback status=stop-failed file={file_name} reason={reason} error={error:#}");
                runtime.record_failure(format!("{error:#}"));
            }
            state.update_audio_snapshot(runtime.snapshot());
            log_audio_snapshot(&state.audio);
        }
        state.voice_notes.stop_playback();
        info!("rustmix-wave=voice-note-playback status=stopped file={file_name} reason={reason}");
    }

    fn apply_voice_notes_ui_request<'d, I2C>(
        session: &mut Option<VoiceRecordingSession>,
        playback: &mut Option<VoicePlaybackSession>,
        audio_runtime: &mut Option<AudioRuntime<'d, I2C>>,
        state: &mut AppState,
        mounted: bool,
    ) where
        I2C: embedded_hal::i2c::I2c,
        I2C::Error: core::fmt::Debug,
    {
        let Some(request) = state.take_voice_notes_request() else {
            return;
        };
        match request {
            VoiceNotesUiRequest::StartRecording => {
                if !mounted {
                    state.voice_notes.fail("SD card unavailable");
                    warn!("rustmix-wave=voice-record status=rejected reason=sd-unavailable");
                    return;
                }
                if state.wifi_transfer.is_active() {
                    state
                        .voice_notes
                        .fail("Stop Wi-Fi Transfer before recording");
                    warn!("rustmix-wave=voice-record status=rejected reason=wifi-transfer-active");
                    return;
                }
                if state.alarms.active.is_some() {
                    state.voice_notes.fail("Alarm active");
                    warn!("rustmix-wave=voice-record status=rejected reason=active-alarm");
                    return;
                }
                if playback.is_some() {
                    stop_voice_note_playback(playback, audio_runtime, state, "recording-start");
                }
                let Some(runtime) = audio_runtime.as_mut() else {
                    state.voice_notes.fail("Microphone unavailable");
                    warn!("rustmix-wave=voice-record status=rejected reason=audio-unavailable");
                    return;
                };
                if session.is_some() {
                    return;
                }
                let recorded_at = state
                    .board
                    .rtc
                    .map(|rtc| state.regional.localize_rtc(rtc).date_time())
                    .unwrap_or_else(|| VOICE_UNKNOWN_RECORDED_AT.into());
                log_runtime_memory("before-voice-record");
                match VoiceRecordingSession::start_with_recorded_at(
                    std::path::Path::new(VOICE_NOTES_ROOT),
                    recorded_at.clone(),
                ) {
                    Ok(created) => {
                        if let Err(error) = runtime.begin_voice_recording() {
                            let _ = created.cancel();
                            state.voice_notes.fail(format!("{error:#}"));
                            warn!("rustmix-wave=voice-record status=failed stage=audio-start error={error:#}");
                            return;
                        }
                        let file_name = created.file_name().to_string();
                        state
                            .voice_notes
                            .begin_recording(file_name.clone(), recorded_at.clone());
                        *session = Some(created);
                        log_runtime_memory("after-voice-record-start");
                        info!("rustmix-wave=voice-record status=starting file={} recorded-at={} sample-rate=16000 bits=16 channels=1 chunk-bytes={} capture=cooperative-bounded-i2s-rx mic-gain={}", file_name, recorded_at, VOICE_PCM_MONO_CHUNK_BYTES, state.voice_notes.mic_gain.marker());
                    }
                    Err(error) => {
                        state.voice_notes.fail(format!("{error:#}"));
                        warn!("rustmix-wave=voice-record status=failed stage=storage-start error={error:#}");
                    }
                }
            }
            VoiceNotesUiRequest::StopRecording => {
                let Some(active) = session.take() else {
                    return;
                };
                match active.finalize() {
                    Ok(entry) => {
                        if let Some(runtime) = audio_runtime.as_mut() {
                            let _ = runtime.finish_voice_recording();
                            state.update_audio_snapshot(runtime.snapshot());
                        }
                        info!("rustmix-wave=voice-record status=completed file={} recorded-at={} duration-seconds={} pcm-bytes={} wav-bytes={}", entry.file_name, entry.recorded_at, entry.duration_seconds, entry.pcm_bytes, entry.wav_bytes);
                        state.voice_notes.complete_recording(entry);
                        state.refresh_voice_notes_catalog();
                        refresh_voice_note_storage_available(state, mounted);
                        log_runtime_memory("after-voice-record-stop");
                    }
                    Err(error) => {
                        if let Some(runtime) = audio_runtime.as_mut() {
                            let _ = runtime.finish_voice_recording();
                            state.update_audio_snapshot(runtime.snapshot());
                        }
                        state.voice_notes.fail(format!("{error:#}"));
                        warn!("rustmix-wave=voice-record status=failed stage=finalize error={error:#}");
                        log_runtime_memory("after-voice-record-stop");
                    }
                }
            }
            VoiceNotesUiRequest::PauseRecording => {
                if session.is_some() {
                    state.voice_notes.pause_recording();
                    info!("rustmix-wave=voice-record status=paused");
                }
            }
            VoiceNotesUiRequest::ResumeRecording => {
                if session.is_some() {
                    state.voice_notes.resume_recording();
                    info!("rustmix-wave=voice-record status=resumed");
                }
            }
            VoiceNotesUiRequest::CancelRecording => {
                if let Some(active) = session.take() {
                    let _ = active.cancel();
                }
                if let Some(runtime) = audio_runtime.as_mut() {
                    let _ = runtime.finish_voice_recording();
                    state.update_audio_snapshot(runtime.snapshot());
                }
                state.voice_notes.cancel_recording();
                refresh_voice_note_storage_available(state, mounted);
                info!("rustmix-wave=voice-record status=cancelled");
            }
            VoiceNotesUiRequest::StartPlayback => {
                if !mounted {
                    state.voice_notes.fail("SD card unavailable");
                    warn!("rustmix-wave=voice-note-playback status=rejected reason=sd-unavailable");
                    return;
                }
                if session.is_some() {
                    state.voice_notes.fail("Stop recording before playback");
                    warn!("rustmix-wave=voice-note-playback status=rejected reason=voice-recording-active");
                    return;
                }
                if state.wifi_transfer.is_active() {
                    state
                        .voice_notes
                        .fail("Stop Wi-Fi Transfer before playback");
                    warn!("rustmix-wave=voice-note-playback status=rejected reason=wifi-transfer-active");
                    return;
                }
                if state.alarms.active.is_some() {
                    state.voice_notes.fail("Alarm active");
                    warn!("rustmix-wave=voice-note-playback status=rejected reason=active-alarm");
                    return;
                }
                let Some(file_name) = state
                    .voice_notes
                    .selected_note()
                    .map(|note| note.file_name.clone())
                else {
                    state.voice_notes.fail("No voice note selected");
                    warn!("rustmix-wave=voice-note-playback status=rejected reason=no-selection");
                    return;
                };
                if audio_runtime.is_none() {
                    state.voice_notes.fail("Speaker unavailable");
                    warn!(
                        "rustmix-wave=voice-note-playback status=rejected reason=audio-unavailable"
                    );
                    return;
                }
                if playback.is_some() {
                    stop_voice_note_playback(playback, audio_runtime, state, "replace-selection");
                }
                match VoicePlaybackSession::open(std::path::Path::new(VOICE_NOTES_ROOT), &file_name)
                {
                    Ok(created) => {
                        let total_pcm_bytes = created.total_pcm_bytes();
                        let runtime = audio_runtime
                            .as_mut()
                            .expect("audio runtime checked before playback start");
                        if let Err(error) = runtime.begin_voice_note_playback() {
                            runtime.record_failure(format!(
                                "Voice-note playback start failed: {error:#}"
                            ));
                            state.update_audio_snapshot(runtime.snapshot());
                            state.voice_notes.fail(format!("{error:#}"));
                            warn!("rustmix-wave=voice-note-playback status=failed stage=audio-start file={file_name} error={error:#}");
                            return;
                        }
                        state
                            .voice_notes
                            .begin_playback(file_name.clone(), total_pcm_bytes);
                        *playback = Some(created);
                        state.update_audio_snapshot(runtime.snapshot());
                        log_audio_snapshot(&state.audio);
                        info!("rustmix-wave=voice-note-playback status=starting file={file_name} pcm-bytes={total_pcm_bytes} sample-rate=16000 bits=16 source-channels=1 output-channels=2 chunk-bytes={VOICE_PCM_MONO_CHUNK_BYTES} volume={}", state.audio.volume_percent);
                    }
                    Err(error) => {
                        state.voice_notes.fail(format!("{error:#}"));
                        warn!("rustmix-wave=voice-note-playback status=failed stage=storage-open file={file_name} error={error:#}");
                    }
                }
            }
            VoiceNotesUiRequest::StopPlayback => {
                stop_voice_note_playback(playback, audio_runtime, state, "ui-stop");
            }
            VoiceNotesUiRequest::PersistMicGain(mic_gain) => {
                let preferences = VoiceNotesPreferences { mic_gain };
                match save_voice_notes_preferences(std::path::Path::new(VOICE_NOTES_ROOT), preferences) {
                    Ok(()) => info!("rustmix-wave=voice-note-settings-write status=completed mic-gain={} path={VOICE_NOTES_ROOT}/SETTINGS.TXT", mic_gain.marker()),
                    Err(error) => {
                        state.voice_notes.fail(format!("{error:#}"));
                        warn!("rustmix-wave=voice-note-settings-write status=failed mic-gain={} error={error:#}", mic_gain.marker());
                    }
                }
            }
            VoiceNotesUiRequest::SaveEditedTitle { file_name, title } => {
                match save_voice_note_title(
                    std::path::Path::new(VOICE_NOTES_ROOT),
                    &file_name,
                    &title,
                ) {
                    Ok(()) => {
                        state.refresh_voice_notes_catalog();
                        info!("rustmix-wave=voice-note-title-write status=completed file={file_name} title={title}");
                    }
                    Err(error) => {
                        state.voice_notes.fail(format!("{error:#}"));
                        warn!("rustmix-wave=voice-note-title-write status=failed file={file_name} error={error:#}");
                    }
                }
            }
            VoiceNotesUiRequest::ExportSelected => {
                if !mounted {
                    state.voice_notes.fail("SD card unavailable");
                    warn!("rustmix-wave=voice-note-export status=rejected reason=sd-unavailable");
                    return;
                }
                if session.is_some() {
                    state.voice_notes.fail("Stop recording before export");
                    warn!("rustmix-wave=voice-note-export status=rejected reason=voice-recording-active");
                    return;
                }
                if playback.is_some() {
                    stop_voice_note_playback(playback, audio_runtime, state, "export-note");
                }
                let Some(file_name) = state
                    .voice_notes
                    .selected_note()
                    .map(|note| note.file_name.clone())
                else {
                    state.voice_notes.fail("No voice note selected");
                    return;
                };
                state.voice_notes.mark_export_requested(file_name.clone());
                state.request_wifi_transfer_start();
                info!("rustmix-wave=voice-note-export status=requested file={file_name} portal-path=VOICE/{file_name}");
            }
            VoiceNotesUiRequest::DeleteSelected => {
                if playback.is_some() {
                    stop_voice_note_playback(playback, audio_runtime, state, "delete-note");
                }
                let selected = state
                    .voice_notes
                    .selected_note()
                    .map(|note| note.file_name.clone());
                if let Some(file_name) = selected {
                    match delete_voice_note(std::path::Path::new(VOICE_NOTES_ROOT), &file_name) {
                        Ok(()) => {
                            state.voice_notes.remove_selected_note();
                            state.refresh_voice_notes_catalog();
                            refresh_voice_note_storage_available(state, mounted);
                            state.router.navigate_to(ScreenRoute::VoiceNotes);
                            info!(
                                "rustmix-wave=voice-note-delete status=completed file={file_name} confirmation=accepted"
                            );
                        }
                        Err(error) => {
                            state.voice_notes.fail(format!("{error:#}"));
                            warn!("rustmix-wave=voice-note-delete status=failed file={file_name} error={error:#}");
                        }
                    }
                }
            }
            VoiceNotesUiRequest::RefreshCatalog => {
                state.refresh_voice_notes_catalog();
                refresh_voice_note_storage_available(state, mounted);
            }
        }
    }

    fn sync_alarm_hardware<I2C>(
        engine: &mut AlarmEngine,
        board_services: &mut BoardServices<I2C>,
        regional: RegionalPreferences,
    ) where
        I2C: embedded_hal::i2c::I2c,
        I2C::Error: core::fmt::Debug,
    {
        if let Some(next) = engine.next_occurrence() {
            let stored = regional.local_to_rtc(next.local);
            match board_services.program_rtc_alarm(stored) {
                Ok(()) => {
                    engine.set_hardware_programmed(true);
                    info!(
                        "rustmix-wave=rtc-alarm-program status=armed local={} stored={} snooze={}",
                        next.local.date_time(),
                        stored.date_time(),
                        next.snooze
                    );
                }
                Err(error) => {
                    engine.set_hardware_programmed(false);
                    warn!("rustmix-wave=rtc-alarm-program status=failed error={error:#}");
                }
            }
        } else {
            match board_services.disable_rtc_alarm() {
                Ok(()) => {
                    engine.set_hardware_programmed(false);
                    info!("rustmix-wave=rtc-alarm-program status=idle");
                }
                Err(error) => warn!("rustmix-wave=rtc-alarm-disable status=failed error={error:#}"),
            }
        }
    }

    fn fallback_local_time() -> RtcDateTime {
        RtcDateTime {
            year: 2000,
            month: 1,
            day: 1,
            weekday: 6,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }

    fn apply_storage_event(browser: &mut StorageBrowser, state: &mut AppState, event: ButtonEvent) {
        if event == ButtonEvent::Select {
            state.note_select_press();
        }
        let outcome = browser.apply_button(event);
        if outcome == StorageUiOutcome::ReturnHome {
            state.router.back();
        }
        state.update_storage_snapshot(browser.snapshot());
        info!(
            "rustmix-wave=storage-browser-event outcome={outcome:?} path={} entries={} retained-entries={} raw-entries={} selected={} preview={}",
            state.storage.current_path,
            state.storage.entries.len(),
            state.storage.scan.retained_entries,
            state.storage.scan.raw_entries,
            state.storage.selected,
            state.storage.preview.is_some()
        );
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum RefreshRequest {
        Normal,
        ForceGlobalAfterWake,
        ForceGlobalManual,
        #[allow(dead_code)]
        ForceGlobalSafetyFallback,
    }

    fn refresh_screen<SPI, DC, RST, CS, BUSY, DELAY, POWER>(
        panel: &mut Epaper397<SPI, DC, RST, CS, BUSY, DELAY, POWER>,
        frame: &mut FrameBuffer,
        state: &mut AppState,
        coordinator: &mut PanelRefreshCoordinator,
        request: RefreshRequest,
    ) -> Result<()>
    where
        SPI: embedded_hal::spi::SpiBus<u8>,
        SPI::Error: core::fmt::Debug,
        DC: embedded_hal::digital::OutputPin,
        DC::Error: core::fmt::Debug,
        RST: embedded_hal::digital::OutputPin,
        RST::Error: core::fmt::Debug,
        CS: embedded_hal::digital::OutputPin,
        CS::Error: core::fmt::Debug,
        BUSY: embedded_hal::digital::InputPin,
        BUSY::Error: core::fmt::Debug,
        DELAY: DelayNs,
        POWER: waveshare_epd397_rust_app::power::PanelPower,
    {
        let coordinator_request = match request {
            RefreshRequest::Normal => PanelRefreshRequest::Normal,
            RefreshRequest::ForceGlobalAfterWake => PanelRefreshRequest::AfterWake,
            RefreshRequest::ForceGlobalManual => PanelRefreshRequest::ManualGhostCleanup,
            RefreshRequest::ForceGlobalSafetyFallback => PanelRefreshRequest::SafetyFallback,
        };
        let plan = coordinator.plan(coordinator_request);
        sync_panel_refresh_diagnostics(state, coordinator);
        render_current_screen(frame, state)?;

        match plan {
            PanelRefreshPlan::GlobalBase { reason } => {
                panel.show_base(frame.as_bytes())?;
                info!(
                    "rustmix-wave=panel-refresh plan=global-base reason={} transport=global-base",
                    reason.marker()
                );
                match reason {
                    PanelGlobalReason::AfterWake => info!("rustmix-wave=wake-global-refresh"),
                    PanelGlobalReason::ManualGhostCleanup => {
                        info!("rustmix-wave=reader-clear-ghosting refresh=global-base");
                        info!("rustmix-wave=power-key-clear-ghosting refresh=global-base")
                    }
                    PanelGlobalReason::PeriodicCleanup => {
                        info!("rustmix-wave=global-refresh-after-partials")
                    }
                    PanelGlobalReason::SafetyFallback => {
                        warn!("rustmix-wave=panel-refresh safety-fallback refresh=global-base")
                    }
                    PanelGlobalReason::InitialBoot | PanelGlobalReason::SleepImage => {}
                }
            }
            PanelRefreshPlan::PartialFullscreen { partial_count } => {
                panel.show_partial_fullscreen(frame.as_bytes())?;
                info!(
                    "rustmix-wave=panel-refresh plan=partial-fullscreen reason=normal partial-count={partial_count} partial-limit={PANEL_PARTIAL_REFRESH_LIMIT} transport=existing-fullscreen-partial"
                );
            }
        }
        Ok(())
    }

    fn sync_panel_refresh_diagnostics(state: &mut AppState, coordinator: &PanelRefreshCoordinator) {
        state.partial_refreshes = coordinator.partial_count();
    }

    fn log_lua_runtime_events(state: &mut AppState) {
        for line in state.take_lua_runtime_diagnostics() {
            info!("{line}");
        }
    }

    fn log_reader_persistence_event(state: &mut AppState) {
        if let Some(event) = state.reader.take_persistence_event() {
            info!("rustmix-wave=reader-persistence {event}");
        }
    }

    fn log_sleep_image_selection(selection: &SleepImageSelection) {
        let error = selection.scan_error.as_deref().unwrap_or("none");
        if selection.fallback {
            warn!(
                "rustmix-wave=sleep-image-fallback source=built-in path={SLEEP_IMAGE_DIRECTORY} raw={} candidates={} metadata-fallbacks={} ignored={} valid={} rejected={} error={}",
                selection.raw_entries,
                selection.candidate_entries,
                selection.metadata_fallbacks,
                selection.ignored_entries,
                selection.valid_count,
                selection.rejected_count,
                error
            );
        } else {
            info!(
                "rustmix-wave=sleep-image-scan status=ready path={SLEEP_IMAGE_DIRECTORY} raw={} candidates={} metadata-fallbacks={} ignored={} valid={} rejected={} error={}",
                selection.raw_entries,
                selection.candidate_entries,
                selection.metadata_fallbacks,
                selection.ignored_entries,
                selection.valid_count,
                selection.rejected_count,
                error
            );
            info!(
                "rustmix-wave=sleep-image-selected file={} width=800 height=480 bpp=1 payload-bytes=48000",
                selection.file_name
            );
            if let Some(choice) = selection.choice {
                info!(
                    "rustmix-wave=sleep-image-choice mode=hardware-random candidates={} random-word=0x{:08X} previous-index={} selected-index={} anti-repeat={}",
                    selection.valid_count,
                    choice.random_word,
                    choice
                        .previous_index
                        .map_or_else(|| "none".into(), |index| index.to_string()),
                    choice.selected_index,
                    choice.anti_repeat
                );
            }
        }
    }

    fn log_board_snapshot(snapshot: BoardSnapshot, regional: RegionalPreferences) {
        let imu = snapshot.imu.map_or_else(
            || "unavailable".into(),
            |reading| {
                format!(
                    "motion={}mg axis={} acc=[{}] gyro=[{}]",
                    reading.motion_magnitude_mg,
                    reading.dominant_axis.label(),
                    reading.acceleration_mg_tenths.compact_label(),
                    reading.gyroscope_dps_tenths.compact_label()
                )
            },
        );
        info!(
            "rustmix-wave=sample-board-snapshot time={} timezone={} battery={} temperature={} humidity={} imu={}",
            snapshot.time_label(regional),
            regional.timezone_label_for_rtc(snapshot.rtc),
            snapshot.battery_label(),
            snapshot.temperature_label(regional.temperature_unit),
            snapshot.humidity_label(),
            imu
        );
    }

    fn log_storage_snapshot(snapshot: &StorageSnapshot) {
        info!(
            "rustmix-wave=storage-browser-snapshot mounted={} path={} entries={} retained-entries={} raw-entries={} metadata-fallbacks={} ignored-special={} selected={} preview={} error={}",
            snapshot.mounted,
            snapshot.current_path,
            snapshot.entries.len(),
            snapshot.scan.retained_entries,
            snapshot.scan.raw_entries,
            snapshot.scan.metadata_fallbacks,
            snapshot.scan.ignored_special,
            snapshot.selected,
            snapshot.preview.is_some(),
            snapshot.error.as_deref().unwrap_or("none")
        );
    }

    fn log_network_snapshot(snapshot: &NetworkSnapshot) {
        info!(
            "rustmix-wave=network-snapshot wifi={} ntp={} ssid={} ipv4={} rssi={} timezone={} ntp-server={} last-sync={} error={}",
            snapshot.wifi_state.label(),
            snapshot.ntp_state.label(),
            snapshot.ssid_label(),
            snapshot.ipv4_label(),
            snapshot.rssi_label(),
            snapshot.timezone_name,
            snapshot.ntp_server,
            snapshot.last_sync_label(),
            snapshot.error.as_deref().unwrap_or("none")
        );
    }

    fn log_audio_snapshot(snapshot: &AudioSnapshot) {
        info!(
            "rustmix-wave=audio-snapshot available={} codec-address={} codec-ready={} i2s-ready={} amp={} mute={} volume={} state={} error={}",
            snapshot.available,
            snapshot.codec_address_label(),
            snapshot.codec_ready,
            snapshot.i2s_ready,
            snapshot.amplifier_enabled,
            snapshot.muted,
            snapshot.volume_percent,
            snapshot.playback_state.label(),
            snapshot.error.as_deref().unwrap_or("none")
        );
    }

    fn log_alarm_snapshot(snapshot: &AlarmSnapshot) {
        info!(
            "rustmix-wave=alarm-snapshot schedules={} active={} selected={} next={} snooze-minutes={} hardware-programmed={} error={}",
            snapshot.alarms.len(),
            snapshot.active.as_ref().map_or("none", |active| active.name.as_str()),
            snapshot.selected,
            snapshot.next_label(),
            snapshot.snooze_minutes,
            snapshot.hardware_programmed,
            snapshot.error.as_deref().unwrap_or("none")
        );
    }

    fn log_weather_snapshot(snapshot: &WeatherSnapshot) {
        info!(
            "rustmix-wave=weather-snapshot state={} provider={} location={} timezone={} current={} forecast-days={} last-success={} error={}",
            snapshot.state.label(),
            snapshot.provider,
            snapshot.location,
            snapshot.provider_timezone,
            snapshot.current_summary(),
            snapshot.forecast.len(),
            snapshot.last_success_label(),
            snapshot.error.as_deref().unwrap_or("none")
        );
    }

    /// FreeRTOS-backed delays are sufficient for the millisecond timings used
    /// by the panel and sample-board reference sequences. Round sub-millisecond
    /// requests up so short sensor waits remain conservative.
    #[derive(Clone, Copy, Debug, Default)]
    struct FreeRtosDelay;

    impl DelayNs for FreeRtosDelay {
        fn delay_ns(&mut self, nanoseconds: u32) {
            let milliseconds = nanoseconds.saturating_add(999_999) / 1_000_000;
            if milliseconds > 0 {
                FreeRtos::delay_ms(milliseconds);
            }
        }
    }
}

#[cfg(target_os = "espidf")]
fn main() -> anyhow::Result<()> {
    firmware::run()
}

#[cfg(not(target_os = "espidf"))]
fn main() {
    println!("Build this firmware for xtensa-esp32s3-espidf. See README.md.");
}
