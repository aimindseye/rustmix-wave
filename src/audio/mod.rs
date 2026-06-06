//! Native audio domain for the Waveshare ESP32-S3 e-Paper 3.97 board.
//!
//! Alarm/test-tone playback and Voice Notes microphone capture share one
//! ES8311 / I2S0 owner. Compressed audio and SD-backed music playback remain
//! out of scope. Host-testable state lives here; ESP-IDF wiring stays in
//! [`espidf`].

pub mod tone;

#[cfg(target_os = "espidf")]
pub mod board_codec;
#[cfg(target_os = "espidf")]
pub mod espidf;

/// ES8311 seven-bit address when the CE strap is low.
pub const ES8311_I2C_ADDRESS_LOW: u8 = 0x18;
/// ES8311 seven-bit address when the CE strap is high.
pub const ES8311_I2C_ADDRESS_HIGH: u8 = 0x19;
/// Eight-bit wire write address commonly printed by board reference material.
pub const ES8311_WIRE_WRITE_ADDRESS_LOW: u8 = ES8311_I2C_ADDRESS_LOW << 1;
/// Uploaded sample-app playback sample rate.
pub const AUDIO_SAMPLE_RATE_HZ: u32 = 16_000;
/// Uploaded board BSP uses a 384 × sample-rate MCLK for its ES8311 path.
pub const AUDIO_MCLK_MULTIPLE: u32 = 384;
/// 16 kHz × 384 = 6.144 MHz.
pub const AUDIO_MCLK_HZ: u32 = AUDIO_SAMPLE_RATE_HZ * AUDIO_MCLK_MULTIPLE;
/// Match the uploaded Waveshare playback BSP while keeping safe startup mute
/// and amplifier-disable behavior until explicit playback begins.
pub const DEFAULT_AUDIO_VOLUME_PERCENT: u8 = 60;
/// Bounded user-facing audio volume range.
pub const MAX_AUDIO_VOLUME_PERCENT: u8 = 100;
/// Volume adjustment step exposed by the diagnostics UI.
pub const AUDIO_VOLUME_STEP_PERCENT: u8 = 5;
/// I2S TX pin ownership inherited from the uploaded BSP.
pub const AUDIO_MCLK_GPIO: u8 = 13;
pub const AUDIO_BCLK_GPIO: u8 = 14;
pub const AUDIO_WS_GPIO: u8 = 47;
/// ESP32-S3 TX data output to the ES8311 DAC. The uploaded BSP names this
/// signal `I2S_DATA_POUT` and routes it to GPIO48.
pub const AUDIO_DOUT_GPIO: u8 = 48;
/// ES8311 ADC data input back to the ESP32-S3 for Voice Notes capture.
pub const AUDIO_DIN_GPIO: u8 = 21;
pub const AUDIO_AMP_ENABLE_GPIO: u8 = 39;

/// Waveshare sample-app codec-profile values inherited from Espressif's
/// `esp_codec_dev` ES8311 open/start sequence. These are deliberately explicit
/// so a successful I2C probe cannot be mistaken for a working DAC path.
pub const BSP_ES8311_GPIO_IDLE_REG44: u8 = 0x08;
pub const BSP_ES8311_DAC_REFERENCE_REG44: u8 = 0x58;
pub const BSP_ES8311_SYSTEM_REG14: u8 = 0x1A;
pub const BSP_ES8311_ADC_REG15: u8 = 0x40;
pub const BSP_ES8311_ADC_REG17: u8 = 0xBF;
pub const BSP_ES8311_GP_REG45: u8 = 0x00;

/// Product-facing audio state.  Keep this small so it can be copied into UI
/// snapshots without retaining I2C or I2S handles.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AudioPlaybackState {
    #[default]
    Unavailable,
    Muted,
    Ready,
    PlayingTestTone,
    PlayingAlarm,
    PlayingVoiceNote,
    RecordingVoiceNote,
    Error,
}

impl AudioPlaybackState {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unavailable => "UNAVAILABLE",
            Self::Muted => "MUTED",
            Self::Ready => "READY",
            Self::PlayingTestTone => "TEST TONE",
            Self::PlayingAlarm => "ALARM CHIME",
            Self::PlayingVoiceNote => "VOICE NOTE",
            Self::RecordingVoiceNote => "VOICE RECORD",
            Self::Error => "ERROR",
        }
    }
}

/// Password-free, handle-free audio state rendered by the product shell.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioSnapshot {
    pub available: bool,
    pub codec_address: Option<u8>,
    pub codec_ready: bool,
    pub i2s_ready: bool,
    pub amplifier_enabled: bool,
    pub muted: bool,
    pub volume_percent: u8,
    pub playback_state: AudioPlaybackState,
    pub error: Option<String>,
}

impl Default for AudioSnapshot {
    fn default() -> Self {
        Self::unavailable("audio subsystem has not been initialized")
    }
}

impl AudioSnapshot {
    #[must_use]
    pub fn unavailable(error: impl Into<String>) -> Self {
        Self {
            available: false,
            codec_address: None,
            codec_ready: false,
            i2s_ready: false,
            amplifier_enabled: false,
            muted: true,
            volume_percent: DEFAULT_AUDIO_VOLUME_PERCENT,
            playback_state: AudioPlaybackState::Unavailable,
            error: Some(error.into()),
        }
    }

    #[must_use]
    pub const fn home_badge(&self) -> &'static str {
        match self.playback_state {
            AudioPlaybackState::Unavailable => "NO AUD",
            AudioPlaybackState::Muted => "MUTED",
            AudioPlaybackState::Ready => "READY",
            AudioPlaybackState::PlayingTestTone => "TEST",
            AudioPlaybackState::PlayingAlarm => "RING",
            AudioPlaybackState::PlayingVoiceNote => "NOTE",
            AudioPlaybackState::RecordingVoiceNote => "REC",
            AudioPlaybackState::Error => "ERROR",
        }
    }

    #[must_use]
    pub const fn alarm_label(&self) -> &'static str {
        match self.playback_state {
            AudioPlaybackState::PlayingAlarm => "Audible alarm chime is active.",
            AudioPlaybackState::PlayingVoiceNote => "Saved voice-note playback owns the codec.",
            AudioPlaybackState::RecordingVoiceNote => "Voice-note recording owns the codec.",
            AudioPlaybackState::Unavailable | AudioPlaybackState::Error => {
                "Audio unavailable - visual alarm only."
            }
            _ => "Audio is ready for the alarm chime.",
        }
    }

    #[must_use]
    pub fn codec_address_label(&self) -> String {
        self.codec_address.map_or_else(
            || "--".into(),
            |address| format!("0x{address:02X} (wire 0x{:02X})", address << 1),
        )
    }
}

/// Hardware-independent requests produced by the Audio diagnostics screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioUiRequest {
    PlayTestChime,
    StopPlayback,
    VolumeUp,
    VolumeDown,
    ToggleMute,
}

#[cfg(test)]
mod tests {
    use super::{
        AudioPlaybackState, AudioSnapshot, AUDIO_DIN_GPIO, AUDIO_DOUT_GPIO, BSP_ES8311_ADC_REG15,
        BSP_ES8311_ADC_REG17, BSP_ES8311_DAC_REFERENCE_REG44, BSP_ES8311_GP_REG45,
        BSP_ES8311_SYSTEM_REG14, DEFAULT_AUDIO_VOLUME_PERCENT, ES8311_I2C_ADDRESS_LOW,
        ES8311_WIRE_WRITE_ADDRESS_LOW,
    };

    #[test]
    fn seven_bit_and_wire_write_addresses_are_explicit() {
        assert_eq!(ES8311_I2C_ADDRESS_LOW, 0x18);
        assert_eq!(ES8311_WIRE_WRITE_ADDRESS_LOW, 0x30);
    }

    #[test]
    fn uploaded_bsp_data_route_keeps_tx_and_deferred_rx_explicit() {
        assert_eq!(AUDIO_DOUT_GPIO, 48);
        assert_eq!(AUDIO_DIN_GPIO, 21);
    }

    #[test]
    fn waveshare_codec_profile_keeps_dac_reference_explicit() {
        assert_eq!(BSP_ES8311_DAC_REFERENCE_REG44, 0x58);
        assert_eq!(BSP_ES8311_SYSTEM_REG14, 0x1A);
        assert_eq!(BSP_ES8311_ADC_REG15, 0x40);
        assert_eq!(BSP_ES8311_ADC_REG17, 0xBF);
        assert_eq!(BSP_ES8311_GP_REG45, 0x00);
        assert_eq!(DEFAULT_AUDIO_VOLUME_PERCENT, 60);
    }

    #[test]
    fn unavailable_audio_falls_back_to_visual_alarm() {
        let snapshot = AudioSnapshot::unavailable("codec absent");
        assert_eq!(snapshot.playback_state, AudioPlaybackState::Unavailable);
        assert_eq!(snapshot.home_badge(), "NO AUD");
        assert!(snapshot.alarm_label().contains("visual alarm"));
    }
}
