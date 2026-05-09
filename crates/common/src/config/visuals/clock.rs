use serde::{Deserialize, Serialize};

use super::{RgbColor, WidgetPositionConfig, input::FontStyle};

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
    pub format: Option<ClockFormat>,
    #[serde(default)]
    pub meridiem_font_size: Option<u16>,
    #[serde(default)]
    pub meridiem_x: Option<i16>,
    #[serde(default)]
    pub meridiem_y: Option<i16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub font_size: Option<u16>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for ClockVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            font_family: Some(super::default_geom_font_family()),
            font_weight: Some(600),
            font_style: Some(FontStyle::Normal),
            style: Some(ClockStyle::Standard),
            format: Some(ClockFormat::TwentyFourHour),
            meridiem_font_size: Some(22),
            meridiem_x: Some(6),
            meridiem_y: Some(7),
            color: Some(RgbColor::rgba(255, 255, 255, 102)),
            font_size: Some(88),
            position: WidgetPositionConfig {
                halign: Some(super::HorizontalAlign::Center),
                valign: Some(super::VerticalAlign::Top),
                x: Some(0),
                y: Some(40),
                relative_to: None,
            },
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
    pub font_size: Option<u16>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for DateVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            font_family: Some(super::default_geom_font_family()),
            font_weight: Some(600),
            font_style: Some(FontStyle::Normal),
            color: Some(RgbColor::rgba(255, 255, 255, 102)),
            font_size: Some(18),
            position: WidgetPositionConfig {
                halign: Some(super::HorizontalAlign::Center),
                valign: Some(super::VerticalAlign::Top),
                x: Some(0),
                y: Some(140),
                relative_to: None,
            },
        }
    }
}

impl super::VisualConfig {
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

    pub fn clock_format(&self) -> ClockFormat {
        self.clock
            .as_ref()
            .and_then(|clock| clock.format)
            .or(self.clock_format)
            .unwrap_or_default()
    }

    pub fn clock_meridiem_font_size(&self) -> Option<u16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.meridiem_font_size)
            .or(self.clock_meridiem_font_size)
    }

    pub fn clock_meridiem_x(&self) -> Option<i16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.meridiem_x)
            .or(self.clock_meridiem_x)
    }

    pub fn clock_meridiem_y(&self) -> Option<i16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.meridiem_y)
            .or(self.clock_meridiem_y)
    }

    pub fn clock_color(&self) -> Option<RgbColor> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.color)
            .or(self.clock_color)
    }

    pub fn clock_font_size(&self) -> Option<u16> {
        self.clock
            .as_ref()
            .and_then(|clock| clock.font_size)
            .or(self.clock_font_size)
    }

    pub fn clock_position(&self) -> WidgetPositionConfig {
        self.clock
            .as_ref()
            .map(|clock| clock.position.clone())
            .unwrap_or_default()
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

    pub fn date_font_size(&self) -> Option<u16> {
        self.date
            .as_ref()
            .and_then(|date| date.font_size)
            .or(self.date_font_size)
    }

    pub fn date_position(&self) -> WidgetPositionConfig {
        self.date
            .as_ref()
            .map(|date| date.position.clone())
            .unwrap_or_default()
    }
}
