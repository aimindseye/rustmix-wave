//! Rust-owned game refresh intent.
//!
//! The game layer reports dirty geometry only. The native panel refresh
//! coordinator owns the one shared partial-refresh counter for menus, Reader
//! screens and SD-loaded games.

use super::dirty_regions::{DirtyRect, DirtyRegions};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefreshTrigger {
    RouteTransition,
    ScriptFrame,
    ManualGhostCleanup,
    SafetyFallback,
}

impl RefreshTrigger {
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::RouteTransition => "route-transition",
            Self::ScriptFrame => "script-frame",
            Self::ManualGhostCleanup => "manual-ghost-cleanup",
            Self::SafetyFallback => "safety-fallback",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameRefreshPlan {
    None,
    /// Existing validated SSD1677 base refresh requested only for explicit
    /// cleanup or invalid geometry. Main-loop coordinator remains final owner.
    Full {
        reason: &'static str,
    },
    /// Existing validated SSD1677 full-screen partial transport. Regions are
    /// diagnostic and renderer-local metadata in this milestone.
    PartialFullscreen {
        regions: Vec<DirtyRect>,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GameRefreshPolicy;

impl GameRefreshPolicy {
    #[must_use]
    pub fn plan(dirty: &DirtyRegions, trigger: RefreshTrigger) -> GameRefreshPlan {
        if trigger == RefreshTrigger::ManualGhostCleanup {
            return GameRefreshPlan::Full {
                reason: "manual-ghost-cleanup",
            };
        }
        if trigger == RefreshTrigger::SafetyFallback || dirty.full_canvas_fallback() {
            return GameRefreshPlan::Full {
                reason: "safety-fallback",
            };
        }
        if dirty.is_empty() {
            return GameRefreshPlan::None;
        }
        GameRefreshPlan::PartialFullscreen {
            regions: dirty.regions().to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GameRefreshPlan, GameRefreshPolicy, RefreshTrigger};
    use crate::games::dirty_regions::{DirtyRect, DirtyRegions};

    #[test]
    fn route_transitions_reuse_proven_partial_fullscreen_transport() {
        let mut dirty = DirtyRegions::default();
        dirty.invalidate(DirtyRect::new(8, 8, 24, 24));
        assert_eq!(
            GameRefreshPolicy::plan(&dirty, RefreshTrigger::RouteTransition),
            GameRefreshPlan::PartialFullscreen {
                regions: vec![DirtyRect::new(8, 8, 24, 24)],
            }
        );
    }

    #[test]
    fn invalid_geometry_and_manual_cleanup_request_safe_global_base() {
        let mut dirty = DirtyRegions::default();
        dirty.invalidate(DirtyRect::new(-1, 0, 8, 8));
        assert_eq!(
            GameRefreshPolicy::plan(&dirty, RefreshTrigger::ScriptFrame),
            GameRefreshPlan::Full {
                reason: "safety-fallback"
            }
        );
        assert_eq!(
            GameRefreshPolicy::plan(&DirtyRegions::default(), RefreshTrigger::ManualGhostCleanup),
            GameRefreshPlan::Full {
                reason: "manual-ghost-cleanup"
            }
        );
    }
}
