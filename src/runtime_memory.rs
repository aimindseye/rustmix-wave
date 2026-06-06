//! Runtime stack and heap telemetry for worker-boundary hardening.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimeMemorySnapshot {
    pub main_stack_high_water_bytes: usize,
    pub heap_free_internal_bytes: usize,
    pub heap_largest_internal_block_bytes: usize,
    pub heap_free_psram_bytes: usize,
}

#[cfg(target_os = "espidf")]
impl RuntimeMemorySnapshot {
    #[must_use]
    pub fn capture() -> Self {
        use esp_idf_svc::sys;
        Self {
            // ESP-IDF's FreeRTOS port reports the minimum remaining stack
            // margin for the current task in bytes.
            main_stack_high_water_bytes: unsafe {
                sys::uxTaskGetStackHighWaterMark(core::ptr::null_mut()) as usize
            },
            heap_free_internal_bytes: unsafe {
                sys::heap_caps_get_free_size(sys::MALLOC_CAP_INTERNAL as u32)
            },
            heap_largest_internal_block_bytes: unsafe {
                sys::heap_caps_get_largest_free_block(sys::MALLOC_CAP_INTERNAL as u32)
            },
            heap_free_psram_bytes: unsafe {
                sys::heap_caps_get_free_size(sys::MALLOC_CAP_SPIRAM as u32)
            },
        }
    }
}

#[cfg(not(target_os = "espidf"))]
impl RuntimeMemorySnapshot {
    #[must_use]
    pub fn capture() -> Self {
        Self::default()
    }
}

pub fn log_runtime_memory(boundary: &str) {
    let snapshot = RuntimeMemorySnapshot::capture();
    log::info!(
        "rustmix-wave=runtime-memory boundary={} main-stack-high-water-bytes={} heap-free-internal-bytes={} heap-largest-internal-block-bytes={} heap-free-psram-bytes={}",
        sanitize_marker(boundary),
        snapshot.main_stack_high_water_bytes,
        snapshot.heap_free_internal_bytes,
        snapshot.heap_largest_internal_block_bytes,
        snapshot.heap_free_psram_bytes
    );
}

fn sanitize_marker(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::RuntimeMemorySnapshot;

    #[test]
    fn host_snapshot_is_safe_without_espidf_heap_apis() {
        assert_eq!(
            RuntimeMemorySnapshot::capture(),
            RuntimeMemorySnapshot::default()
        );
    }
}
