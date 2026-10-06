use anyhow::{Result, bail};
use veila_common::ipc::LatencyReportMode;

use super::LogOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ControlCommand {
    Version,
    Lock(LockOptions),
    Stop,
    Status,
    Health,
    Doctor,
    CheckConfig,
    Init { theme: Option<String>, force: bool },
    Reload,
    Logs(LogOptions),
    ThemeList,
    ThemeCurrent,
    ThemePrint(String),
    ThemeSet(String),
    ThemeUnset,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct LockOptions {
    pub(crate) wait_ready: bool,
    pub(crate) force_emergency_ui: bool,
    pub(crate) latency_report: LatencyReportMode,
}

impl LockOptions {
    pub(super) fn validate_command(&self, command: Option<&ControlCommand>) -> Result<()> {
        if matches!(command, Some(ControlCommand::Lock(_))) {
            return Ok(());
        }
        if self.wait_ready {
            bail!("--wait-ready can only be used with `veila lock`");
        }
        if self.force_emergency_ui {
            bail!("--force-emergency-ui can only be used with `veila lock`");
        }
        if self.latency_report.is_enabled() {
            bail!("--latency-report can only be used with `veila lock`");
        }
        Ok(())
    }
}
