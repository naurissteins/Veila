use std::collections::HashMap;

use veila_common::{AppConfig, FontStyle, HorizontalAlign, VerticalAlign};
use veila_renderer::{ClearColor, RenderScale};

use super::{
    WidgetPosition, color::to_color, resolve_position, scale_i32_opt, scale_position, scale_u32_opt,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherTheme {
    pub enabled: bool,
    pub icon_enabled: bool,
    pub icon_position: Option<WidgetPosition>,
    pub icon_size: Option<i32>,
    pub icon_opacity: Option<u8>,
    pub temperature_enabled: bool,
    pub temperature_color: Option<ClearColor>,
    pub temperature_font_family: Option<String>,
    pub temperature_font_weight: Option<u16>,
    pub temperature_font_style: Option<FontStyle>,
    pub temperature_letter_spacing: Option<u32>,
    pub temperature_font_size: Option<u32>,
    pub temperature_position: Option<WidgetPosition>,
    pub location_enabled: bool,
    pub location_color: Option<ClearColor>,
    pub location_font_family: Option<String>,
    pub location_font_weight: Option<u16>,
    pub location_font_style: Option<FontStyle>,
    pub location_font_size: Option<u32>,
    pub location_position: Option<WidgetPosition>,
}

impl WeatherTheme {
    pub(super) fn from_config(
        config: &AppConfig,
        named_backdrops: &HashMap<String, usize>,
    ) -> Self {
        Self {
            enabled: config.visuals.weather_enabled(),
            icon_enabled: config.visuals.weather_icon_enabled(),
            icon_position: resolve_position(
                config.visuals.weather_icon_position(),
                HorizontalAlign::Left,
                VerticalAlign::Bottom,
                named_backdrops,
            ),
            icon_size: config.visuals.weather_icon_size().map(i32::from),
            icon_opacity: config.visuals.weather_icon_opacity(),
            temperature_enabled: config.visuals.weather_temperature_enabled(),
            temperature_color: config.visuals.weather_temperature_color().map(to_color),
            temperature_font_family: config
                .visuals
                .weather_temperature_font_family()
                .map(str::to_owned),
            temperature_font_weight: config.visuals.weather_temperature_font_weight(),
            temperature_font_style: config.visuals.weather_temperature_font_style(),
            temperature_letter_spacing: config
                .visuals
                .weather_temperature_letter_spacing()
                .map(u32::from),
            temperature_font_size: config
                .visuals
                .weather_temperature_font_size()
                .map(u32::from),
            temperature_position: resolve_position(
                config.visuals.weather_temperature_position(),
                HorizontalAlign::Left,
                VerticalAlign::Bottom,
                named_backdrops,
            ),
            location_enabled: config.visuals.weather_location_enabled(),
            location_color: config.visuals.weather_location_color().map(to_color),
            location_font_family: config
                .visuals
                .weather_location_font_family()
                .map(str::to_owned),
            location_font_weight: config.visuals.weather_location_font_weight(),
            location_font_style: config.visuals.weather_location_font_style(),
            location_font_size: config.visuals.weather_location_font_size().map(u32::from),
            location_position: resolve_position(
                config.visuals.weather_location_position(),
                HorizontalAlign::Left,
                VerticalAlign::Bottom,
                named_backdrops,
            ),
        }
    }

    pub(super) fn scale_for_render(&mut self, scale: RenderScale) {
        // Opacity, colors and font selection are independent of render scale.
        self.icon_position = self
            .icon_position
            .map(|position| scale_position(position, scale));
        self.icon_size = scale_i32_opt(self.icon_size, scale);
        self.temperature_font_size = scale_u32_opt(self.temperature_font_size, scale);
        self.temperature_letter_spacing = scale_u32_opt(self.temperature_letter_spacing, scale);
        self.temperature_position = self
            .temperature_position
            .map(|position| scale_position(position, scale));
        self.location_font_size = scale_u32_opt(self.location_font_size, scale);
        self.location_position = self
            .location_position
            .map(|position| scale_position(position, scale));
    }
}
