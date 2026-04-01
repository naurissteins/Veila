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
    CenterStackOrder, CenterStackStyle, ClockAlignment, ClockFormat, ClockStyle, ClockVisualConfig,
    ConfigColor, DateVisualConfig, EyeVisualConfig, FontStyle, GeoCoordinate, InputAlignment,
    InputVisualConfig, InputVisualEntry, KeyboardVisualConfig, LayerAlignment, LayerHeight,
    LayerMode, LayerStyle, LayerVerticalAlignment, LayerVisualConfig, LayerWidth,
    LayoutVisualConfig, LoadedConfig, NowPlayingVisualConfig, PaletteVisualConfig,
    PlaceholderVisualConfig, RgbColor, StatusVisualConfig, UsernameVisualConfig, WeatherAlignment,
    WeatherConfig, WeatherUnit, WeatherVisualConfig, active_theme_source_path, default_config_path,
};
pub use error::{Result, VeilaError};
pub use now_playing::NowPlayingSnapshot;
pub use weather::{WeatherCondition, WeatherSnapshot};
