use std::path::PathBuf;

use veila_common::ipc::LatencyReportMode;

mod control;
mod daemon;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LogTarget {
    #[default]
    LockService,
    All,
    Daemon,
    Curtain,
    Ui,
    Idle,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonOptions {
    pub config_path: Option<PathBuf>,
    pub log_file_path: Option<PathBuf>,
    pub session_id: Option<String>,
    pub help: bool,
    pub current_theme: bool,
    pub print_theme: Option<String>,
    pub set_theme: Option<String>,
    pub unset_theme: bool,
    pub lock_now: bool,
    pub force_emergency_ui: bool,
    pub latency_report: LatencyReportMode,
    pub wait_ready: bool,
    pub stop: bool,
    pub list_themes: bool,
    pub status: bool,
    pub health: bool,
    pub doctor: bool,
    pub check_config: bool,
    pub init_config: bool,
    pub init_force: bool,
    pub init_theme: Option<String>,
    pub version: bool,
    pub reload_config: bool,
    pub idle: bool,
    pub idle_lock_after_seconds: Option<u64>,
    pub idle_lock_before_sleep: bool,
    pub logs: bool,
    pub logs_file: bool,
    pub logs_follow: bool,
    pub logs_since: Option<String>,
    pub logs_lines: Option<u32>,
    pub logs_target: LogTarget,
}
