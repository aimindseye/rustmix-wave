#![allow(dead_code)]

//! Consolidated Rustmix-owned read-only storage boundary for the Xteink X4 target.
//!
//! This public boundary entrypoint composes the Rustmix facade with the X4-backed
//! bridge while active SD mount/probe, FAT, SPI arbitration, display, reader, and
//! file-browser behavior remain in `target-xteink-x4/src/rustmix_x4`.

use crate::rustmix_x4::io::storage_readonly_adapter::{
    RustmixDirectoryEntry, RustmixReadonlyStorage, RustmixReadonlyStorageContract,
    RustmixReadonlyStorageFacade, RustmixResolvedStoragePaths, RustmixStoragePathRef,
    RustmixStorageReadChunk,
};
use crate::rustmix_x4::io::storage_readonly_x4_bridge::{
    RustmixStorageReadonlyX4BridgeContract, X4ReadonlyStorageBackend, X4ReadonlyStorageBridge,
    X4ReadonlyStorageBridgeError,
};

pub const STORAGE_READONLY_BOUNDARY_MARKER: &str = "x4-storage-readonly-boundary-ok";
pub const STORAGE_READONLY_BOUNDARY_OWNER: &str = "target-xteink-x4 Rustmix read-only boundary";
pub const STORAGE_READONLY_BOUNDARY_ACTIVE_BACKEND_OWNER: &str = "Rustmix-owned X4 runtime";

pub struct RustmixStorageReadonlyBoundaryContract;

impl RustmixStorageReadonlyBoundaryContract {
    pub const BOUNDARY_MARKER: &'static str = STORAGE_READONLY_BOUNDARY_MARKER;
    pub const BOUNDARY_OWNER: &'static str = STORAGE_READONLY_BOUNDARY_OWNER;
    pub const ACTIVE_BACKEND_OWNER: &'static str = STORAGE_READONLY_BOUNDARY_ACTIVE_BACKEND_OWNER;
    pub const PUBLIC_CONTRACT_LAYER: &'static str = "RustmixReadonlyStorage facade trait";
    pub const ACTIVE_IMPLEMENTATION_LAYER: &'static str = "X4ReadonlyStorageBridge";
    pub const EMBEDDED_BACKEND_LAYER: &'static str = "X4X4ReadonlyStorageBackend";
    pub const FACADE_SOURCE: &'static str = "rustmix_x4/io/storage_readonly_adapter.rs";
    pub const BRIDGE_SOURCE: &'static str = "rustmix_x4/io/storage_readonly_x4_bridge.rs";
    pub const BOUNDARY_SOURCE: &'static str = "rustmix_x4/io/storage_readonly_boundary.rs";
    pub const CANONICAL_DOC_SOURCE: &'static str = "docs/architecture/storage-readonly-boundary.md";

    pub const SD_MOUNT_OR_PROBE_MOVED_TO_BOUNDARY: bool = false;
    pub const SD_DRIVER_MOVED_TO_BOUNDARY: bool = false;
    pub const FAT_BEHAVIOR_MOVED_TO_BOUNDARY: bool = false;
    pub const SPI_ARBITRATION_MOVED_TO_BOUNDARY: bool = false;
    pub const DISPLAY_BEHAVIOR_MOVED_TO_BOUNDARY: bool = false;
    pub const READER_OR_FILE_BROWSER_BEHAVIOR_CHANGED: bool = false;
    pub const WRITABLE_STORAGE_BEHAVIOR_ADDED: bool = false;

    pub const fn physical_behavior_moved() -> bool {
        Self::SD_MOUNT_OR_PROBE_MOVED_TO_BOUNDARY
            || Self::SD_DRIVER_MOVED_TO_BOUNDARY
            || Self::FAT_BEHAVIOR_MOVED_TO_BOUNDARY
            || Self::SPI_ARBITRATION_MOVED_TO_BOUNDARY
            || Self::DISPLAY_BEHAVIOR_MOVED_TO_BOUNDARY
            || Self::READER_OR_FILE_BROWSER_BEHAVIOR_CHANGED
            || Self::WRITABLE_STORAGE_BEHAVIOR_ADDED
            || RustmixReadonlyStorageContract::physical_behavior_moved()
            || RustmixStorageReadonlyX4BridgeContract::physical_behavior_moved()
    }

    pub const fn active_runtime_preflight() -> StorageReadonlyBoundaryPreflight {
        StorageReadonlyBoundaryPreflight {
            boundary_marker_present: !Self::BOUNDARY_MARKER.is_empty(),
            facade_marker_present: !RustmixReadonlyStorageContract::CONTRACT_MARKER.is_empty(),
            bridge_marker_present: !RustmixStorageReadonlyX4BridgeContract::BRIDGE_MARKER
                .is_empty(),
            active_backend_is_x4: !Self::ACTIVE_BACKEND_OWNER.is_empty(),
            public_contract_is_facade: !Self::PUBLIC_CONTRACT_LAYER.is_empty(),
            active_implementation_is_bridge: !Self::ACTIVE_IMPLEMENTATION_LAYER.is_empty(),
            canonical_doc_present: !Self::CANONICAL_DOC_SOURCE.is_empty(),
            physical_behavior_moved: Self::physical_behavior_moved(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageReadonlyBoundaryPreflight {
    pub boundary_marker_present: bool,
    pub facade_marker_present: bool,
    pub bridge_marker_present: bool,
    pub active_backend_is_x4: bool,
    pub public_contract_is_facade: bool,
    pub active_implementation_is_bridge: bool,
    pub canonical_doc_present: bool,
    pub physical_behavior_moved: bool,
}

impl StorageReadonlyBoundaryPreflight {
    pub const fn ok(self) -> bool {
        self.boundary_marker_present
            && self.facade_marker_present
            && self.bridge_marker_present
            && self.active_backend_is_x4
            && self.public_contract_is_facade
            && self.active_implementation_is_bridge
            && self.canonical_doc_present
            && !self.physical_behavior_moved
    }
}

/// Canonical read-only storage boundary entrypoint.
pub struct RustmixStorageReadonlyBoundary<B> {
    facade: RustmixReadonlyStorageFacade<X4ReadonlyStorageBridge<B>>,
}

impl<B> RustmixStorageReadonlyBoundary<B> {
    pub const fn new_x4_backed(backend: B) -> Self {
        Self {
            facade: RustmixReadonlyStorageFacade::new(X4ReadonlyStorageBridge::new(backend)),
        }
    }

    pub fn facade(&self) -> &RustmixReadonlyStorageFacade<X4ReadonlyStorageBridge<B>> {
        &self.facade
    }

    pub fn facade_mut(&mut self) -> &mut RustmixReadonlyStorageFacade<X4ReadonlyStorageBridge<B>> {
        &mut self.facade
    }

    pub fn bridge(&self) -> &X4ReadonlyStorageBridge<B> {
        self.facade.backend()
    }

    pub fn bridge_mut(&mut self) -> &mut X4ReadonlyStorageBridge<B> {
        self.facade.backend_mut()
    }

    pub fn into_bridge(self) -> X4ReadonlyStorageBridge<B> {
        self.facade.into_backend()
    }
}

impl<B> RustmixReadonlyStorage for RustmixStorageReadonlyBoundary<B>
where
    B: X4ReadonlyStorageBackend,
{
    type Error = X4ReadonlyStorageBridgeError<B::Error>;

    fn file_exists(&mut self, path: RustmixStoragePathRef<'_>) -> Result<bool, Self::Error> {
        self.facade.file_exists(path)
    }

    fn read_file_start(
        &mut self,
        path: RustmixStoragePathRef<'_>,
        out: &mut [u8],
    ) -> Result<RustmixStorageReadChunk, Self::Error> {
        self.facade.read_file_start(path, out)
    }

    fn read_chunk(
        &mut self,
        path: RustmixStoragePathRef<'_>,
        offset: u64,
        out: &mut [u8],
    ) -> Result<RustmixStorageReadChunk, Self::Error> {
        self.facade.read_chunk(path, offset, out)
    }

    fn list_directory_metadata(
        &mut self,
        path: RustmixStoragePathRef<'_>,
        out: &mut [RustmixDirectoryEntry],
    ) -> Result<usize, Self::Error> {
        self.facade.list_directory_metadata(path, out)
    }

    fn resolve_current_storage_paths(&self) -> RustmixResolvedStoragePaths<'static> {
        self.facade.resolve_current_storage_paths()
    }
}
