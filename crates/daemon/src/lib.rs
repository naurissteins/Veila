#![forbid(unsafe_code)]

//! Daemon entrypoints for Veila lock orchestration.

mod adapters;
mod app;
mod control;
mod domain;
mod options;

pub use control::{component_name, local_build_info, run, run_control};
pub use options::DaemonOptions;
