//! ESP-IDF playback and Voice Notes runtime for the ES8311 codec and bidirectional I2S0 channel.

use anyhow::{anyhow, Result};
use embedded_hal::{delay::DelayNs, i2c::I2c};
use es8311::{ClockConfig, Resolution};
use esp_idf_svc::hal::{
    delay::{BLOCK, NON_BLOCK},
    gpio::{Output, PinDriver},
    i2s::{I2sBiDir, I2sDriver},
};

use crate::voice_notes::{
    apply_pcm16_gain_in_place, expand_pcm16_mono_to_stereo, VoiceCaptureMetrics, VoiceMicGain,
};

use super::{
    board_codec::{BoardEs8311, CodecProfileSnapshot},
    tone::{ChimeGenerator, ChimeMode, PCM_CHUNK_BYTES},
    AudioPlaybackState, AudioSnapshot, AudioUiRequest, AUDIO_MCLK_HZ, AUDIO_SAMPLE_RATE_HZ,
    AUDIO_VOLUME_STEP_PERCENT, DEFAULT_AUDIO_VOLUME_PERCENT, ES8311_I2C_ADDRESS_HIGH,
    ES8311_I2C_ADDRESS_LOW, MAX_AUDIO_VOLUME_PERCENT,
};

/// Own the safe-start playback runtime. The amplifier is held low unless PCM
/// audio is actively being streamed.
pub struct AudioRuntime<'d, I2C> {
    bus: I2C,
    codec: BoardEs8311,
    profile: CodecProfileSnapshot,
    tx: I2sDriver<'d, I2sBiDir>,
    amplifier: PinDriver<'d, Output>,
    snapshot: AudioSnapshot,
    chime: ChimeGenerator,
}

impl<'d, I2C> AudioRuntime<'d, I2C>
where
    I2C: I2c,
    I2C::Error: core::fmt::Debug,
{
    pub fn initialize<D>(
        mut bus: I2C,
        tx: I2sDriver<'d, I2sBiDir>,
        mut amplifier: PinDriver<'d, Output>,
        delay: &mut D,
    ) -> Result<Self>
    where
        D: DelayNs,
    {
        amplifier
            .set_low()
            .map_err(|error| anyhow!("failed to disable audio amplifier: {error:?}"))?;
        let clock = ClockConfig {
            mclk_inverted: false,
            sclk_inverted: false,
            mclk_from_mclk_pin: true,
            mclk_frequency: AUDIO_MCLK_HZ,
            sample_frequency: AUDIO_SAMPLE_RATE_HZ,
        };
        let mut last_error = None;
        let mut detected = None;
        for address in [ES8311_I2C_ADDRESS_LOW, ES8311_I2C_ADDRESS_HIGH] {
            let codec = BoardEs8311::new(address);
            match codec.init(
                &mut bus,
                &clock,
                Resolution::Bits16,
                Resolution::Bits16,
                delay,
            ) {
                Ok(profile) => {
                    detected = Some((codec, address, profile));
                    break;
                }
                Err(error) => last_error = Some(format!("{error:?}")),
            }
        }
        let (codec, codec_address, profile) = detected.ok_or_else(|| {
            anyhow!(
                "ES8311 probe failed at 0x{ES8311_I2C_ADDRESS_LOW:02X} and 0x{ES8311_I2C_ADDRESS_HIGH:02X}: {}",
                last_error.unwrap_or_else(|| "unknown codec error".into())
            )
        })?;
        codec
            .volume_set(&mut bus, DEFAULT_AUDIO_VOLUME_PERCENT, None)
            .map_err(|error| anyhow!("failed to set ES8311 volume: {error:?}"))?;
        codec
            .mute(&mut bus, true)
            .map_err(|error| anyhow!("failed to mute ES8311: {error:?}"))?;

        Ok(Self {
            bus,
            codec,
            profile,
            tx,
            amplifier,
            snapshot: AudioSnapshot {
                available: true,
                codec_address: Some(codec_address),
                codec_ready: true,
                i2s_ready: true,
                amplifier_enabled: false,
                muted: true,
                volume_percent: DEFAULT_AUDIO_VOLUME_PERCENT,
                playback_state: AudioPlaybackState::Muted,
                error: None,
            },
            chime: ChimeGenerator::default(),
        })
    }

    #[must_use]
    pub fn snapshot(&self) -> AudioSnapshot {
        self.snapshot.clone()
    }

    #[must_use]
    pub const fn profile(&self) -> CodecProfileSnapshot {
        self.profile
    }

    pub fn start_alarm_chime(&mut self) -> Result<()> {
        self.begin_playback(ChimeMode::AlarmRepeat)
    }

    pub fn apply_request(&mut self, request: AudioUiRequest) -> Result<&'static str> {
        match request {
            AudioUiRequest::PlayTestChime => {
                self.begin_playback(ChimeMode::TestOnce)?;
                Ok("test-tone-start")
            }
            AudioUiRequest::StopPlayback => {
                self.stop_playback()?;
                Ok("playback-stop")
            }
            AudioUiRequest::VolumeUp => {
                self.set_volume(
                    self.snapshot
                        .volume_percent
                        .saturating_add(AUDIO_VOLUME_STEP_PERCENT),
                )?;
                Ok("volume-up")
            }
            AudioUiRequest::VolumeDown => {
                self.set_volume(
                    self.snapshot
                        .volume_percent
                        .saturating_sub(AUDIO_VOLUME_STEP_PERCENT),
                )?;
                Ok("volume-down")
            }
            AudioUiRequest::ToggleMute => {
                self.set_muted(!self.snapshot.muted)?;
                Ok(if self.snapshot.muted {
                    "muted"
                } else {
                    "unmuted"
                })
            }
        }
    }

    /// Feed one bounded DMA chunk. Returns `true` when the visible diagnostics
    /// state changes, for example when a one-shot test chime completes.
    pub fn tick(&mut self) -> Result<bool> {
        if !self.chime.is_playing() {
            return Ok(false);
        }
        let mut bytes = [0_u8; PCM_CHUNK_BYTES];
        let completed_test = self.chime.fill_stereo_pcm(
            &mut bytes,
            self.snapshot.volume_percent,
            self.snapshot.muted,
        );
        self.tx
            .write_all(&bytes, BLOCK)
            .map_err(|error| anyhow!("I2S TX write failed: {error:?}"))?;
        if completed_test {
            self.stop_playback()?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn begin_voice_note_playback(&mut self) -> Result<()> {
        self.chime.stop();
        self.codec
            .mute(&mut self.bus, false)
            .map_err(|error| anyhow!("failed to unmute ES8311 for voice note: {error:?}"))?;
        if let Err(error) = self.amplifier.set_high() {
            let _ = self.codec.mute(&mut self.bus, true);
            return Err(anyhow!("failed to enable audio amplifier: {error:?}"));
        }
        self.snapshot.amplifier_enabled = true;
        self.snapshot.muted = false;
        self.snapshot.playback_state = AudioPlaybackState::PlayingVoiceNote;
        self.snapshot.error = None;
        Ok(())
    }

    pub fn write_voice_pcm16_mono(&mut self, mono: &[u8], stereo: &mut [u8]) -> Result<()> {
        if self.snapshot.playback_state != AudioPlaybackState::PlayingVoiceNote {
            return Err(anyhow!("voice-note playback is not active"));
        }
        let stereo_bytes = expand_pcm16_mono_to_stereo(mono, stereo)?;
        self.tx
            .write_all(&stereo[..stereo_bytes], BLOCK)
            .map_err(|error| anyhow!("I2S voice-note TX write failed: {error:?}"))
    }

    pub fn finish_voice_note_playback(&mut self) -> Result<()> {
        self.stop_playback()
    }

    pub fn begin_voice_recording(&mut self) -> Result<()> {
        self.chime.stop();
        self.amplifier
            .set_low()
            .map_err(|error| anyhow!("failed to disable audio amplifier: {error:?}"))?;
        self.codec
            .mute(&mut self.bus, true)
            .map_err(|error| anyhow!("failed to mute ES8311 DAC before recording: {error:?}"))?;
        self.snapshot.amplifier_enabled = false;
        self.snapshot.muted = true;
        self.snapshot.playback_state = AudioPlaybackState::RecordingVoiceNote;
        self.snapshot.error = None;
        Ok(())
    }

    pub fn finish_voice_recording(&mut self) -> Result<()> {
        self.snapshot.playback_state = AudioPlaybackState::Muted;
        self.snapshot.muted = true;
        self.snapshot.amplifier_enabled = false;
        Ok(())
    }

    pub fn read_voice_pcm_mono(
        &mut self,
        stereo: &mut [u8],
        mono: &mut [u8],
        gain: VoiceMicGain,
    ) -> Result<VoiceCaptureMetrics> {
        if stereo.len() < mono.len().saturating_mul(2) || mono.len() % 2 != 0 {
            return Err(anyhow!("invalid voice PCM buffers"));
        }
        let bytes = match self.tx.read(stereo, NON_BLOCK) {
            Ok(bytes) => bytes,
            Err(error) if error.code() == esp_idf_svc::sys::ESP_ERR_TIMEOUT => {
                return Ok(VoiceCaptureMetrics::default());
            }
            Err(error) => return Err(anyhow!("I2S RX read failed: {error:?}")),
        };
        let frames = (bytes / 4).min(mono.len() / 2);
        for frame in 0..frames {
            let source = frame * 4;
            let target = frame * 2;
            mono[target..target + 2].copy_from_slice(&stereo[source..source + 2]);
        }
        Ok(apply_pcm16_gain_in_place(&mut mono[..frames * 2], gain))
    }

    /// Drain one bounded I2S RX chunk while a voice-note recording is paused.
    /// This keeps stale microphone frames out of the resumed WAV stream without
    /// moving codec or DMA ownership away from the native main-loop runtime.
    pub fn discard_voice_pcm(&mut self, stereo: &mut [u8]) -> Result<usize> {
        match self.tx.read(stereo, NON_BLOCK) {
            Ok(bytes) => Ok(bytes),
            Err(error) if error.code() == esp_idf_svc::sys::ESP_ERR_TIMEOUT => Ok(0),
            Err(error) => Err(anyhow!("I2S RX discard failed: {error:?}")),
        }
    }

    pub fn stop_playback(&mut self) -> Result<()> {
        self.chime.stop();
        self.amplifier
            .set_low()
            .map_err(|error| anyhow!("failed to disable audio amplifier: {error:?}"))?;
        self.codec
            .mute(&mut self.bus, true)
            .map_err(|error| anyhow!("failed to mute ES8311: {error:?}"))?;
        self.snapshot.amplifier_enabled = false;
        self.snapshot.muted = true;
        self.snapshot.playback_state = AudioPlaybackState::Muted;
        Ok(())
    }

    pub fn record_failure(&mut self, error: impl Into<String>) {
        let _ = self.amplifier.set_low();
        let _ = self.codec.mute(&mut self.bus, true);
        self.chime.stop();
        self.snapshot.amplifier_enabled = false;
        self.snapshot.muted = true;
        self.snapshot.playback_state = AudioPlaybackState::Error;
        self.snapshot.error = Some(error.into());
    }

    fn begin_playback(&mut self, mode: ChimeMode) -> Result<()> {
        match mode {
            ChimeMode::TestOnce => self.chime.start_test_once(),
            ChimeMode::AlarmRepeat => self.chime.start_alarm_repeat(),
            ChimeMode::Idle => self.chime.stop(),
        }
        self.codec
            .mute(&mut self.bus, false)
            .map_err(|error| anyhow!("failed to unmute ES8311: {error:?}"))?;
        self.amplifier
            .set_high()
            .map_err(|error| anyhow!("failed to enable audio amplifier: {error:?}"))?;
        self.snapshot.muted = false;
        self.snapshot.amplifier_enabled = true;
        self.snapshot.playback_state = match mode {
            ChimeMode::TestOnce => AudioPlaybackState::PlayingTestTone,
            ChimeMode::AlarmRepeat => AudioPlaybackState::PlayingAlarm,
            ChimeMode::Idle => AudioPlaybackState::Ready,
        };
        self.snapshot.error = None;
        Ok(())
    }

    fn set_muted(&mut self, muted: bool) -> Result<()> {
        self.codec
            .mute(&mut self.bus, muted)
            .map_err(|error| anyhow!("failed to change ES8311 mute state: {error:?}"))?;
        let voice_note_playing =
            self.snapshot.playback_state == AudioPlaybackState::PlayingVoiceNote;
        if muted || (!self.chime.is_playing() && !voice_note_playing) {
            self.amplifier
                .set_low()
                .map_err(|error| anyhow!("failed to disable audio amplifier: {error:?}"))?;
            self.snapshot.amplifier_enabled = false;
        } else {
            self.amplifier
                .set_high()
                .map_err(|error| anyhow!("failed to enable audio amplifier: {error:?}"))?;
            self.snapshot.amplifier_enabled = true;
        }
        self.snapshot.muted = muted;
        self.snapshot.playback_state = if self.chime.mode() == ChimeMode::AlarmRepeat {
            AudioPlaybackState::PlayingAlarm
        } else if self.chime.mode() == ChimeMode::TestOnce {
            AudioPlaybackState::PlayingTestTone
        } else if voice_note_playing {
            AudioPlaybackState::PlayingVoiceNote
        } else if muted {
            AudioPlaybackState::Muted
        } else {
            AudioPlaybackState::Ready
        };
        Ok(())
    }

    fn set_volume(&mut self, requested: u8) -> Result<()> {
        let volume = requested.min(MAX_AUDIO_VOLUME_PERCENT);
        self.codec
            .volume_set(&mut self.bus, volume, None)
            .map_err(|error| anyhow!("failed to set ES8311 volume: {error:?}"))?;
        self.snapshot.volume_percent = volume;
        Ok(())
    }
}
