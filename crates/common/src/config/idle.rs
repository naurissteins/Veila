use serde::{Deserialize, Serialize};

pub const MIN_IDLE_LOCK_AFTER_SECONDS: u64 = 5;
pub const MAX_IDLE_LOCK_AFTER_SECONDS: u64 = 86_400;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdleConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_lock_after_seconds")]
    pub lock_after_seconds: u64,
    #[serde(default = "default_lock_before_sleep")]
    pub lock_before_sleep: bool,
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            lock_after_seconds: default_lock_after_seconds(),
            lock_before_sleep: default_lock_before_sleep(),
        }
    }
}

impl IdleConfig {
    /// Idle timeout clamped to the supported range; `None` when idle locking is disabled.
    pub fn effective_lock_after_seconds(&self) -> Option<u64> {
        self.enabled.then(|| {
            self.lock_after_seconds
                .clamp(MIN_IDLE_LOCK_AFTER_SECONDS, MAX_IDLE_LOCK_AFTER_SECONDS)
        })
    }

    pub fn lock_after_in_range(&self) -> bool {
        (MIN_IDLE_LOCK_AFTER_SECONDS..=MAX_IDLE_LOCK_AFTER_SECONDS)
            .contains(&self.lock_after_seconds)
    }
}

const fn default_lock_after_seconds() -> u64 {
    300
}

const fn default_lock_before_sleep() -> bool {
    true
}
