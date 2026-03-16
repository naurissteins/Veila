#![forbid(unsafe_code)]

//! Shared types used by the Kwylock workspace.

pub mod config;
pub mod error;
pub mod ipc;

pub use config::AppConfig;
pub use error::{KwylockError, Result};
