mod clock;
mod identity;
mod indicators;
mod input;
mod layer;
mod layout;
mod now_playing;
mod weather;

use serde::{Deserialize, Serialize};

use super::RgbColor;

pub use clock::{ClockAlignment, ClockFormat, ClockStyle, ClockVisualConfig, DateVisualConfig};
pub use identity::{AvatarVisualConfig, UsernameVisualConfig};
pub use indicators::{
    BatteryVisualConfig, CapsLockVisualConfig, EyeVisualConfig, KeyboardVisualConfig,
    PlaceholderVisualConfig, StatusVisualConfig,
};
pub use input::{FontStyle, InputAlignment, InputVisualConfig, InputVisualEntry};
pub use layer::{
    LayerAlignment, LayerHeight, LayerMode, LayerStyle, LayerVerticalAlignment, LayerVisualConfig,
    LayerWidth,
};
pub use layout::{
    CenterStackOrder, CenterStackStyle, LayoutVisualConfig, PaletteVisualConfig,
};
pub use now_playing::NowPlayingVisualConfig;
pub use weather::{WeatherAlignment, WeatherVisualConfig};

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
    pub input_opacity: Option<u8>,
    #[serde(default)]
    pub input_font_family: Option<String>,
    #[serde(default)]
    pub input_font_weight: Option<u16>,
    #[serde(default)]
    pub input_font_style: Option<FontStyle>,
    #[serde(default)]
    pub input_font_size: Option<u16>,
    #[serde(default)]
    pub input_center_in_layer: Option<bool>,
    #[serde(default = "default_input_border_color")]
    pub input_border: RgbColor,
    #[serde(default)]
    pub input_border_opacity: Option<u8>,
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
    pub avatar_offset_y: Option<i16>,
    #[serde(default)]
    pub avatar_placeholder_padding: Option<u16>,
    #[serde(default)]
    pub avatar_icon_color: Option<RgbColor>,
    #[serde(default)]
    pub avatar_ring_color: Option<RgbColor>,
    #[serde(default)]
    pub avatar_ring_width: Option<u16>,
    #[serde(default)]
    pub avatar_background_opacity: Option<u8>,
    #[serde(default)]
    pub username_color: Option<RgbColor>,
    #[serde(default)]
    pub username_opacity: Option<u8>,
    #[serde(default)]
    pub username_size: Option<u16>,
    #[serde(default)]
    pub username_offset_y: Option<i16>,
    #[serde(default)]
    pub avatar_gap: Option<u16>,
    #[serde(default)]
    pub username_gap: Option<u16>,
    #[serde(default)]
    pub status_gap: Option<u16>,
    #[serde(default)]
    pub clock_gap: Option<u16>,
    #[serde(default)]
    pub auth_stack_offset: Option<i16>,
    #[serde(default)]
    pub header_top_offset: Option<i16>,
    #[serde(default)]
    pub identity_gap: Option<u16>,
    #[serde(default)]
    pub center_stack_order: Option<CenterStackOrder>,
    #[serde(default)]
    pub center_stack_style: Option<CenterStackStyle>,
    #[serde(default)]
    pub clock_font_family: Option<String>,
    #[serde(default)]
    pub clock_font_weight: Option<u16>,
    #[serde(default)]
    pub clock_font_style: Option<FontStyle>,
    #[serde(default)]
    pub clock_style: Option<ClockStyle>,
    #[serde(default)]
    pub clock_center_in_layer: Option<bool>,
    #[serde(default)]
    pub clock_offset_x: Option<i16>,
    #[serde(default)]
    pub clock_offset_y: Option<i16>,
    #[serde(default)]
    pub clock_format: Option<ClockFormat>,
    #[serde(default)]
    pub clock_meridiem_size: Option<u16>,
    #[serde(default)]
    pub clock_meridiem_offset_x: Option<i16>,
    #[serde(default)]
    pub clock_meridiem_offset_y: Option<i16>,
    #[serde(default)]
    pub clock_color: Option<RgbColor>,
    #[serde(default)]
    pub clock_opacity: Option<u8>,
    #[serde(default)]
    pub date_color: Option<RgbColor>,
    #[serde(default)]
    pub date_opacity: Option<u8>,
    #[serde(default)]
    pub clock_size: Option<u16>,
    #[serde(default)]
    pub date_size: Option<u16>,
    #[serde(default)]
    pub placeholder_color: Option<RgbColor>,
    #[serde(default)]
    pub placeholder_opacity: Option<u8>,
    #[serde(default)]
    pub eye_icon_color: Option<RgbColor>,
    #[serde(default)]
    pub eye_icon_opacity: Option<u8>,
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
    pub keyboard_opacity: Option<u8>,
    #[serde(default)]
    pub battery_opacity: Option<u8>,
    #[serde(default)]
    pub keyboard_size: Option<u16>,
    #[serde(default)]
    pub battery_size: Option<u16>,
    #[serde(default)]
    pub keyboard_top_offset: Option<i16>,
    #[serde(default)]
    pub battery_top_offset: Option<i16>,
    #[serde(default)]
    pub keyboard_right_offset: Option<i16>,
    #[serde(default)]
    pub battery_right_offset: Option<i16>,
    #[serde(default)]
    pub battery_gap: Option<u16>,
    #[serde(default)]
    pub weather_size: Option<u16>,
    #[serde(default)]
    pub status_color: Option<RgbColor>,
    #[serde(default)]
    pub status_opacity: Option<u8>,
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
    pub weather: Option<WeatherVisualConfig>,
    #[serde(default)]
    pub layer: Option<LayerVisualConfig>,
    #[serde(default)]
    pub now_playing: Option<NowPlayingVisualConfig>,
    #[serde(default)]
    pub layout: Option<LayoutVisualConfig>,
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
            input_opacity: Some(5),
            input_font_family: Some(default_google_sans_flex_font_family()),
            input_font_weight: Some(400),
            input_font_style: Some(FontStyle::Normal),
            input_font_size: Some(2),
            input_center_in_layer: Some(false),
            input_border: RgbColor::rgb(255, 255, 255),
            input_border_opacity: Some(0),
            input_width: Some(310),
            input_height: Some(54),
            input_radius: 10,
            input_border_width: Some(0),
            avatar_size: Some(192),
            avatar_offset_y: Some(0),
            avatar_placeholder_padding: Some(28),
            avatar_icon_color: Some(RgbColor::rgb(255, 255, 255)),
            avatar_ring_color: Some(RgbColor::rgb(148, 178, 255)),
            avatar_ring_width: Some(0),
            avatar_background_opacity: Some(6),
            username_color: Some(RgbColor::rgb(255, 255, 255)),
            username_opacity: Some(84),
            username_size: Some(4),
            username_offset_y: Some(0),
            avatar_gap: Some(24),
            username_gap: Some(28),
            status_gap: Some(18),
            clock_gap: Some(20),
            auth_stack_offset: Some(0),
            header_top_offset: Some(-12),
            identity_gap: Some(18),
            center_stack_order: Some(CenterStackOrder::HeroAuth),
            center_stack_style: Some(CenterStackStyle::HeroAuth),
            clock_font_family: Some(default_geom_font_family()),
            clock_font_weight: Some(600),
            clock_font_style: Some(FontStyle::Normal),
            clock_style: Some(ClockStyle::Standard),
            clock_center_in_layer: Some(false),
            clock_offset_x: Some(0),
            clock_offset_y: Some(0),
            clock_format: Some(ClockFormat::TwentyFourHour),
            clock_meridiem_size: Some(3),
            clock_meridiem_offset_x: Some(6),
            clock_meridiem_offset_y: Some(7),
            clock_color: Some(RgbColor::rgb(255, 255, 255)),
            clock_opacity: Some(40),
            date_color: Some(RgbColor::rgb(255, 255, 255)),
            date_opacity: Some(50),
            clock_size: Some(14),
            date_size: Some(2),
            placeholder_color: Some(RgbColor::rgb(255, 255, 255)),
            placeholder_opacity: Some(60),
            eye_icon_color: Some(RgbColor::rgb(255, 255, 255)),
            eye_icon_opacity: Some(72),
            keyboard_color: Some(RgbColor::rgb(255, 255, 255)),
            battery_color: Some(RgbColor::rgb(255, 255, 255)),
            battery_background_color: Some(RgbColor::rgba(255, 255, 255, 13)),
            keyboard_background_size: Some(46),
            battery_background_size: Some(46),
            keyboard_opacity: Some(68),
            battery_opacity: Some(68),
            keyboard_size: Some(2),
            battery_size: Some(20),
            keyboard_top_offset: Some(-24),
            battery_top_offset: Some(-24),
            keyboard_right_offset: Some(8),
            battery_right_offset: Some(8),
            battery_gap: Some(8),
            weather_size: Some(2),
            status_color: Some(RgbColor::rgb(255, 224, 160)),
            status_opacity: Some(88),
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
            status: Some(StatusVisualConfig::default()),
            eye: Some(EyeVisualConfig::default()),
            caps_lock: Some(CapsLockVisualConfig::default()),
            keyboard: Some(KeyboardVisualConfig::default()),
            battery: Some(BatteryVisualConfig::default()),
            weather: Some(WeatherVisualConfig::default()),
            layer: Some(LayerVisualConfig::default()),
            now_playing: Some(NowPlayingVisualConfig::default()),
            layout: Some(LayoutVisualConfig::default()),
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
    RgbColor::rgb(255, 194, 92)
}

pub(super) const fn default_rejected_color() -> RgbColor {
    RgbColor::rgb(220, 96, 96)
}
