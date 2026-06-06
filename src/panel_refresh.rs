//! Native panel refresh coordinator.
//!
//! All e-paper mode decisions remain Rust-owned. SD-loaded applications may
//! submit dirty rectangles and draw intent, but they never select SSD1677
//! commands directly. The coordinator deliberately retains the proven
//! full-screen partial transport until windowed RAM writes receive their own
//! isolated hardware experiment.

/// Periodic ghost-cleanup cadence shared by menus, Reader screens and games.
pub const PANEL_PARTIAL_REFRESH_LIMIT: u8 = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelRefreshRequest {
    Normal,
    AfterWake,
    ManualGhostCleanup,
    SafetyFallback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelGlobalReason {
    InitialBoot,
    AfterWake,
    ManualGhostCleanup,
    PeriodicCleanup,
    SafetyFallback,
    SleepImage,
}

impl PanelGlobalReason {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::InitialBoot => "initial-boot",
            Self::AfterWake => "after-wake",
            Self::ManualGhostCleanup => "manual-ghost-cleanup",
            Self::PeriodicCleanup => "ghost-cleanup-threshold",
            Self::SafetyFallback => "safety-fallback",
            Self::SleepImage => "sleep-image",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelRefreshPlan {
    PartialFullscreen { partial_count: u8 },
    GlobalBase { reason: PanelGlobalReason },
}

/// One counter for every normal UI and game refresh. This replaces the former
/// split between the six-refresh UI counter and the independent game policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PanelRefreshCoordinator {
    partial_count: u8,
}

impl PanelRefreshCoordinator {
    #[must_use]
    pub const fn partial_count(self) -> u8 {
        self.partial_count
    }

    pub fn reset_after_external_global(&mut self, _reason: PanelGlobalReason) {
        self.partial_count = 0;
    }

    #[must_use]
    pub fn plan(&mut self, request: PanelRefreshRequest) -> PanelRefreshPlan {
        let forced = match request {
            PanelRefreshRequest::Normal => None,
            PanelRefreshRequest::AfterWake => Some(PanelGlobalReason::AfterWake),
            PanelRefreshRequest::ManualGhostCleanup => Some(PanelGlobalReason::ManualGhostCleanup),
            PanelRefreshRequest::SafetyFallback => Some(PanelGlobalReason::SafetyFallback),
        };
        if let Some(reason) = forced {
            self.partial_count = 0;
            return PanelRefreshPlan::GlobalBase { reason };
        }
        if self.partial_count >= PANEL_PARTIAL_REFRESH_LIMIT {
            self.partial_count = 0;
            return PanelRefreshPlan::GlobalBase {
                reason: PanelGlobalReason::PeriodicCleanup,
            };
        }
        self.partial_count = self.partial_count.saturating_add(1);
        PanelRefreshPlan::PartialFullscreen {
            partial_count: self.partial_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PanelGlobalReason, PanelRefreshCoordinator, PanelRefreshPlan, PanelRefreshRequest,
        PANEL_PARTIAL_REFRESH_LIMIT,
    };

    #[test]
    fn normal_routes_share_one_partial_counter_before_periodic_cleanup() {
        let mut coordinator = PanelRefreshCoordinator::default();
        for partial_count in 1..=PANEL_PARTIAL_REFRESH_LIMIT {
            assert_eq!(
                coordinator.plan(PanelRefreshRequest::Normal),
                PanelRefreshPlan::PartialFullscreen { partial_count }
            );
        }
        assert_eq!(
            coordinator.plan(PanelRefreshRequest::Normal),
            PanelRefreshPlan::GlobalBase {
                reason: PanelGlobalReason::PeriodicCleanup
            }
        );
    }

    #[test]
    fn wake_manual_and_safety_requests_force_global_base() {
        let mut coordinator = PanelRefreshCoordinator::default();
        let _ = coordinator.plan(PanelRefreshRequest::Normal);
        assert_eq!(
            coordinator.plan(PanelRefreshRequest::AfterWake),
            PanelRefreshPlan::GlobalBase {
                reason: PanelGlobalReason::AfterWake
            }
        );
        assert_eq!(coordinator.partial_count(), 0);
        assert_eq!(
            coordinator.plan(PanelRefreshRequest::ManualGhostCleanup),
            PanelRefreshPlan::GlobalBase {
                reason: PanelGlobalReason::ManualGhostCleanup
            }
        );
        assert_eq!(
            coordinator.plan(PanelRefreshRequest::SafetyFallback),
            PanelRefreshPlan::GlobalBase {
                reason: PanelGlobalReason::SafetyFallback
            }
        );
    }
}
