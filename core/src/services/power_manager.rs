pub struct PowerManager {
    idle_timeout_ms: u32,
}

impl Default for PowerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerManager {
    pub fn new() -> Self {
        Self {
            idle_timeout_ms: 300_000,
        }
    }

    pub fn idle_timeout_ms(&self) -> u32 {
        self.idle_timeout_ms
    }
}
