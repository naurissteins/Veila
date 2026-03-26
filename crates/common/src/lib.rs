#![forbid(unsafe_code)]

//! Shared types used by the Veila workspace.

pub mod config;
pub mod error;
pub mod ipc;
pub mod weather;

pub use config::{
    AppConfig, AvatarVisualConfig, ClockVisualConfig, ConfigColor, DateVisualConfig,
    EyeVisualConfig, GeoCoordinate, InputVisualConfig, InputVisualEntry, KeyboardVisualConfig,
    LayoutVisualConfig, LoadedConfig, PaletteVisualConfig, PlaceholderVisualConfig, RgbColor,
    StatusVisualConfig, UsernameVisualConfig, WeatherAlignment, WeatherConfig, WeatherVisualConfig,
};
pub use error::{Result, VeilaError};
pub use weather::{WeatherCondition, WeatherSnapshot};
