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
    #[serde(default = "default_lock_show_username")]
    pub show_username: bool,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub user_hint: Option<String>,
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
            show_username: default_lock_show_username(),
            username: None,
            user_hint: Some(String::from("Password")),
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

fn default_lock_log_file_path() -> PathBuf {
    PathBuf::from("~/.local/state/veila/veilad.log")
}

const fn default_auth_backoff_max_seconds() -> u64 {
    12
}

const fn default_lock_show_username() -> bool {
    true
}
