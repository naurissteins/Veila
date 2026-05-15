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
pub use input::{FontStyle, InputRevealMode, InputVisualConfig, InputVisualEntry};
pub use layer::{LayerKind, LayerVisualConfig};
pub use layout::{HorizontalAlign, PaletteVisualConfig, VerticalAlign, WidgetPositionConfig};
pub use now_playing::{
    NowPlayingArtworkVisualConfig, NowPlayingTextVisualConfig, NowPlayingVisualConfig,
};
pub use outputs::{OutputUiMode, OutputVisualConfig};
pub use weather::{
    WeatherIconVisualConfig, WeatherLocationVisualConfig, WeatherTemperatureVisualConfig,
    WeatherVisualConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualConfig {
    #[serde(default = "default_panel_color")]
    pub panel: RgbColor,
    #[serde(default)]
    pub avatar_background_color: Option<RgbColor>,
    #[serde(default = "default_panel_border_color")]
    pub panel_border: RgbColor,
    #[serde(default)]
    pub input: InputVisualEntry,
    #[serde(default)]
    pub input_font_family: Option<String>,
    #[serde(default)]
    pub input_font_weight: Option<u16>,
    #[serde(default)]
    pub input_font_style: Option<FontStyle>,
    #[serde(default)]
    pub input_font_size: Option<u16>,
    #[serde(default = "default_input_border_color")]
    pub input_border: RgbColor,
    #[serde(default)]
    pub input_width: Option<u16>,
    #[serde(default)]
    pub input_height: Option<u16>,
    #[serde(default = "default_input_radius")]
    pub input_radius: u16,
    #[serde(default)]
    pub input_border_width: Option<u16>,
    #[serde(default)]
    pub avatar_size: Option<u16>,
    #[serde(default)]
    pub avatar_radius: Option<u16>,
    #[serde(default)]
    pub avatar_placeholder_padding: Option<u16>,
    #[serde(default)]
    pub avatar_icon_color: Option<RgbColor>,
    #[serde(default)]
    pub avatar_ring_color: Option<RgbColor>,
    #[serde(default)]
    pub avatar_ring_width: Option<u16>,
    #[serde(default)]
    pub username_color: Option<RgbColor>,
    #[serde(default)]
    pub username_font_size: Option<u16>,
    #[serde(default)]
    pub clock_font_family: Option<String>,
    #[serde(default)]
    pub clock_font_weight: Option<u16>,
    #[serde(default)]
    pub clock_font_style: Option<FontStyle>,
    #[serde(default)]
    pub clock_style: Option<ClockStyle>,
    #[serde(default)]
    pub clock_format: Option<ClockFormat>,
    #[serde(default)]
    pub clock_meridiem_font_size: Option<u16>,
    #[serde(default)]
    pub clock_meridiem_x: Option<i16>,
    #[serde(default)]
    pub clock_meridiem_y: Option<i16>,
    #[serde(default)]
    pub clock_color: Option<RgbColor>,
    #[serde(default)]
    pub date_color: Option<RgbColor>,
    #[serde(default)]
    pub clock_font_size: Option<u16>,
    #[serde(default)]
    pub date_font_size: Option<u16>,
    #[serde(default)]
    pub placeholder_color: Option<RgbColor>,
    #[serde(default)]
    pub eye_icon_color: Option<RgbColor>,
    #[serde(default)]
    pub keyboard_color: Option<RgbColor>,
    #[serde(default)]
    pub battery_color: Option<RgbColor>,
    #[serde(default)]
    pub battery_background_color: Option<RgbColor>,
    #[serde(default)]
    pub keyboard_background_size: Option<u16>,
    #[serde(default)]
    pub battery_background_size: Option<u16>,
    #[serde(default)]
    pub keyboard_size: Option<u16>,
    #[serde(default)]
    pub battery_size: Option<u16>,
    #[serde(default)]
    pub status_color: Option<RgbColor>,
    #[serde(default)]
    pub input_mask_color: Option<RgbColor>,
    #[serde(default = "default_foreground_color")]
    pub foreground: RgbColor,
    #[serde(default = "default_muted_color")]
    pub muted: RgbColor,
    #[serde(default = "default_pending_color")]
    pub pending: RgbColor,
    #[serde(default = "default_rejected_color")]
    pub rejected: RgbColor,
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
            panel: default_panel_color(),
            avatar_background_color: None,
            panel_border: default_panel_border_color(),
            input: InputVisualEntry::Section(InputVisualConfig::default()),
            input_font_family: Some(default_google_sans_flex_font_family()),
            input_font_weight: Some(400),
            input_font_style: Some(FontStyle::Normal),
            input_font_size: Some(16),
            input_border: RgbColor::rgba(255, 255, 255, 0),
            input_width: Some(310),
            input_height: Some(54),
            input_radius: 10,
            input_border_width: Some(0),
            avatar_size: Some(150),
            avatar_radius: None,
            avatar_placeholder_padding: Some(28),
            avatar_icon_color: Some(RgbColor::rgb(255, 255, 255)),
            avatar_ring_color: Some(RgbColor::rgb(148, 178, 255)),
            avatar_ring_width: Some(0),
            username_color: Some(RgbColor::rgba(255, 255, 255, 214)),
            username_font_size: Some(28),
            clock_font_family: Some(default_geom_font_family()),
            clock_font_weight: Some(600),
            clock_font_style: Some(FontStyle::Normal),
            clock_style: Some(ClockStyle::Standard),
            clock_format: Some(ClockFormat::TwentyFourHour),
            clock_meridiem_font_size: Some(22),
            clock_meridiem_x: Some(6),
            clock_meridiem_y: Some(7),
            clock_color: Some(RgbColor::rgba(255, 255, 255, 102)),
            date_color: Some(RgbColor::rgba(255, 255, 255, 102)),
            clock_font_size: Some(88),
            date_font_size: Some(18),
            placeholder_color: Some(RgbColor::rgba(255, 255, 255, 230)),
            eye_icon_color: Some(RgbColor::rgba(255, 255, 255, 184)),
            keyboard_color: Some(RgbColor::rgba(255, 255, 255, 173)),
            battery_color: Some(RgbColor::rgba(255, 255, 255, 173)),
            battery_background_color: Some(RgbColor::rgba(255, 255, 255, 10)),
            keyboard_background_size: Some(46),
            battery_background_size: Some(46),
            keyboard_size: Some(16),
            battery_size: Some(20),
            status_color: None,
            input_mask_color: Some(RgbColor::rgb(255, 255, 255)),
            foreground: default_foreground_color(),
            muted: default_muted_color(),
            pending: default_pending_color(),
            rejected: default_rejected_color(),
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

pub(super) const fn default_panel_color() -> RgbColor {
    RgbColor::rgb(22, 28, 38)
}

pub(super) const fn default_panel_border_color() -> RgbColor {
    RgbColor::rgb(74, 86, 110)
}

pub(super) const fn default_input_border_color() -> RgbColor {
    RgbColor::rgb(92, 108, 146)
}

pub(super) const fn default_input_radius() -> u16 {
    32
}

pub(super) fn default_geom_font_family() -> String {
    String::from("Geom")
}

pub(super) fn default_google_sans_flex_font_family() -> String {
    String::from("Google Sans Flex")
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
