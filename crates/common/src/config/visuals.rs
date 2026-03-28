use serde::{Deserialize, Serialize};

use super::RgbColor;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum InputVisualEntry {
    Color(RgbColor),
    Section(InputVisualConfig),
}

impl Default for InputVisualEntry {
    fn default() -> Self {
        Self::Color(default_input_color())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputVisualConfig {
    #[serde(default)]
    pub alignment: Option<InputAlignment>,
    #[serde(default)]
    pub horizontal_padding: Option<u16>,
    #[serde(default)]
    pub vertical_padding: Option<u16>,
    #[serde(default)]
    pub offset_x: Option<i16>,
    #[serde(default)]
    pub offset_y: Option<i16>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub font_size: Option<u16>,
    #[serde(default)]
    pub background_color: Option<RgbColor>,
    #[serde(default)]
    pub background_opacity: Option<u8>,
    #[serde(default)]
    pub border_color: Option<RgbColor>,
    #[serde(default)]
    pub border_opacity: Option<u8>,
    #[serde(default)]
    pub width: Option<u16>,
    #[serde(default)]
    pub height: Option<u16>,
    #[serde(default)]
    pub radius: Option<u16>,
    #[serde(default)]
    pub border_width: Option<u16>,
    #[serde(default)]
    pub mask_color: Option<RgbColor>,
}

impl Default for InputVisualConfig {
    fn default() -> Self {
        Self {
            alignment: Some(InputAlignment::CenterCenter),
            horizontal_padding: None,
            vertical_padding: None,
            offset_x: Some(0),
            offset_y: Some(0),
            font_family: Some(default_google_sans_flex_font_family()),
            font_weight: Some(400),
            font_size: Some(2),
            background_color: Some(RgbColor::rgb(255, 255, 255)),
            background_opacity: Some(5),
            border_color: Some(RgbColor::rgb(255, 255, 255)),
            border_opacity: Some(0),
            width: Some(310),
            height: Some(54),
            radius: Some(10),
            border_width: Some(0),
            mask_color: Some(RgbColor::rgb(255, 255, 255)),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum InputAlignment {
    #[default]
    #[serde(rename = "center-center")]
    CenterCenter,
    #[serde(rename = "center-right")]
    CenterRight,
    #[serde(rename = "center-left")]
    CenterLeft,
    #[serde(rename = "top-center")]
    TopCenter,
    #[serde(rename = "top-right")]
    TopRight,
    #[serde(rename = "top-left")]
    TopLeft,
    #[serde(rename = "bottom-center")]
    BottomCenter,
    #[serde(rename = "bottom-right")]
    BottomRight,
    #[serde(rename = "bottom-left")]
    BottomLeft,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvatarVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub gap: Option<u16>,
    #[serde(default)]
    pub background_color: Option<RgbColor>,
    #[serde(default)]
    pub background_opacity: Option<u8>,
    #[serde(default)]
    pub placeholder_padding: Option<u16>,
    #[serde(default)]
    pub ring_color: Option<RgbColor>,
    #[serde(default)]
    pub ring_width: Option<u16>,
    #[serde(default)]
    pub icon_color: Option<RgbColor>,
}

impl Default for AvatarVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            size: Some(192),
            gap: Some(24),
            background_color: Some(RgbColor::rgb(255, 255, 255)),
            background_opacity: Some(6),
            placeholder_padding: Some(28),
            ring_color: Some(RgbColor::rgb(148, 178, 255)),
            ring_width: Some(0),
            icon_color: Some(RgbColor::rgb(255, 255, 255)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsernameVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub gap: Option<u16>,
}

impl Default for UsernameVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            font_family: Some(default_google_sans_flex_font_family()),
            font_weight: Some(400),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(84),
            size: Some(4),
            gap: Some(28),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClockVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub format: Option<ClockFormat>,
    #[serde(default)]
    pub meridiem_size: Option<u16>,
    #[serde(default)]
    pub meridiem_offset_x: Option<i16>,
    #[serde(default)]
    pub meridiem_offset_y: Option<i16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub gap: Option<u16>,
}

impl Default for ClockVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            font_family: Some(default_geom_font_family()),
            font_weight: Some(600),
            format: Some(ClockFormat::TwentyFourHour),
            meridiem_size: Some(3),
            meridiem_offset_x: Some(6),
            meridiem_offset_y: Some(7),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(40),
            size: Some(14),
            gap: Some(20),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClockFormat {
    #[default]
    #[serde(rename = "24h")]
    TwentyFourHour,
    #[serde(rename = "12h")]
    TwelveHour,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DateVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(default)]
    pub size: Option<u16>,
}

impl Default for DateVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            font_family: Some(default_geom_font_family()),
            font_weight: Some(600),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(50),
            size: Some(2),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaceholderVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
}

impl Default for PlaceholderVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(60),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(default)]
    pub gap: Option<u16>,
}

impl Default for StatusVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            color: Some(RgbColor::rgb(255, 224, 160)),
            opacity: Some(88),
            gap: Some(18),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EyeVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
}

impl Default for EyeVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(72),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapsLockVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
}

impl Default for CapsLockVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyboardVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub background_color: Option<RgbColor>,
    #[serde(default)]
    pub background_size: Option<u16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub top_offset: Option<i16>,
    #[serde(default)]
    pub right_offset: Option<i16>,
}

impl Default for KeyboardVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            background_color: Some(RgbColor::rgba(255, 255, 255, 13)),
            background_size: Some(46),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(68),
            size: Some(2),
            top_offset: Some(-24),
            right_offset: Some(8),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatteryVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub background_color: Option<RgbColor>,
    #[serde(default)]
    pub background_size: Option<u16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub top_offset: Option<i16>,
    #[serde(default)]
    pub right_offset: Option<i16>,
    #[serde(default)]
    pub gap: Option<u16>,
}

impl Default for BatteryVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            background_color: Some(RgbColor::rgba(255, 255, 255, 13)),
            background_size: Some(46),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(68),
            size: Some(20),
            top_offset: Some(-24),
            right_offset: Some(8),
            gap: Some(8),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WeatherAlignment {
    #[default]
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeatherVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(default)]
    pub icon_opacity: Option<u8>,
    #[serde(default)]
    pub temperature_opacity: Option<u8>,
    #[serde(default)]
    pub location_opacity: Option<u8>,
    #[serde(default)]
    pub temperature_color: Option<RgbColor>,
    #[serde(default)]
    pub location_color: Option<RgbColor>,
    #[serde(default)]
    pub temperature_font_family: Option<String>,
    #[serde(default)]
    pub temperature_font_weight: Option<u16>,
    #[serde(default)]
    pub temperature_letter_spacing: Option<u16>,
    #[serde(default)]
    pub location_font_family: Option<String>,
    #[serde(default)]
    pub location_font_weight: Option<u16>,
    #[serde(default)]
    pub temperature_size: Option<u16>,
    #[serde(default)]
    pub location_size: Option<u16>,
    #[serde(default)]
    pub icon_size: Option<u16>,
    #[serde(default)]
    pub icon_gap: Option<u16>,
    #[serde(default)]
    pub location_gap: Option<u16>,
    #[serde(default)]
    pub left_offset: Option<i16>,
    #[serde(default)]
    pub bottom_offset: Option<i16>,
    #[serde(default)]
    pub left_padding: Option<u16>,
    #[serde(default)]
    pub horizontal_padding: Option<u16>,
    #[serde(default)]
    pub bottom_padding: Option<u16>,
    #[serde(default)]
    pub alignment: Option<WeatherAlignment>,
}

impl Default for WeatherVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            size: Some(2),
            opacity: Some(50),
            icon_opacity: None,
            temperature_opacity: None,
            location_opacity: None,
            temperature_color: Some(RgbColor::rgb(255, 255, 255)),
            location_color: Some(RgbColor::rgb(214, 227, 255)),
            temperature_font_family: Some(default_geom_font_family()),
            temperature_font_weight: Some(600),
            temperature_letter_spacing: Some(0),
            location_font_family: Some(default_google_sans_flex_font_family()),
            location_font_weight: Some(400),
            temperature_size: Some(6),
            location_size: Some(3),
            icon_size: Some(40),
            icon_gap: Some(1),
            location_gap: Some(1),
            left_offset: Some(12),
            bottom_offset: Some(-6),
            left_padding: Some(48),
            horizontal_padding: None,
            bottom_padding: Some(48),
            alignment: Some(WeatherAlignment::Left),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NowPlayingVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub fade_duration_ms: Option<u16>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(default)]
    pub title_opacity: Option<u8>,
    #[serde(default)]
    pub artist_opacity: Option<u8>,
    #[serde(default)]
    pub artwork_opacity: Option<u8>,
    #[serde(default)]
    pub title_color: Option<RgbColor>,
    #[serde(default)]
    pub artist_color: Option<RgbColor>,
    #[serde(default)]
    pub title_font_family: Option<String>,
    #[serde(default)]
    pub artist_font_family: Option<String>,
    #[serde(default)]
    pub title_font_weight: Option<u16>,
    #[serde(default)]
    pub artist_font_weight: Option<u16>,
    #[serde(default)]
    pub title_size: Option<u16>,
    #[serde(default)]
    pub artist_size: Option<u16>,
    #[serde(default)]
    pub width: Option<u16>,
    #[serde(default)]
    pub content_gap: Option<u16>,
    #[serde(default)]
    pub text_gap: Option<u16>,
    #[serde(default)]
    pub artwork_size: Option<u16>,
    #[serde(default)]
    pub artwork_radius: Option<u16>,
    #[serde(default)]
    pub right_padding: Option<u16>,
    #[serde(default)]
    pub bottom_padding: Option<u16>,
    #[serde(default)]
    pub right_offset: Option<i16>,
    #[serde(default)]
    pub bottom_offset: Option<i16>,
}

impl Default for NowPlayingVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            fade_duration_ms: Some(320),
            opacity: Some(72),
            title_opacity: Some(74),
            artist_opacity: Some(54),
            artwork_opacity: Some(90),
            title_color: Some(RgbColor::rgb(255, 255, 255)),
            artist_color: Some(RgbColor::rgb(255, 255, 255)),
            title_font_family: Some(default_google_sans_flex_font_family()),
            artist_font_family: Some(default_google_sans_flex_font_family()),
            title_font_weight: Some(400),
            artist_font_weight: Some(400),
            title_size: Some(2),
            artist_size: Some(2),
            width: Some(380),
            content_gap: Some(18),
            text_gap: Some(10),
            artwork_size: Some(44),
            artwork_radius: Some(8),
            right_padding: Some(52),
            bottom_padding: Some(56),
            right_offset: Some(0),
            bottom_offset: Some(0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayoutVisualConfig {
    #[serde(default)]
    pub auth_stack_offset: Option<i16>,
    #[serde(default)]
    pub header_top_offset: Option<i16>,
}

impl Default for LayoutVisualConfig {
    fn default() -> Self {
        Self {
            auth_stack_offset: Some(0),
            header_top_offset: Some(-12),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaletteVisualConfig {
    #[serde(default)]
    pub foreground: Option<RgbColor>,
    #[serde(default)]
    pub muted: Option<RgbColor>,
    #[serde(default)]
    pub pending: Option<RgbColor>,
    #[serde(default)]
    pub rejected: Option<RgbColor>,
}

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
    pub input_font_size: Option<u16>,
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
    pub clock_font_family: Option<String>,
    #[serde(default)]
    pub clock_font_weight: Option<u16>,
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
            input_font_size: Some(2),
            input_border: RgbColor::rgb(255, 255, 255),
            input_border_opacity: Some(0),
            input_width: Some(310),
            input_height: Some(54),
            input_radius: 10,
            input_border_width: Some(0),
            avatar_size: Some(192),
            avatar_placeholder_padding: Some(28),
            avatar_icon_color: Some(RgbColor::rgb(255, 255, 255)),
            avatar_ring_color: Some(RgbColor::rgb(148, 178, 255)),
            avatar_ring_width: Some(0),
            avatar_background_opacity: Some(6),
            username_color: Some(RgbColor::rgb(255, 255, 255)),
            username_opacity: Some(84),
            username_size: Some(4),
            avatar_gap: Some(24),
            username_gap: Some(28),
            status_gap: Some(18),
            clock_gap: Some(20),
            auth_stack_offset: Some(0),
            header_top_offset: Some(-12),
            clock_font_family: Some(default_geom_font_family()),
            clock_font_weight: Some(600),
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
            now_playing: Some(NowPlayingVisualConfig::default()),
            layout: Some(LayoutVisualConfig::default()),
            palette: None,
        }
    }
}

impl VisualConfig {
    pub fn avatar_enabled(&self) -> bool {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.enabled)
            .unwrap_or(true)
    }

    pub fn avatar_background_color(&self) -> Option<RgbColor> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.background_color)
            .or(self.avatar_background_color)
    }

    pub fn input_background_color(&self) -> RgbColor {
        match &self.input {
            InputVisualEntry::Color(color) => *color,
            InputVisualEntry::Section(config) => {
                config.background_color.unwrap_or_else(default_input_color)
            }
        }
    }

    pub fn input_background_opacity(&self) -> Option<u8> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_opacity,
            InputVisualEntry::Section(config) => config.background_opacity.or(self.input_opacity),
        }
    }

    pub fn input_border_color(&self) -> RgbColor {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_border,
            InputVisualEntry::Section(config) => config.border_color.unwrap_or(self.input_border),
        }
    }

    pub fn input_border_opacity(&self) -> Option<u8> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_border_opacity,
            InputVisualEntry::Section(config) => {
                config.border_opacity.or(self.input_border_opacity)
            }
        }
    }

    pub fn input_width(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_width,
            InputVisualEntry::Section(config) => config.width.or(self.input_width),
        }
    }

    pub fn input_font_family(&self) -> Option<&str> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_font_family.as_deref(),
            InputVisualEntry::Section(config) => config
                .font_family
                .as_deref()
                .or(self.input_font_family.as_deref()),
        }
    }

    pub fn input_alignment(&self) -> InputAlignment {
        match &self.input {
            InputVisualEntry::Color(_) => InputAlignment::default(),
            InputVisualEntry::Section(config) => config.alignment.unwrap_or_default(),
        }
    }

    pub fn input_horizontal_padding(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => None,
            InputVisualEntry::Section(config) => config.horizontal_padding,
        }
    }

    pub fn input_vertical_padding(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => None,
            InputVisualEntry::Section(config) => config.vertical_padding,
        }
    }

    pub fn input_offset_x(&self) -> Option<i16> {
        match &self.input {
            InputVisualEntry::Color(_) => None,
            InputVisualEntry::Section(config) => config.offset_x,
        }
    }

    pub fn input_offset_y(&self) -> Option<i16> {
        match &self.input {
            InputVisualEntry::Color(_) => None,
            InputVisualEntry::Section(config) => config.offset_y,
        }
    }

    pub fn input_font_weight(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_font_weight,
            InputVisualEntry::Section(config) => config.font_weight.or(self.input_font_weight),
        }
    }

    pub fn input_font_size(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_font_size,
            InputVisualEntry::Section(config) => config.font_size.or(self.input_font_size),
        }
    }

    pub fn input_height(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_height,
            InputVisualEntry::Section(config) => config.height.or(self.input_height),
        }
    }

    pub fn input_radius(&self) -> u16 {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_radius,
            InputVisualEntry::Section(config) => config.radius.unwrap_or(self.input_radius),
        }
    }

    pub fn input_border_width(&self) -> Option<u16> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_border_width,
            InputVisualEntry::Section(config) => config.border_width.or(self.input_border_width),
        }
    }

    pub fn input_mask_color(&self) -> Option<RgbColor> {
        match &self.input {
            InputVisualEntry::Color(_) => self.input_mask_color,
            InputVisualEntry::Section(config) => config.mask_color.or(self.input_mask_color),
        }
    }

    pub fn avatar_size(&self) -> Option<u16> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.size)
            .or(self.avatar_size)
    }

    pub fn avatar_gap(&self) -> Option<u16> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.gap)
            .or(self.avatar_gap)
    }

    pub fn avatar_background_opacity(&self) -> Option<u8> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.background_opacity)
            .or(self.avatar_background_opacity)
    }

    pub fn avatar_placeholder_padding(&self) -> Option<u16> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.placeholder_padding)
            .or(self.avatar_placeholder_padding)
    }

    pub fn avatar_ring_color(&self) -> Option<RgbColor> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.ring_color)
            .or(self.avatar_ring_color)
    }

    pub fn avatar_ring_width(&self) -> Option<u16> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.ring_width)
            .or(self.avatar_ring_width)
    }

    pub fn avatar_icon_color(&self) -> Option<RgbColor> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.icon_color)
            .or(self.avatar_icon_color)
    }

    pub fn username_color(&self) -> Option<RgbColor> {
        self.username
            .as_ref()
            .and_then(|username| username.color)
            .or(self.username_color)
    }

    pub fn username_enabled(&self) -> bool {
        self.username
            .as_ref()
            .and_then(|username| username.enabled)
            .unwrap_or(true)
    }

    pub fn username_font_family(&self) -> Option<&str> {
        self.username
            .as_ref()
            .and_then(|username| username.font_family.as_deref())
    }

    pub fn username_font_weight(&self) -> Option<u16> {
        self.username
            .as_ref()
            .and_then(|username| username.font_weight)
    }

    pub fn username_opacity(&self) -> Option<u8> {
        self.username
            .as_ref()
            .and_then(|username| username.opacity)
            .or(self.username_opacity)
    }

    pub fn username_size(&self) -> Option<u16> {
        self.username
            .as_ref()
            .and_then(|username| username.size)
            .or(self.username_size)
    }

    pub fn username_gap(&self) -> Option<u16> {
        self.username
            .as_ref()
            .and_then(|username| username.gap)
            .or(self.username_gap)
    }

    pub fn status_gap(&self) -> Option<u16> {
        self.status
            .as_ref()
            .and_then(|status| status.gap)
            .or(self.status_gap)
    }

    pub fn clock_gap(&self) -> Option<u16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.gap)
            .or(self.clock_gap)
    }

    pub fn auth_stack_offset(&self) -> Option<i16> {
        self.layout
            .as_ref()
            .and_then(|layout| layout.auth_stack_offset)
            .or(self.auth_stack_offset)
    }

    pub fn header_top_offset(&self) -> Option<i16> {
        self.layout
            .as_ref()
            .and_then(|layout| layout.header_top_offset)
            .or(self.header_top_offset)
    }

    pub fn clock_font_family(&self) -> Option<&str> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.font_family.as_deref())
            .or(self.clock_font_family.as_deref())
    }

    pub fn clock_enabled(&self) -> bool {
        self.clock
            .as_ref()
            .and_then(|clock| clock.enabled)
            .unwrap_or(true)
    }

    pub fn clock_font_weight(&self) -> Option<u16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.font_weight)
            .or(self.clock_font_weight)
    }

    pub fn clock_format(&self) -> ClockFormat {
        self.clock
            .as_ref()
            .and_then(|clock| clock.format)
            .or(self.clock_format)
            .unwrap_or_default()
    }

    pub fn clock_meridiem_size(&self) -> Option<u16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.meridiem_size)
            .or(self.clock_meridiem_size)
    }

    pub fn clock_meridiem_offset_x(&self) -> Option<i16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.meridiem_offset_x)
            .or(self.clock_meridiem_offset_x)
    }

    pub fn clock_meridiem_offset_y(&self) -> Option<i16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.meridiem_offset_y)
            .or(self.clock_meridiem_offset_y)
    }

    pub fn clock_color(&self) -> Option<RgbColor> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.color)
            .or(self.clock_color)
    }

    pub fn clock_opacity(&self) -> Option<u8> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.opacity)
            .or(self.clock_opacity)
    }

    pub fn clock_size(&self) -> Option<u16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.size)
            .or(self.clock_size)
    }

    pub fn date_color(&self) -> Option<RgbColor> {
        self.date
            .as_ref()
            .and_then(|date| date.color)
            .or(self.date_color)
    }

    pub fn date_enabled(&self) -> bool {
        self.date
            .as_ref()
            .and_then(|date| date.enabled)
            .unwrap_or(true)
    }

    pub fn date_font_family(&self) -> Option<&str> {
        self.date
            .as_ref()
            .and_then(|date| date.font_family.as_deref())
    }

    pub fn date_font_weight(&self) -> Option<u16> {
        self.date.as_ref().and_then(|date| date.font_weight)
    }

    pub fn date_opacity(&self) -> Option<u8> {
        self.date
            .as_ref()
            .and_then(|date| date.opacity)
            .or(self.date_opacity)
    }

    pub fn date_size(&self) -> Option<u16> {
        self.date
            .as_ref()
            .and_then(|date| date.size)
            .or(self.date_size)
    }

    pub fn placeholder_color(&self) -> Option<RgbColor> {
        self.placeholder
            .as_ref()
            .and_then(|placeholder| placeholder.color)
            .or(self.placeholder_color)
    }

    pub fn placeholder_enabled(&self) -> bool {
        self.placeholder
            .as_ref()
            .and_then(|placeholder| placeholder.enabled)
            .unwrap_or(true)
    }

    pub fn placeholder_opacity(&self) -> Option<u8> {
        self.placeholder
            .as_ref()
            .and_then(|placeholder| placeholder.opacity)
            .or(self.placeholder_opacity)
    }

    pub fn eye_icon_color(&self) -> Option<RgbColor> {
        self.eye
            .as_ref()
            .and_then(|eye| eye.color)
            .or(self.eye_icon_color)
    }

    pub fn eye_enabled(&self) -> bool {
        self.eye
            .as_ref()
            .and_then(|eye| eye.enabled)
            .unwrap_or(true)
    }

    pub fn eye_icon_opacity(&self) -> Option<u8> {
        self.eye
            .as_ref()
            .and_then(|eye| eye.opacity)
            .or(self.eye_icon_opacity)
    }

    pub fn keyboard_size(&self) -> Option<u16> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.size)
            .or(self.keyboard_size)
    }

    pub fn caps_lock_enabled(&self) -> bool {
        self.caps_lock
            .as_ref()
            .and_then(|caps_lock| caps_lock.enabled)
            .unwrap_or(true)
    }

    pub fn keyboard_enabled(&self) -> bool {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.enabled)
            .unwrap_or(true)
    }

    pub fn keyboard_color(&self) -> Option<RgbColor> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.color)
            .or(self.keyboard_color)
    }

    pub fn keyboard_background_color(&self) -> Option<RgbColor> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.background_color)
    }

    pub fn keyboard_background_size(&self) -> Option<u16> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.background_size)
            .or(self.keyboard_background_size)
    }

    pub fn keyboard_opacity(&self) -> Option<u8> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.opacity)
            .or(self.keyboard_opacity)
    }

    pub fn keyboard_top_offset(&self) -> Option<i16> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.top_offset)
            .or(self.keyboard_top_offset)
    }

    pub fn keyboard_right_offset(&self) -> Option<i16> {
        self.keyboard
            .as_ref()
            .and_then(|keyboard| keyboard.right_offset)
            .or(self.keyboard_right_offset)
    }

    pub fn battery_background_color(&self) -> Option<RgbColor> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.background_color)
            .or(self.battery_background_color)
    }

    pub fn battery_enabled(&self) -> bool {
        self.battery
            .as_ref()
            .and_then(|battery| battery.enabled)
            .unwrap_or(true)
    }

    pub fn battery_color(&self) -> Option<RgbColor> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.color)
            .or(self.battery_color)
    }

    pub fn battery_background_size(&self) -> Option<u16> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.background_size)
            .or(self.battery_background_size)
    }

    pub fn battery_opacity(&self) -> Option<u8> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.opacity)
            .or(self.battery_opacity)
    }

    pub fn battery_size(&self) -> Option<u16> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.size)
            .or(self.battery_size)
    }

    pub fn battery_top_offset(&self) -> Option<i16> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.top_offset)
            .or(self.battery_top_offset)
    }

    pub fn battery_right_offset(&self) -> Option<i16> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.right_offset)
            .or(self.battery_right_offset)
    }

    pub fn battery_gap(&self) -> Option<u16> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.gap)
            .or(self.battery_gap)
    }

    pub fn weather_size(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.size)
            .or(self.weather_size)
    }

    pub fn weather_enabled(&self) -> bool {
        self.weather
            .as_ref()
            .and_then(|weather| weather.enabled)
            .unwrap_or(true)
    }

    pub fn weather_temperature_size(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature_size)
            .or_else(|| self.weather_size())
    }

    pub fn weather_opacity(&self) -> Option<u8> {
        self.weather.as_ref().and_then(|weather| weather.opacity)
    }

    pub fn weather_icon_opacity(&self) -> Option<u8> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.icon_opacity)
            .or_else(|| self.weather_opacity())
    }

    pub fn weather_temperature_opacity(&self) -> Option<u8> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature_opacity)
            .or_else(|| self.weather_opacity())
    }

    pub fn weather_location_opacity(&self) -> Option<u8> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location_opacity)
            .or_else(|| self.weather_opacity())
    }

    pub fn weather_temperature_color(&self) -> Option<RgbColor> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature_color)
    }

    pub fn weather_temperature_font_family(&self) -> Option<&str> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature_font_family.as_deref())
    }

    pub fn weather_temperature_font_weight(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature_font_weight)
    }

    pub fn weather_location_font_family(&self) -> Option<&str> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location_font_family.as_deref())
    }

    pub fn weather_location_font_weight(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location_font_weight)
    }

    pub fn weather_temperature_letter_spacing(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature_letter_spacing)
    }

    pub fn weather_location_size(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location_size)
    }

    pub fn weather_location_color(&self) -> Option<RgbColor> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location_color)
    }

    pub fn weather_icon_size(&self) -> Option<u16> {
        self.weather.as_ref().and_then(|weather| weather.icon_size)
    }

    pub fn weather_icon_gap(&self) -> Option<u16> {
        self.weather.as_ref().and_then(|weather| weather.icon_gap)
    }

    pub fn weather_location_gap(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location_gap)
    }

    pub fn weather_left_offset(&self) -> Option<i16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.left_offset)
    }

    pub fn weather_bottom_offset(&self) -> Option<i16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.bottom_offset)
    }

    pub fn weather_left_padding(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.left_padding)
    }

    pub fn weather_horizontal_padding(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.horizontal_padding.or(weather.left_padding))
    }

    pub fn weather_bottom_padding(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.bottom_padding)
    }

    pub fn weather_alignment(&self) -> WeatherAlignment {
        self.weather
            .as_ref()
            .and_then(|weather| weather.alignment)
            .unwrap_or_default()
    }

    pub fn now_playing_title_color(&self) -> Option<RgbColor> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title_color)
    }

    pub fn now_playing_enabled(&self) -> bool {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.enabled)
            .unwrap_or(true)
    }

    pub fn now_playing_fade_duration_ms(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.fade_duration_ms)
    }

    pub fn now_playing_opacity(&self) -> Option<u8> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.opacity)
    }

    pub fn now_playing_title_opacity(&self) -> Option<u8> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title_opacity)
            .or_else(|| self.now_playing_opacity())
    }

    pub fn now_playing_artist_opacity(&self) -> Option<u8> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist_opacity)
            .or_else(|| self.now_playing_opacity())
    }

    pub fn now_playing_artwork_opacity(&self) -> Option<u8> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artwork_opacity)
            .or_else(|| self.now_playing_opacity())
    }

    pub fn now_playing_artist_color(&self) -> Option<RgbColor> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist_color)
    }

    pub fn now_playing_title_font_family(&self) -> Option<&str> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title_font_family.as_deref())
    }

    pub fn now_playing_artist_font_family(&self) -> Option<&str> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist_font_family.as_deref())
    }

    pub fn now_playing_title_font_weight(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title_font_weight)
    }

    pub fn now_playing_artist_font_weight(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist_font_weight)
    }

    pub fn now_playing_title_size(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title_size)
    }

    pub fn now_playing_artist_size(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist_size)
    }

    pub fn now_playing_width(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.width)
    }

    pub fn now_playing_content_gap(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.content_gap)
    }

    pub fn now_playing_text_gap(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.text_gap)
    }

    pub fn now_playing_artwork_size(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artwork_size)
    }

    pub fn now_playing_artwork_radius(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artwork_radius)
    }

    pub fn now_playing_right_padding(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.right_padding)
    }

    pub fn now_playing_bottom_padding(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.bottom_padding)
    }

    pub fn now_playing_right_offset(&self) -> Option<i16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.right_offset)
    }

    pub fn now_playing_bottom_offset(&self) -> Option<i16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.bottom_offset)
    }

    pub fn status_color(&self) -> Option<RgbColor> {
        self.status
            .as_ref()
            .and_then(|status| status.color)
            .or(self.status_color)
    }

    pub fn status_enabled(&self) -> bool {
        self.status
            .as_ref()
            .and_then(|status| status.enabled)
            .unwrap_or(true)
    }

    pub fn status_opacity(&self) -> Option<u8> {
        self.status
            .as_ref()
            .and_then(|status| status.opacity)
            .or(self.status_opacity)
    }

    pub fn foreground_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.foreground)
            .unwrap_or(self.foreground)
    }

    pub fn muted_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.muted)
            .unwrap_or(self.muted)
    }

    pub fn pending_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.pending)
            .unwrap_or(self.pending)
    }

    pub fn rejected_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.rejected)
            .unwrap_or(self.rejected)
    }
}

const fn default_panel_color() -> RgbColor {
    RgbColor::rgb(22, 28, 38)
}

const fn default_panel_border_color() -> RgbColor {
    RgbColor::rgb(74, 86, 110)
}

const fn default_input_color() -> RgbColor {
    RgbColor::rgb(13, 18, 28)
}

const fn default_input_border_color() -> RgbColor {
    RgbColor::rgb(92, 108, 146)
}

const fn default_input_radius() -> u16 {
    32
}

fn default_geom_font_family() -> String {
    String::from("Geom")
}

fn default_google_sans_flex_font_family() -> String {
    String::from("Google Sans Flex")
}

const fn default_foreground_color() -> RgbColor {
    RgbColor::rgb(240, 244, 250)
}

const fn default_muted_color() -> RgbColor {
    RgbColor::rgb(68, 78, 102)
}

const fn default_pending_color() -> RgbColor {
    RgbColor::rgb(255, 194, 92)
}

const fn default_rejected_color() -> RgbColor {
    RgbColor::rgb(220, 96, 96)
}
