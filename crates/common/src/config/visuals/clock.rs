use serde::{Deserialize, Serialize};

use super::{RgbColor, input::FontStyle};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClockVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub font_style: Option<FontStyle>,
    #[serde(default)]
    pub style: Option<ClockStyle>,
    #[serde(default)]
    pub alignment: Option<ClockAlignment>,
    #[serde(default)]
    pub center_in_layer: Option<bool>,
    #[serde(default)]
    pub offset_x: Option<i16>,
    #[serde(default)]
    pub offset_y: Option<i16>,
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
            font_family: Some(super::default_geom_font_family()),
            font_weight: Some(600),
            font_style: Some(FontStyle::Normal),
            style: Some(ClockStyle::Standard),
            alignment: Some(ClockAlignment::TopCenter),
            center_in_layer: Some(false),
            offset_x: Some(0),
            offset_y: Some(0),
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
pub enum ClockStyle {
    #[default]
    #[serde(rename = "standard")]
    Standard,
    #[serde(rename = "stacked")]
    Stacked,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClockAlignment {
    #[default]
    #[serde(rename = "top-center")]
    TopCenter,
    #[serde(rename = "top-right")]
    TopRight,
    #[serde(rename = "top-left")]
    TopLeft,
    #[serde(rename = "center-center")]
    CenterCenter,
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
    pub font_style: Option<FontStyle>,
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
            font_family: Some(super::default_geom_font_family()),
            font_weight: Some(600),
            font_style: Some(FontStyle::Normal),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(50),
            size: Some(2),
        }
    }
}

impl super::VisualConfig {
    pub fn clock_gap(&self) -> Option<u16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.gap)
            .or(self.clock_gap)
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

    pub fn clock_font_style(&self) -> Option<FontStyle> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.font_style)
            .or(self.clock_font_style)
    }

    pub fn clock_style(&self) -> ClockStyle {
        self.clock
            .as_ref()
            .and_then(|clock| clock.style)
            .or(self.clock_style)
            .unwrap_or_default()
    }

    pub fn clock_alignment(&self) -> ClockAlignment {
        self.clock
            .as_ref()
            .and_then(|clock| clock.alignment)
            .unwrap_or_default()
    }

    pub fn clock_center_in_layer(&self) -> bool {
        self.clock
            .as_ref()
            .and_then(|clock| clock.center_in_layer)
            .or(self.clock_center_in_layer)
            .unwrap_or(false)
    }

    pub fn clock_offset_x(&self) -> Option<i16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.offset_x)
            .or(self.clock_offset_x)
    }

    pub fn clock_offset_y(&self) -> Option<i16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.offset_y)
            .or(self.clock_offset_y)
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

    pub fn date_font_style(&self) -> Option<FontStyle> {
        self.date.as_ref().and_then(|date| date.font_style)
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
}
