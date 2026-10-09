use std::collections::HashMap;

use veila_common::{
    AppConfig, ClockAlignment, ClockFormat, ClockStyle, DateFormat, FontStyle, HorizontalAlign,
    VerticalAlign,
};
use veila_renderer::{ClearColor, RenderScale};

use super::{
    WidgetPosition, color::to_color, resolve_position, scale_i32_opt, scale_position, scale_u32_opt,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockTheme {
    pub enabled: bool,
    pub alignment: ClockAlignment,
    pub center_in_layer: bool,
    pub offset_x: Option<i32>,
    pub offset_y: Option<i32>,
    pub position: Option<WidgetPosition>,
    pub font_family: Option<String>,
    pub font_weight: Option<u16>,
    pub font_style: Option<FontStyle>,
    pub style: ClockStyle,
    pub format: ClockFormat,
    pub meridiem_font_size: Option<u32>,
    pub meridiem_x: Option<i32>,
    pub meridiem_y: Option<i32>,
    pub color: Option<ClearColor>,
    pub font_size: Option<u32>,
    pub gap: Option<i32>,
}

impl ClockTheme {
    pub(super) fn from_config(
        config: &AppConfig,
        named_backdrops: &HashMap<String, usize>,
    ) -> Self {
        Self {
            enabled: config.visuals.clock_enabled(),
            // Flow alignment and offsets also position an unanchored date.
            alignment: ClockAlignment::TopCenter,
            center_in_layer: false,
            offset_x: Some(0),
            offset_y: Some(0),
            position: resolve_position(
                config.visuals.clock_position(),
                HorizontalAlign::Center,
                VerticalAlign::Top,
                named_backdrops,
            ),
            font_family: config.visuals.clock_font_family().map(str::to_owned),
            font_weight: config.visuals.clock_font_weight(),
            font_style: config.visuals.clock_font_style(),
            style: config.visuals.clock_style(),
            format: config.visuals.clock_format(),
            meridiem_font_size: config.visuals.clock_meridiem_font_size().map(u32::from),
            meridiem_x: config.visuals.clock_meridiem_x().map(i32::from),
            meridiem_y: config.visuals.clock_meridiem_y().map(i32::from),
            color: config.visuals.clock_color().map(to_color),
            font_size: config.visuals.clock_font_size().map(u32::from),
            gap: Some(20),
        }
    }

    pub(super) fn scale_for_render(&mut self, scale: RenderScale) {
        self.offset_x = scale_i32_opt(self.offset_x, scale);
        self.offset_y = scale_i32_opt(self.offset_y, scale);
        self.position = self
            .position
            .map(|position| scale_position(position, scale));
        self.meridiem_font_size = scale_u32_opt(self.meridiem_font_size, scale);
        self.meridiem_x = scale_i32_opt(self.meridiem_x, scale);
        self.meridiem_y = scale_i32_opt(self.meridiem_y, scale);
        self.font_size = scale_u32_opt(self.font_size, scale);
        self.gap = scale_i32_opt(self.gap, scale);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateTheme {
    pub enabled: bool,
    pub font_family: Option<String>,
    pub font_weight: Option<u16>,
    pub font_style: Option<FontStyle>,
    pub format: DateFormat,
    pub color: Option<ClearColor>,
    pub position: Option<WidgetPosition>,
    pub font_size: Option<u32>,
}

impl DateTheme {
    pub(super) fn from_config(
        config: &AppConfig,
        named_backdrops: &HashMap<String, usize>,
    ) -> Self {
        Self {
            enabled: config.visuals.date_enabled(),
            font_family: config.visuals.date_font_family().map(str::to_owned),
            font_weight: config.visuals.date_font_weight(),
            font_style: config.visuals.date_font_style(),
            format: config.visuals.date_format(),
            color: config.visuals.date_color().map(to_color),
            position: resolve_position(
                config.visuals.date_position(),
                HorizontalAlign::Center,
                VerticalAlign::Top,
                named_backdrops,
            ),
            font_size: config.visuals.date_font_size().map(u32::from),
        }
    }

    pub(super) fn scale_for_render(&mut self, scale: RenderScale) {
        self.position = self
            .position
            .map(|position| scale_position(position, scale));
        self.font_size = scale_u32_opt(self.font_size, scale);
    }
}
