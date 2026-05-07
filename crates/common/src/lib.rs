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
    AppConfig, AvatarVisualConfig, BackgroundSlideshowConfig, BackgroundSlideshowMode,
    BackgroundSlideshowOrder, BatteryConfig, BatteryVisualConfig, CapsLockVisualConfig,
    CenterStackStyle, ClockAlignment, ClockFormat, ClockStyle, ClockVisualConfig, ConfigColor,
    DateVisualConfig, EyeVisualConfig, FontStyle, GeoCoordinate, HorizontalAlign, InputRevealMode,
    InputVisualConfig, InputVisualEntry, KeyboardVisualConfig, LayerAlignment, LayerHeight,
    LayerMode, LayerStyle, LayerVerticalAlignment, LayerVisualConfig, LayerWidth,
    LayoutVisualConfig, LoadedConfig, NowPlayingBackgroundConfig, NowPlayingConfig,
    NowPlayingVisualConfig, OutputUiMode, OutputVisualConfig, PaletteVisualConfig,
    PlaceholderVisualConfig, RevealVisualConfig, RgbColor, StatusVisualConfig,
    UsernameVisualConfig, VerticalAlign, WeatherAlignment, WeatherConfig, WeatherUnit,
    WeatherVisualConfig, WidgetPositionConfig, active_include_source_paths, active_theme_name,
    active_theme_source_path, default_config_path,
};
pub use error::{Result, VeilaError};
pub use now_playing::NowPlayingSnapshot;
pub use weather::{WeatherCondition, WeatherSnapshot};
