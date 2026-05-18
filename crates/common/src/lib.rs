#![forbid(unsafe_code)]

//! Shared types used by the Veila workspace.

pub mod battery;
pub mod config;
pub mod error;
pub mod ipc;
pub mod now_playing;
pub mod power;
pub mod weather;

pub use battery::BatterySnapshot;
pub use config::{
    AppConfig, AvatarVisualConfig, BackdropMode, BackdropShowWhen, BackdropVisualConfig,
    BackgroundSlideshowConfig, BackgroundSlideshowMode, BackgroundSlideshowOrder, BatteryConfig,
    BatteryVisualConfig, CapsLockVisualConfig, ClockAlignment, ClockFormat, ClockStyle,
    ClockVisualConfig, ConfigColor, ConfigValidationIssue, ConfigValidationReport,
    ConfigValidationSource, ConfigValidationSourceKind, DateFormat, DateVisualConfig,
    EyeVisualConfig, FontStyle, GeoCoordinate, GridVisualConfig, HorizontalAlign, InputRevealMode,
    InputVisualConfig, InputVisualEntry, KeyboardVisualConfig, LayerKind, LayerVisualConfig,
    LoadedConfig, NowPlayingArtworkVisualConfig, NowPlayingConfig, NowPlayingTextVisualConfig,
    NowPlayingVisualConfig, OutputUiMode, OutputVisualConfig, PaletteVisualConfig,
    PlaceholderVisualConfig, PowerButtonVisualConfig, PowerStatusVisualConfig, PowerVisualConfig,
    RevealDisplayMode, RevealVisualConfig, RgbColor, StatusDisplayMode, StatusVisualConfig,
    UsernameVisualConfig, VerticalAlign, WeatherConfig, WeatherIconVisualConfig,
    WeatherLocationVisualConfig, WeatherTemperatureVisualConfig, WeatherUnit, WeatherVisualConfig,
    WidgetPositionConfig, active_include_source_paths, active_theme_name, active_theme_source_path,
    default_config_path,
};
pub use error::{Result, VeilaError};
pub use now_playing::NowPlayingSnapshot;
pub use power::PowerAction;
pub use weather::{WeatherCondition, WeatherSnapshot};
