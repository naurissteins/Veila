use std::collections::HashMap;

use veila_common::{AppConfig, FontStyle, HorizontalAlign, VerticalAlign};
use veila_renderer::{ClearColor, RenderScale};

use super::{
    DEFAULT_SURFACE_COLOR, WidgetPosition, color::to_color, resolve_position, scale_i32_opt,
    scale_position, scale_u32_opt,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvatarTheme {
    pub enabled: bool,
    pub background_color: ClearColor,
    pub size: Option<i32>,
    pub radius: Option<i32>,
    pub offset_y: Option<i32>,
    pub position: Option<WidgetPosition>,
    pub placeholder_padding: Option<i32>,
    pub icon_color: Option<ClearColor>,
    pub ring_color: Option<ClearColor>,
    pub ring_width: Option<i32>,
    pub gap: Option<i32>,
}

impl AvatarTheme {
    pub(super) fn from_config(
        config: &AppConfig,
        named_backdrops: &HashMap<String, usize>,
    ) -> Self {
        Self {
            enabled: config.visuals.avatar_enabled(),
            background_color: config
                .visuals
                .avatar_background_color()
                .map(to_color)
                .unwrap_or_else(|| to_color(DEFAULT_SURFACE_COLOR)),
            size: config.visuals.avatar_size().map(i32::from),
            radius: config
                .visuals
                .avatar_radius()
                .map(|radius| i32::from(radius).clamp(0, 320)),
            // Flow offsets are independent of explicit widget positions.
            offset_y: Some(0),
            position: resolve_position(
                config.visuals.avatar_position(),
                HorizontalAlign::Center,
                VerticalAlign::Center,
                named_backdrops,
            ),
            placeholder_padding: config.visuals.avatar_placeholder_padding().map(i32::from),
            icon_color: config.visuals.avatar_icon_color().map(to_color),
            ring_color: config.visuals.avatar_ring_color().map(to_color),
            ring_width: config.visuals.avatar_ring_width().map(i32::from),
            gap: Some(24),
        }
    }

    pub(super) fn scale_for_render(&mut self, scale: RenderScale) {
        self.size = scale_i32_opt(self.size, scale);
        self.radius = scale_i32_opt(self.radius, scale);
        self.offset_y = scale_i32_opt(self.offset_y, scale);
        self.position = self
            .position
            .map(|position| scale_position(position, scale));
        self.placeholder_padding = scale_i32_opt(self.placeholder_padding, scale);
        self.ring_width = scale_i32_opt(self.ring_width, scale);
        self.gap = scale_i32_opt(self.gap, scale);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsernameTheme {
    pub enabled: bool,
    pub font_family: Option<String>,
    pub font_weight: Option<u16>,
    pub font_style: Option<FontStyle>,
    pub color: Option<ClearColor>,
    pub font_size: Option<u32>,
    pub offset_y: Option<i32>,
    pub position: Option<WidgetPosition>,
    pub gap: Option<i32>,
}

impl UsernameTheme {
    pub(super) fn from_config(
        config: &AppConfig,
        named_backdrops: &HashMap<String, usize>,
    ) -> Self {
        Self {
            enabled: config.visuals.username_enabled(),
            font_family: config.visuals.username_font_family().map(str::to_owned),
            font_weight: config.visuals.username_font_weight(),
            font_style: config.visuals.username_font_style(),
            color: config.visuals.username_color().map(to_color),
            font_size: config.visuals.username_font_size().map(u32::from),
            // Flow offsets are independent of explicit widget positions.
            offset_y: Some(0),
            position: resolve_position(
                config.visuals.username_position(),
                HorizontalAlign::Center,
                VerticalAlign::Center,
                named_backdrops,
            ),
            gap: Some(28),
        }
    }

    pub(super) fn scale_for_render(&mut self, scale: RenderScale) {
        self.font_size = scale_u32_opt(self.font_size, scale);
        self.offset_y = scale_i32_opt(self.offset_y, scale);
        self.position = self
            .position
            .map(|position| scale_position(position, scale));
        self.gap = scale_i32_opt(self.gap, scale);
    }
}
