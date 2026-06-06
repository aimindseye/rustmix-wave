//! Fixed-point PCM chime generation.
//!
//! Avoid runtime floating-point trigonometry and large embedded PCM arrays.
//! A small triangle oscillator is sufficient for an audible hardware test and
//! alarm cadence while keeping host tests deterministic.

use super::AUDIO_SAMPLE_RATE_HZ;

pub const PCM_CHANNELS: usize = 2;
pub const PCM_BYTES_PER_SAMPLE: usize = 2;
pub const PCM_BYTES_PER_FRAME: usize = PCM_CHANNELS * PCM_BYTES_PER_SAMPLE;
pub const PCM_CHUNK_FRAMES: usize = 320;
pub const PCM_CHUNK_BYTES: usize = PCM_CHUNK_FRAMES * PCM_BYTES_PER_FRAME;
const BASE_AMPLITUDE: i32 = 12_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ChimeMode {
    #[default]
    Idle,
    TestOnce,
    AlarmRepeat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ChimeSegment {
    frequency_hz: u32,
    duration_ms: u32,
}

const CHIME_SEGMENTS: [ChimeSegment; 4] = [
    ChimeSegment {
        frequency_hz: 440,
        duration_ms: 250,
    },
    ChimeSegment {
        frequency_hz: 0,
        duration_ms: 150,
    },
    ChimeSegment {
        frequency_hz: 660,
        duration_ms: 250,
    },
    ChimeSegment {
        frequency_hz: 0,
        duration_ms: 1_000,
    },
];

/// Stateful, allocation-free chime generator.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChimeGenerator {
    mode: ChimeMode,
    segment_index: usize,
    frames_in_segment: u32,
    phase: u32,
}

impl ChimeGenerator {
    pub fn start_test_once(&mut self) {
        self.start(ChimeMode::TestOnce);
    }

    pub fn start_alarm_repeat(&mut self) {
        self.start(ChimeMode::AlarmRepeat);
    }

    pub fn stop(&mut self) {
        self.start(ChimeMode::Idle);
    }

    #[must_use]
    pub const fn mode(&self) -> ChimeMode {
        self.mode
    }

    #[must_use]
    pub const fn is_playing(&self) -> bool {
        !matches!(self.mode, ChimeMode::Idle)
    }

    /// Fill signed-16-bit little-endian stereo PCM. Returns `true` when a
    /// one-shot test chime finishes during this chunk.
    pub fn fill_stereo_pcm(&mut self, output: &mut [u8], volume_percent: u8, muted: bool) -> bool {
        assert_eq!(output.len() % PCM_BYTES_PER_FRAME, 0);
        let mut completed_test = false;
        for frame in output.chunks_exact_mut(PCM_BYTES_PER_FRAME) {
            let sample = if self.is_playing() {
                self.next_sample(volume_percent, muted, &mut completed_test)
            } else {
                0
            };
            let bytes = sample.to_le_bytes();
            frame[0] = bytes[0];
            frame[1] = bytes[1];
            frame[2] = bytes[0];
            frame[3] = bytes[1];
        }
        completed_test
    }

    fn start(&mut self, mode: ChimeMode) {
        self.mode = mode;
        self.segment_index = 0;
        self.frames_in_segment = 0;
        self.phase = 0;
    }

    fn next_sample(&mut self, volume_percent: u8, muted: bool, completed_test: &mut bool) -> i16 {
        let segment = CHIME_SEGMENTS[self.segment_index];
        let sample = if muted || segment.frequency_hz == 0 {
            0
        } else {
            self.phase = self.phase.wrapping_add(phase_step(segment.frequency_hz));
            scale_sample(triangle_sample(self.phase), volume_percent)
        };
        self.frames_in_segment += 1;
        let segment_frames = frames_for_ms(segment.duration_ms);
        if self.frames_in_segment >= segment_frames {
            self.frames_in_segment = 0;
            self.phase = 0;
            self.segment_index += 1;
            if self.segment_index >= CHIME_SEGMENTS.len() {
                self.segment_index = 0;
                if self.mode == ChimeMode::TestOnce {
                    self.mode = ChimeMode::Idle;
                    *completed_test = true;
                }
            }
        }
        sample
    }
}

#[must_use]
const fn frames_for_ms(duration_ms: u32) -> u32 {
    AUDIO_SAMPLE_RATE_HZ.saturating_mul(duration_ms) / 1_000
}

#[must_use]
const fn phase_step(frequency_hz: u32) -> u32 {
    (((frequency_hz as u64) << 32) / AUDIO_SAMPLE_RATE_HZ as u64) as u32
}

#[must_use]
fn triangle_sample(phase: u32) -> i16 {
    let ramp = ((phase >> 16) & 0xFFFF) as i32;
    let centered = if ramp < 32_768 {
        ramp * 2 - 32_768
    } else {
        (65_535 - ramp) * 2 - 32_768
    };
    (centered * BASE_AMPLITUDE / 32_768) as i16
}

#[must_use]
fn scale_sample(sample: i16, volume_percent: u8) -> i16 {
    let volume = i32::from(volume_percent.min(100));
    (i32::from(sample) * volume / 100) as i16
}

#[cfg(test)]
mod tests {
    use super::{scale_sample, ChimeGenerator, ChimeMode, PCM_BYTES_PER_FRAME, PCM_CHUNK_BYTES};

    #[test]
    fn generated_pcm_is_stereo_and_bounded() {
        let mut generator = ChimeGenerator::default();
        generator.start_test_once();
        let mut bytes = [0_u8; PCM_CHUNK_BYTES];
        generator.fill_stereo_pcm(&mut bytes, 20, false);
        for frame in bytes.chunks_exact(PCM_BYTES_PER_FRAME) {
            assert_eq!(&frame[..2], &frame[2..]);
            let sample = i16::from_le_bytes([frame[0], frame[1]]);
            assert!(sample.unsigned_abs() <= 2_400);
        }
    }

    #[test]
    fn mute_produces_silence() {
        let mut generator = ChimeGenerator::default();
        generator.start_alarm_repeat();
        let mut bytes = [1_u8; PCM_CHUNK_BYTES];
        generator.fill_stereo_pcm(&mut bytes, 100, true);
        assert!(bytes.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn volume_scaling_is_bounded() {
        assert_eq!(scale_sample(12_000, 0), 0);
        assert_eq!(scale_sample(12_000, 20), 2_400);
        assert_eq!(scale_sample(12_000, 200), 12_000);
    }

    #[test]
    fn one_shot_chime_finishes_but_alarm_repeats() {
        let mut generator = ChimeGenerator::default();
        generator.start_test_once();
        let mut bytes = [0_u8; PCM_CHUNK_BYTES];
        let mut completed = false;
        for _ in 0..100 {
            completed |= generator.fill_stereo_pcm(&mut bytes, 20, false);
        }
        assert!(completed);
        assert_eq!(generator.mode(), ChimeMode::Idle);

        generator.start_alarm_repeat();
        for _ in 0..100 {
            generator.fill_stereo_pcm(&mut bytes, 20, false);
        }
        assert_eq!(generator.mode(), ChimeMode::AlarmRepeat);
    }
}
