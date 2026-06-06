//! Reusable application modules for the Waveshare ESP32-S3 e-Paper 3.97 board.
//!
//! Hardware-independent code stays in this library so framebuffer, routing,
//! widgets and protocol helpers can be unit-tested on the host. ESP-IDF wiring
//! remains isolated in `main.rs`.

pub mod alarm;
pub mod app;
pub mod audio;
pub mod board_services;
pub mod build_info;
pub mod buttons;
pub mod calendar;
pub mod dictionary;
pub mod environment;
pub mod epaper;
pub mod epub;
pub mod framebuffer;
pub mod games;
pub mod imu;
pub mod imu_events;
pub mod keyboard_navigation;
pub mod lua_runtime;
pub mod network;
pub mod network_config;
pub mod ntp;
pub mod orientation;
pub mod panel_refresh;
pub mod power;
pub mod power_key;
pub mod power_key_menu;
pub mod reader;
pub mod regional;
pub mod rtc;
pub mod rtc_alarm_interrupt;
pub mod runtime_memory;
pub mod runtime_worker;
pub mod shared_i2c;
pub mod sleep_images;
pub mod sleep_mode;
pub mod sleep_network;

pub mod storage;
pub mod unit_converter;
pub mod voice_note_metadata;
pub mod voice_notes;
pub mod weather;
pub mod weather_config;
pub mod wifi_transfer;
