use std::path::PathBuf;

mod commands;
mod control;
mod daemon;
mod logs;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod validation_tests;

pub(crate) use commands::{ControlCommand, LockOptions};
pub(crate) use logs::{LogOptions, LogTarget};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonOptions {
    pub config_path: Option<PathBuf>,
    pub log_file_path: Option<PathBuf>,
    pub session_id: Option<String>,
    pub help: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ControlOptions {
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) help: bool,
    pub(crate) command: Option<ControlCommand>,
}
