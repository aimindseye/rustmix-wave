use rustmix_core::hal::{
    ButtonEventKind, ButtonId, ButtonThreshold, InputEvent, InputHal, InputSample, InputTiming,
};

pub const DEFAULT_TOLERANCE_MV: u16 = 150;
pub const X4_DEBOUNCE_MS: u32 = 15;
pub const X4_LONG_PRESS_MS: u32 = 1_000;
pub const X4_REPEAT_MS: u32 = 150;
pub const X4_ADC_OVERSAMPLE: u8 = 4;
pub const X4_INPUT_TIMING: InputTiming = InputTiming::new(
    X4_DEBOUNCE_MS,
    X4_LONG_PRESS_MS,
    X4_REPEAT_MS,
    X4_ADC_OVERSAMPLE,
);

pub const ROW1_THRESHOLDS: [ButtonThreshold; 4] = [
    ButtonThreshold::new(3, 50, ButtonId::Right),
    ButtonThreshold::new(1113, DEFAULT_TOLERANCE_MV, ButtonId::Left),
    ButtonThreshold::new(1984, DEFAULT_TOLERANCE_MV, ButtonId::Select),
    ButtonThreshold::new(2556, DEFAULT_TOLERANCE_MV, ButtonId::Back),
];

pub const ROW2_THRESHOLDS: [ButtonThreshold; 2] = [
    ButtonThreshold::new(3, 50, ButtonId::Down),
    ButtonThreshold::new(1659, DEFAULT_TOLERANCE_MV, ButtonId::Up),
];

const EVENT_QUEUE_LEN: usize = 8;

#[derive(Clone, Copy)]
struct EventQueue {
    buf: [Option<InputEvent>; EVENT_QUEUE_LEN],
}

impl EventQueue {
    const fn new() -> Self {
        Self {
            buf: [None; EVENT_QUEUE_LEN],
        }
    }

    fn push(&mut self, event: InputEvent) {
        for slot in &mut self.buf {
            if slot.is_none() {
                *slot = Some(event);
                return;
            }
        }
    }

    fn pop(&mut self) -> Option<InputEvent> {
        for slot in &mut self.buf {
            if let Some(event) = slot.take() {
                return Some(event);
            }
        }
        None
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

pub struct X4Input {
    stable: Option<ButtonId>,
    candidate: Option<ButtonId>,
    candidate_since_ms: u32,
    press_since_ms: u32,
    long_press_fired: bool,
    last_repeat_ms: u32,
    hold_consumed: bool,
    queue: EventQueue,
}

impl X4Input {
    pub const fn new() -> Self {
        Self {
            stable: None,
            candidate: None,
            candidate_since_ms: 0,
            press_since_ms: 0,
            long_press_fired: false,
            last_repeat_ms: 0,
            hold_consumed: false,
            queue: EventQueue::new(),
        }
    }

    /// Real HAL seam: feed one oversampled ADC/power snapshot and receive at
    /// most one decoded event. Additional queued events can be drained through
    /// `poll()`.
    pub fn ingest_sample(&mut self, sample: InputSample) -> Option<InputEvent> {
        let raw = decode_sample(sample);
        self.step(raw, sample.at_ms);
        self.poll()
    }

    /// Timer-only tick used to generate long-press/repeat events while the
    /// physical state remains stable between ADC samples.
    pub fn tick(&mut self, now_ms: u32) -> Option<InputEvent> {
        self.step(self.stable, now_ms);
        self.poll()
    }

    /// Compatibility/test helper used by earlier bootstrap tests. It performs
    /// raw decode only and queues an immediate Press without debounce state.
    pub fn inject_adc_snapshot(
        &mut self,
        row1_mv: u16,
        row2_mv: u16,
        power_low: bool,
    ) -> Option<InputEvent> {
        let button = decode_sample(InputSample::new(row1_mv, row2_mv, power_low, 0))?;
        self.queue
            .push(InputEvent::new(button, ButtonEventKind::Press));
        self.poll()
    }

    pub fn hold_consumed(&self) -> bool {
        self.hold_consumed
    }

    pub fn stable_button(&self) -> Option<ButtonId> {
        self.stable
    }

    fn step(&mut self, raw: Option<ButtonId>, now_ms: u32) {
        if raw != self.candidate {
            if self.stable.is_some() && raw != self.stable {
                self.press_since_ms = now_ms;
                self.long_press_fired = false;
                self.last_repeat_ms = now_ms;
            }
            self.candidate = raw;
            self.candidate_since_ms = now_ms;
        }

        let debounced =
            if now_ms.saturating_sub(self.candidate_since_ms) >= X4_INPUT_TIMING.debounce_ms {
                self.candidate
            } else {
                self.stable
            };

        if debounced != self.stable {
            if let Some(old) = self.stable {
                self.queue
                    .push(InputEvent::new(old, ButtonEventKind::Release));
                self.hold_consumed = false;
            }
            if let Some(new) = debounced {
                self.queue
                    .push(InputEvent::new(new, ButtonEventKind::Press));
                self.press_since_ms = now_ms;
                self.long_press_fired = false;
                self.last_repeat_ms = now_ms;
            }
            self.stable = debounced;
            return;
        }

        if let Some(button) = self.stable
            && !self.hold_consumed
        {
            let held_ms = now_ms.saturating_sub(self.press_since_ms);
            if !self.long_press_fired && held_ms >= X4_INPUT_TIMING.long_press_ms {
                self.long_press_fired = true;
                self.last_repeat_ms = now_ms;
                self.queue
                    .push(InputEvent::new(button, ButtonEventKind::LongPress));
            } else if self.long_press_fired
                && now_ms.saturating_sub(self.last_repeat_ms) >= X4_INPUT_TIMING.repeat_ms
            {
                self.last_repeat_ms = now_ms;
                self.queue
                    .push(InputEvent::new(button, ButtonEventKind::Repeat));
            }
        }
    }
}

impl Default for X4Input {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHal for X4Input {
    fn poll(&mut self) -> Option<InputEvent> {
        self.queue.pop()
    }

    fn reset_hold_state(&mut self) {
        self.hold_consumed = true;
    }

    fn timing(&self) -> InputTiming {
        X4_INPUT_TIMING
    }

    fn row1_thresholds(&self) -> &[ButtonThreshold] {
        &ROW1_THRESHOLDS
    }

    fn row2_thresholds(&self) -> &[ButtonThreshold] {
        &ROW2_THRESHOLDS
    }
}

fn decode_sample(sample: InputSample) -> Option<ButtonId> {
    if sample.power_low {
        return Some(ButtonId::Power);
    }

    decode_thresholds(sample.row1_mv, &ROW1_THRESHOLDS)
        .or_else(|| decode_thresholds(sample.row2_mv, &ROW2_THRESHOLDS))
}

fn decode_thresholds(mv: u16, thresholds: &[ButtonThreshold]) -> Option<ButtonId> {
    thresholds.iter().find_map(|threshold| {
        if threshold.contains(mv) {
            Some(threshold.logical)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustmix_core::hal::InputHal;

    #[test]
    fn row1_thresholds_preserve_bottom_left_cluster_mapping() {
        let mut input = X4Input::default();
        assert_eq!(
            input.inject_adc_snapshot(3, 4095, false).unwrap().button,
            ButtonId::Right
        );
        assert_eq!(
            input.inject_adc_snapshot(1113, 4095, false).unwrap().button,
            ButtonId::Left
        );
        assert_eq!(
            input.inject_adc_snapshot(1984, 4095, false).unwrap().button,
            ButtonId::Select
        );
        assert_eq!(
            input.inject_adc_snapshot(2556, 4095, false).unwrap().button,
            ButtonId::Back
        );
    }

    #[test]
    fn row2_thresholds_map_vertical_buttons() {
        let mut input = X4Input::default();
        assert_eq!(
            input.inject_adc_snapshot(4095, 3, false).unwrap().button,
            ButtonId::Down
        );
        assert_eq!(
            input.inject_adc_snapshot(4095, 1659, false).unwrap().button,
            ButtonId::Up
        );
    }

    #[test]
    fn power_button_wins_over_adc_ladders() {
        let mut input = X4Input::default();
        assert_eq!(
            input.inject_adc_snapshot(1984, 1659, true).unwrap().button,
            ButtonId::Power
        );
    }

    #[test]
    fn debounced_press_and_release_follow_x4_timing() {
        let mut input = X4Input::default();
        assert_eq!(
            input.ingest_sample(InputSample::new(1984, 4095, false, 0)),
            None
        );
        assert_eq!(
            input.ingest_sample(InputSample::new(1984, 4095, false, 15)),
            Some(InputEvent::new(ButtonId::Select, ButtonEventKind::Press))
        );
        assert_eq!(input.stable_button(), Some(ButtonId::Select));

        assert_eq!(
            input.ingest_sample(InputSample::new(4095, 4095, false, 20)),
            None
        );
        assert_eq!(
            input.ingest_sample(InputSample::new(4095, 4095, false, 35)),
            Some(InputEvent::new(ButtonId::Select, ButtonEventKind::Release))
        );
        assert_eq!(input.stable_button(), None);
    }

    #[test]
    fn long_press_and_repeat_match_proven_driver_policy() {
        let mut input = X4Input::default();
        assert_eq!(
            input.ingest_sample(InputSample::new(1984, 4095, false, 0)),
            None
        );
        assert_eq!(
            input.ingest_sample(InputSample::new(1984, 4095, false, 15)),
            Some(InputEvent::new(ButtonId::Select, ButtonEventKind::Press))
        );
        assert_eq!(
            input.tick(1_015),
            Some(InputEvent::new(
                ButtonId::Select,
                ButtonEventKind::LongPress
            ))
        );
        assert_eq!(
            input.tick(1_165),
            Some(InputEvent::new(ButtonId::Select, ButtonEventKind::Repeat))
        );
    }

    #[test]
    fn reset_hold_state_suppresses_repeat_until_release() {
        let mut input = X4Input::default();
        let _ = input.ingest_sample(InputSample::new(1984, 4095, false, 0));
        let _ = input.ingest_sample(InputSample::new(1984, 4095, false, 15));
        assert_eq!(
            input.tick(1_015),
            Some(InputEvent::new(
                ButtonId::Select,
                ButtonEventKind::LongPress
            ))
        );

        input.reset_hold_state();
        assert!(input.hold_consumed());
        assert_eq!(input.tick(1_500), None);

        let _ = input.ingest_sample(InputSample::new(4095, 4095, false, 1_500));
        assert_eq!(
            input.ingest_sample(InputSample::new(4095, 4095, false, 1_515)),
            Some(InputEvent::new(ButtonId::Select, ButtonEventKind::Release))
        );
        assert!(!input.hold_consumed());
    }
}
