use std::path::PathBuf;

mod commands;
mod control;
mod logs;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod validation_tests;

pub(crate) use commands::ControlCommand;
pub(crate) use logs::{LogOptions, LogSource, LogTarget};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ControlOptions {
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) help: bool,
    pub(crate) command: Option<ControlCommand>,
}
