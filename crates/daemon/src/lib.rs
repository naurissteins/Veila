#![forbid(unsafe_code)]

//! Daemon entrypoints for Veila lock orchestration.

mod adapters;
mod app;
mod control;
mod domain;
mod entry;
mod logging;
mod options;

pub use adapters::process::{
    CURTAIN_PROCESS_NAME, CURTAIN_SUBCOMMAND, DAEMON_PROCESS_NAME, PREWARM_PROCESS_NAME,
    PREWARM_SUBCOMMAND,
};
pub use control::run_control;
pub use entry::{run_daemon, run_prewarm};
pub use logging::daemon_log_file_path;
pub use options::DaemonOptions;
