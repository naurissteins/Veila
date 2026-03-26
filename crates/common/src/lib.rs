#![forbid(unsafe_code)]

//! Shared types used by the Veila workspace.

pub mod config;
pub mod error;
pub mod ipc;

pub use config::{
    AppConfig, AvatarVisualConfig, ClockVisualConfig, ConfigColor, DateVisualConfig,
    EyeVisualConfig, InputVisualConfig, InputVisualEntry, LayoutVisualConfig, LoadedConfig,
    PaletteVisualConfig, PlaceholderVisualConfig, RgbColor, StatusVisualConfig,
    UsernameVisualConfig,
};
pub use error::{Result, VeilaError};
