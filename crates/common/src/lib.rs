#![forbid(unsafe_code)]

//! Shared types used by the Veila workspace.

pub mod battery;
pub mod config;
pub mod error;
pub mod ipc;
pub mod now_playing;
pub mod weather;

pub use battery::BatterySnapshot;
pub use config::{
    AppConfig, AvatarVisualConfig, BatteryConfig, BatteryVisualConfig, CapsLockVisualConfig,
    ClockFormat, ClockVisualConfig, ConfigColor, DateVisualConfig, EyeVisualConfig, GeoCoordinate,
    InputVisualConfig, InputVisualEntry, KeyboardVisualConfig, LayoutVisualConfig, LoadedConfig,
    NowPlayingVisualConfig, PaletteVisualConfig, PlaceholderVisualConfig, RgbColor,
    StatusVisualConfig, UsernameVisualConfig, WeatherAlignment, WeatherConfig, WeatherUnit,
    WeatherVisualConfig,
};
pub use error::{Result, VeilaError};
pub use now_playing::NowPlayingSnapshot;
pub use weather::{WeatherCondition, WeatherSnapshot};
