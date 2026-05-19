use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockConfig {
    #[serde(default = "default_lock_acquire_timeout_seconds")]
    pub acquire_timeout_seconds: u64,
    #[serde(default = "default_lock_auto_reload_config")]
    pub auto_reload_config: bool,
    #[serde(default = "default_lock_auto_reload_debounce_ms")]
    pub auto_reload_debounce_ms: u64,
    #[serde(default = "default_lock_log_to_file")]
    pub log_to_file: bool,
    #[serde(default = "default_lock_log_file_path")]
    pub log_file_path: PathBuf,
    #[serde(default = "default_auth_backoff_base_ms")]
    pub auth_backoff_base_ms: u64,
    #[serde(default = "default_auth_backoff_max_seconds")]
    pub auth_backoff_max_seconds: u64,
    #[serde(default)]
    pub hide_cursor: bool,
    #[serde(default = "default_allow_empty_password")]
    pub allow_empty_password: bool,
    #[serde(default)]
    pub screen_off_seconds: Option<u64>,
    #[serde(default)]
    pub power_off_secondary_outputs: bool,
    #[serde(default)]
    pub suspend_seconds: Option<u64>,
    #[serde(default)]
    pub suspend_only_on_battery: bool,
    #[serde(default)]
    pub skip_suspend_while_media_playing: bool,
    #[serde(default)]
    pub avatar_path: Option<PathBuf>,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            acquire_timeout_seconds: default_lock_acquire_timeout_seconds(),
            auto_reload_config: default_lock_auto_reload_config(),
            auto_reload_debounce_ms: default_lock_auto_reload_debounce_ms(),
            log_to_file: default_lock_log_to_file(),
            log_file_path: default_lock_log_file_path(),
            auth_backoff_base_ms: default_auth_backoff_base_ms(),
            auth_backoff_max_seconds: default_auth_backoff_max_seconds(),
            hide_cursor: false,
            allow_empty_password: default_allow_empty_password(),
            screen_off_seconds: None,
            power_off_secondary_outputs: false,
            suspend_seconds: None,
            suspend_only_on_battery: false,
            skip_suspend_while_media_playing: false,
            avatar_path: None,
        }
    }
}

const fn default_lock_acquire_timeout_seconds() -> u64 {
    5
}

const fn default_auth_backoff_base_ms() -> u64 {
    750
}

const fn default_lock_auto_reload_config() -> bool {
    true
}

const fn default_lock_auto_reload_debounce_ms() -> u64 {
    250
}

const fn default_lock_log_to_file() -> bool {
    false
}

const fn default_allow_empty_password() -> bool {
    true
}

fn default_lock_log_file_path() -> PathBuf {
    PathBuf::from("~/.local/state/veila/veilad.log")
}

const fn default_auth_backoff_max_seconds() -> u64 {
    12
}
