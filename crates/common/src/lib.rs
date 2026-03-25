#![forbid(unsafe_code)]

//! Shared types used by the Veila workspace.

pub mod config;
pub mod error;
pub mod ipc;

pub use config::{AppConfig, ConfigColor, LoadedConfig, RgbColor};
pub use error::{Result, VeilaError};
