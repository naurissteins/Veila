use serde::{Deserialize, Serialize};

use super::{input::FontStyle, RgbColor};

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
    pub temperature_font_style: Option<FontStyle>,
    #[serde(default)]
    pub temperature_letter_spacing: Option<u16>,
    #[serde(default)]
    pub location_font_family: Option<String>,
    #[serde(default)]
    pub location_font_weight: Option<u16>,
    #[serde(default)]
    pub location_font_style: Option<FontStyle>,
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
            temperature_font_family: Some(super::default_geom_font_family()),
            temperature_font_weight: Some(600),
            temperature_font_style: Some(FontStyle::Normal),
            temperature_letter_spacing: Some(0),
            location_font_family: Some(super::default_google_sans_flex_font_family()),
            location_font_weight: Some(400),
            location_font_style: Some(FontStyle::Normal),
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

impl super::VisualConfig {
    pub fn weather_enabled(&self) -> bool {
        self.weather
            .as_ref()
            .and_then(|weather| weather.enabled)
            .unwrap_or(true)
    }

    pub fn weather_size(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.size)
            .or(self.weather_size)
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

    pub fn weather_temperature_font_style(&self) -> Option<FontStyle> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature_font_style)
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

    pub fn weather_location_font_style(&self) -> Option<FontStyle> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location_font_style)
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
}
