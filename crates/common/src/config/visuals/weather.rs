use serde::{Deserialize, Serialize};

use super::{RgbColor, WidgetPositionConfig, input::FontStyle};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeatherVisualConfig {
    #[serde(default)]
    pub icon: Option<WeatherIconVisualConfig>,
    #[serde(default)]
    pub temperature: Option<WeatherTemperatureVisualConfig>,
    #[serde(default)]
    pub location: Option<WeatherLocationVisualConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeatherIconVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeatherTemperatureVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub font_size: Option<u16>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub font_style: Option<FontStyle>,
    #[serde(default)]
    pub letter_spacing: Option<u16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeatherLocationVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub font_size: Option<u16>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub font_style: Option<FontStyle>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for WeatherVisualConfig {
    fn default() -> Self {
        Self {
            icon: Some(WeatherIconVisualConfig::default()),
            temperature: Some(WeatherTemperatureVisualConfig::default()),
            location: Some(WeatherLocationVisualConfig::default()),
        }
    }
}

impl Default for WeatherIconVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            size: Some(40),
            opacity: Some(80),
            position: WidgetPositionConfig {
                halign: Some(super::HorizontalAlign::Left),
                valign: Some(super::VerticalAlign::Bottom),
                x: Some(40),
                y: Some(-106),
                relative_to: None,
            },
        }
    }
}

impl Default for WeatherTemperatureVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            font_size: Some(40),
            font_family: Some(super::default_geom_font_family()),
            font_weight: Some(600),
            font_style: Some(FontStyle::Normal),
            letter_spacing: Some(0),
            color: Some(RgbColor::rgba(255, 255, 255, 186)),
            position: WidgetPositionConfig {
                halign: Some(super::HorizontalAlign::Left),
                valign: Some(super::VerticalAlign::Bottom),
                x: Some(40),
                y: Some(-70),
                relative_to: None,
            },
        }
    }
}

impl Default for WeatherLocationVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            font_size: Some(22),
            font_family: Some(super::default_google_sans_flex_font_family()),
            font_weight: Some(400),
            font_style: Some(FontStyle::Normal),
            color: Some(RgbColor::rgba(214, 227, 255, 148)),
            position: WidgetPositionConfig {
                halign: Some(super::HorizontalAlign::Left),
                valign: Some(super::VerticalAlign::Bottom),
                x: Some(40),
                y: Some(-44),
                relative_to: None,
            },
        }
    }
}

impl super::VisualConfig {
    pub fn weather_enabled(&self) -> bool {
        self.weather_icon_enabled()
            || self.weather_temperature_enabled()
            || self.weather_location_enabled()
    }

    pub fn weather_icon_enabled(&self) -> bool {
        self.weather
            .as_ref()
            .and_then(|weather| weather.icon.as_ref())
            .and_then(|icon| icon.enabled)
            .unwrap_or(true)
    }

    pub fn weather_icon_size(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.icon.as_ref())
            .and_then(|icon| icon.size)
    }

    pub fn weather_icon_opacity(&self) -> Option<u8> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.icon.as_ref())
            .and_then(|icon| icon.opacity)
    }

    pub fn weather_icon_position(&self) -> WidgetPositionConfig {
        self.weather
            .as_ref()
            .and_then(|weather| weather.icon.as_ref())
            .map(|icon| icon.position.clone())
            .unwrap_or_default()
    }

    pub fn weather_temperature_enabled(&self) -> bool {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature.as_ref())
            .and_then(|temperature| temperature.enabled)
            .unwrap_or(true)
    }

    pub fn weather_temperature_color(&self) -> Option<RgbColor> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature.as_ref())
            .and_then(|temperature| temperature.color)
    }

    pub fn weather_temperature_font_family(&self) -> Option<&str> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature.as_ref())
            .and_then(|temperature| temperature.font_family.as_deref())
    }

    pub fn weather_temperature_font_weight(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature.as_ref())
            .and_then(|temperature| temperature.font_weight)
    }

    pub fn weather_temperature_font_style(&self) -> Option<FontStyle> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature.as_ref())
            .and_then(|temperature| temperature.font_style)
    }

    pub fn weather_temperature_letter_spacing(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature.as_ref())
            .and_then(|temperature| temperature.letter_spacing)
    }

    pub fn weather_temperature_font_size(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature.as_ref())
            .and_then(|temperature| temperature.font_size)
    }

    pub fn weather_temperature_position(&self) -> WidgetPositionConfig {
        self.weather
            .as_ref()
            .and_then(|weather| weather.temperature.as_ref())
            .map(|temperature| temperature.position.clone())
            .unwrap_or_default()
    }

    pub fn weather_location_enabled(&self) -> bool {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location.as_ref())
            .and_then(|location| location.enabled)
            .unwrap_or(true)
    }

    pub fn weather_location_font_family(&self) -> Option<&str> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location.as_ref())
            .and_then(|location| location.font_family.as_deref())
    }

    pub fn weather_location_font_weight(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location.as_ref())
            .and_then(|location| location.font_weight)
    }

    pub fn weather_location_font_style(&self) -> Option<FontStyle> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location.as_ref())
            .and_then(|location| location.font_style)
    }

    pub fn weather_location_font_size(&self) -> Option<u16> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location.as_ref())
            .and_then(|location| location.font_size)
    }

    pub fn weather_location_color(&self) -> Option<RgbColor> {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location.as_ref())
            .and_then(|location| location.color)
    }

    pub fn weather_location_position(&self) -> WidgetPositionConfig {
        self.weather
            .as_ref()
            .and_then(|weather| weather.location.as_ref())
            .map(|location| location.position.clone())
            .unwrap_or_default()
    }
}
