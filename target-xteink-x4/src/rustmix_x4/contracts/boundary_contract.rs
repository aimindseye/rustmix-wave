#![allow(dead_code)]

/// Consolidated Rustmix boundary contract.
///
/// This module is Rustmix-owned. It consolidates the display/input/storage
/// metadata boundaries introduced in earlier extraction steps without moving physical
/// hardware behavior out of the imported X4/X4 runtime yet.
#[cfg(target_arch = "riscv32")]
pub struct RustmixBoundaryContract;

#[cfg(target_arch = "riscv32")]
impl RustmixBoundaryContract {
    /// Validation marker.
    pub const BOUNDARY_CONTRACT_MARKER: &'static str = "x4-boundary-contract-ok";

    /// Current ownership model.
    pub const METADATA_OWNER: &'static str = "Rustmix runtime boundary contract";
    pub const PHYSICAL_BEHAVIOR_OWNER: &'static str = "Rustmix-owned X4 runtime";

    /// The current implementation does not move physical hardware behavior.
    pub const DISPLAY_BEHAVIOR_MOVED_TO_BOUNDARY: bool = false;
    pub const INPUT_BEHAVIOR_MOVED_TO_BOUNDARY: bool = false;
    pub const STORAGE_BEHAVIOR_MOVED_TO_BOUNDARY: bool = false;

    /// Emit only the consolidated contract marker.
    pub fn emit_contract_marker() {
        esp_println::println!("{}", Self::BOUNDARY_CONTRACT_MARKER);
    }

    /// Emit the consolidated boundary marker set.
    ///
    /// Compatibility rule:
    /// - Keep the existing scaffold marker.
    /// - Keep the existing storage boundary marker.
    /// - Keep the existing input boundary marker.
    /// - Keep the existing display boundary marker.
    /// - Add the consolidated consolidated contract marker.
    pub fn emit_all_boundary_markers() {
        Self::emit_contract_marker();
        crate::rustmix_x4::contracts::display::RustmixDisplayBoundary::emit_display_boundary_marker(
        );
        crate::rustmix_x4::contracts::storage::RustmixStorageBoundary::emit_boot_marker();
        crate::rustmix_x4::contracts::input::RustmixInputBoundary::emit_boot_marker();
        crate::rustmix_x4::contracts::display::RustmixDisplayBoundary::emit_scaffold_marker();
    }

    /// Human-readable current ownership summary.
    pub fn ownership_summary() -> &'static str {
        "Rustmix owns boundary metadata/contracts; imported X4/X4 runtime still owns display/input/storage physical behavior"
    }

    /// Physical behavior remains imported until explicit later extraction steps.
    pub fn physical_behavior_is_still_imported() -> bool {
        !Self::DISPLAY_BEHAVIOR_MOVED_TO_BOUNDARY
            && !Self::INPUT_BEHAVIOR_MOVED_TO_BOUNDARY
            && !Self::STORAGE_BEHAVIOR_MOVED_TO_BOUNDARY
    }
}
