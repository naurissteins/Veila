mod backdrop;
mod clock;
mod grid;
mod identity;
mod indicators;
mod input;
mod layer;
mod layout;
mod now_playing;
mod outputs;
mod power;
mod weather;

use serde::{Deserialize, Serialize};

use super::RgbColor;

pub use backdrop::{BackdropMode, BackdropShowWhen, BackdropVisualConfig};
pub use clock::{
    ClockAlignment, ClockFormat, ClockStyle, ClockVisualConfig, DateFormat, DateVisualConfig,
};
pub use grid::GridVisualConfig;
pub use identity::{AvatarVisualConfig, UsernameVisualConfig};
pub use indicators::{
    BatteryVisualConfig, CapsLockVisualConfig, EyeVisualConfig, KeyboardVisualConfig,
    PlaceholderVisualConfig, PowerStatusVisualConfig, RevealDisplayMode, RevealVisualConfig,
    StatusDisplayMode, StatusVisualConfig,
};
pub use input::{FontStyle, InputRevealMode, InputVisualConfig};
pub use layer::{LayerKind, LayerVisualConfig};
pub use layout::{HorizontalAlign, PaletteVisualConfig, VerticalAlign, WidgetPositionConfig};
pub use now_playing::{
    NowPlayingArtworkVisualConfig, NowPlayingTextVisualConfig, NowPlayingVisualConfig,
};
pub use outputs::{OutputUiMode, OutputVisualConfig};
pub use power::{PowerButtonVisualConfig, PowerVisualConfig};
pub use weather::{
    WeatherIconVisualConfig, WeatherLocationVisualConfig, WeatherTemperatureVisualConfig,
    WeatherVisualConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualConfig {
    #[serde(default)]
    pub input: InputVisualConfig,
    #[serde(default)]
    pub avatar: Option<AvatarVisualConfig>,
    #[serde(default)]
    pub username: Option<UsernameVisualConfig>,
    #[serde(default)]
    pub clock: Option<ClockVisualConfig>,
    #[serde(default)]
    pub date: Option<DateVisualConfig>,
    #[serde(default)]
    pub placeholder: Option<PlaceholderVisualConfig>,
    #[serde(default)]
    pub reveal: Option<RevealVisualConfig>,
    #[serde(default)]
    pub status: Option<StatusVisualConfig>,
    #[serde(default)]
    pub eye: Option<EyeVisualConfig>,
    #[serde(default)]
    pub caps_lock: Option<CapsLockVisualConfig>,
    #[serde(default)]
    pub keyboard: Option<KeyboardVisualConfig>,
    #[serde(default)]
    pub battery: Option<BatteryVisualConfig>,
    #[serde(default)]
    pub power_status: Option<PowerStatusVisualConfig>,
    #[serde(default)]
    pub power: Option<PowerVisualConfig>,
    #[serde(default)]
    pub grid: Option<GridVisualConfig>,
    #[serde(default)]
    pub weather: Option<WeatherVisualConfig>,
    #[serde(default)]
    pub backdrop: Vec<BackdropVisualConfig>,
    #[serde(default)]
    pub layer: Vec<LayerVisualConfig>,
    #[serde(default)]
    pub now_playing: Option<NowPlayingVisualConfig>,
    #[serde(default)]
    pub outputs: Option<OutputVisualConfig>,
    #[serde(default)]
    pub palette: Option<PaletteVisualConfig>,
}

impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            input: InputVisualConfig::default(),
            avatar: Some(AvatarVisualConfig::default()),
            username: Some(UsernameVisualConfig::default()),
            clock: Some(ClockVisualConfig::default()),
            date: Some(DateVisualConfig::default()),
            placeholder: Some(PlaceholderVisualConfig::default()),
            reveal: Some(RevealVisualConfig::default()),
            status: Some(StatusVisualConfig::default()),
            eye: Some(EyeVisualConfig::default()),
            caps_lock: Some(CapsLockVisualConfig::default()),
            keyboard: Some(KeyboardVisualConfig::default()),
            battery: Some(BatteryVisualConfig::default()),
            power_status: Some(PowerStatusVisualConfig::default()),
            power: Some(PowerVisualConfig::default()),
            grid: Some(GridVisualConfig::default()),
            weather: Some(WeatherVisualConfig::default()),
            backdrop: vec![BackdropVisualConfig {
                name: Some(String::from("now_playing_panel")),
                enabled: Some(true),
                show_when: Some(BackdropShowWhen::NowPlaying),
                mode: Some(BackdropMode::Blur),
                color: Some(RgbColor::rgba(255, 255, 255, 5)),
                blur_strength: Some(12),
                radius: Some(10),
                border_color: Some(RgbColor::rgba(255, 255, 255, 24)),
                border_width: None,
                full_width: None,
                full_height: None,
                inset_top: None,
                inset_bottom: None,
                inset_left: None,
                inset_right: None,
                width: Some(400),
                height: Some(60),
                rotate: Some(0),
                z: Some(0),
                position: WidgetPositionConfig {
                    halign: Some(HorizontalAlign::Right),
                    valign: Some(VerticalAlign::Bottom),
                    x: Some(-40),
                    y: Some(-40),
                    relative_to: None,
                },
            }],
            layer: Vec::new(),
            now_playing: Some(NowPlayingVisualConfig::default()),
            outputs: Some(OutputVisualConfig::default()),
            palette: None,
        }
    }
}

pub(super) const DEFAULT_GEOM_FONT_FAMILY: &str = "Geom";
pub(super) const DEFAULT_INPUT_FONT_FAMILY: &str = "Google Sans Flex";

pub(super) fn default_geom_font_family() -> String {
    String::from(DEFAULT_GEOM_FONT_FAMILY)
}

pub(super) fn default_google_sans_flex_font_family() -> String {
    String::from(DEFAULT_INPUT_FONT_FAMILY)
}

pub(super) const fn default_foreground_color() -> RgbColor {
    RgbColor::rgb(240, 244, 250)
}

pub(super) const fn default_muted_color() -> RgbColor {
    RgbColor::rgb(68, 78, 102)
}

pub(super) const fn default_pending_color() -> RgbColor {
    RgbColor::rgb(236, 236, 236)
}

pub(super) const fn default_rejected_color() -> RgbColor {
    RgbColor::rgb(255, 83, 83)
}
