//! Hardware-independent network suspension policy for sleep-image mode.
//!
//! The MCU intentionally remains awake in this milestone so PMIC power-key
//! polling and the proven GPIO45 RTC alarm route stay reliable. Optional
//! network services are suspended while the static e-paper image is visible.

/// Tracks whether optional networking should be paused for sleep-image mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SleepNetworkState {
    suspended: bool,
    suspend_count: u32,
    resume_count: u32,
}

impl SleepNetworkState {
    #[must_use]
    pub const fn is_suspended(self) -> bool {
        self.suspended
    }

    #[must_use]
    pub const fn suspend_count(self) -> u32 {
        self.suspend_count
    }

    #[must_use]
    pub const fn resume_count(self) -> u32 {
        self.resume_count
    }

    /// Enter the paused network state. Returns true only for a real transition.
    pub fn suspend(&mut self) -> bool {
        if self.suspended {
            return false;
        }
        self.suspended = true;
        self.suspend_count = self.suspend_count.saturating_add(1);
        true
    }

    /// Leave the paused network state. Returns true only for a real transition.
    pub fn resume(&mut self) -> bool {
        if !self.suspended {
            return false;
        }
        self.suspended = false;
        self.resume_count = self.resume_count.saturating_add(1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::SleepNetworkState;

    #[test]
    fn suspend_and_resume_are_idempotent() {
        let mut state = SleepNetworkState::default();
        assert!(state.suspend());
        assert!(!state.suspend());
        assert!(state.is_suspended());
        assert_eq!(state.suspend_count(), 1);
        assert!(state.resume());
        assert!(!state.resume());
        assert!(!state.is_suspended());
        assert_eq!(state.resume_count(), 1);
    }
}
