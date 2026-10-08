use std::collections::HashMap;

use veila_common::{
    AppConfig, FontStyle, HorizontalAlign, InputRevealMode, StatusDisplayMode, VerticalAlign,
};
use veila_renderer::{ClearColor, RenderScale};

use super::{
    WidgetPosition, color::to_color, resolve_position, scale_i32, scale_i32_opt, scale_position,
    scale_u32_opt,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputTheme {
    pub background_color: ClearColor,
    pub border_color: ClearColor,
    pub font_family: Option<String>,
    pub font_weight: Option<u16>,
    pub font_style: Option<FontStyle>,
    pub font_size: Option<u32>,
    pub reveal_on_interaction: bool,
    pub reveal_mode: InputRevealMode,
    pub position: Option<WidgetPosition>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub radius: i32,
    pub border_width: Option<i32>,
    pub mask_color: Option<ClearColor>,
}

impl InputTheme {
    pub(super) fn from_config(
        config: &AppConfig,
        named_backdrops: &HashMap<String, usize>,
    ) -> Self {
        Self {
            background_color: to_color(config.visuals.input_background_color()),
            border_color: to_color(config.visuals.input_border_color()),
            font_family: config.visuals.input_font_family().map(str::to_owned),
            font_weight: config.visuals.input_font_weight(),
            font_style: config.visuals.input_font_style(),
            font_size: config.visuals.input_font_size().map(u32::from),
            reveal_on_interaction: config.visuals.input_reveal_on_interaction(),
            reveal_mode: config.visuals.input_reveal_mode(),
            position: resolve_position(
                config.visuals.input_position(),
                HorizontalAlign::Center,
                VerticalAlign::Center,
                named_backdrops,
            ),
            width: config.visuals.input_width().map(i32::from),
            height: config.visuals.input_height().map(i32::from),
            radius: i32::from(config.visuals.input_radius()),
            border_width: config.visuals.input_border_width().map(i32::from),
            mask_color: config.visuals.input_mask_color().map(to_color),
        }
    }

    pub(super) fn scale_for_render(&mut self, scale: RenderScale) {
        self.font_size = scale_u32_opt(self.font_size, scale);
        self.position = self
            .position
            .map(|position| scale_position(position, scale));
        self.width = scale_i32_opt(self.width, scale);
        self.height = scale_i32_opt(self.height, scale);
        self.radius = scale_i32(self.radius, scale);
        self.border_width = scale_i32_opt(self.border_width, scale);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealTheme {
    pub text: String,
    pub enabled: bool,
    pub color: Option<ClearColor>,
    pub font_family: Option<String>,
    pub font_weight: Option<u16>,
    pub font_style: Option<FontStyle>,
    pub font_size: Option<u32>,
}

impl RevealTheme {
    pub(super) fn from_config(config: &AppConfig) -> Self {
        Self {
            text: config.visuals.reveal_text(),
            enabled: config.visuals.reveal_enabled(),
            color: config.visuals.reveal_color().map(to_color),
            font_family: config.visuals.reveal_font_family().map(str::to_owned),
            font_weight: config.visuals.reveal_font_weight(),
            font_style: config.visuals.reveal_font_style(),
            font_size: config.visuals.reveal_font_size().map(u32::from),
        }
    }

    pub(super) fn scale_for_render(&mut self, scale: RenderScale) {
        self.font_size = scale_u32_opt(self.font_size, scale);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceholderTheme {
    pub enabled: bool,
    pub color: Option<ClearColor>,
}

impl PlaceholderTheme {
    pub(super) fn from_config(config: &AppConfig) -> Self {
        Self {
            enabled: config.visuals.placeholder_enabled(),
            color: config.visuals.placeholder_color().map(to_color),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EyeTheme {
    pub enabled: bool,
    pub color: Option<ClearColor>,
}

impl EyeTheme {
    pub(super) fn from_config(config: &AppConfig) -> Self {
        Self {
            enabled: config.visuals.eye_enabled(),
            color: config.visuals.eye_icon_color().map(to_color),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapsLockTheme {
    pub enabled: bool,
    pub color: Option<ClearColor>,
}

impl CapsLockTheme {
    pub(super) fn from_config(config: &AppConfig) -> Self {
        Self {
            enabled: config.visuals.caps_lock_enabled(),
            color: config.visuals.caps_lock_color().map(to_color),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusTheme {
    pub position: Option<WidgetPosition>,
    pub mode: StatusDisplayMode,
    pub enabled: bool,
    pub color: Option<ClearColor>,
    pub pending_color: Option<ClearColor>,
    pub rejected_color: Option<ClearColor>,
}

impl StatusTheme {
    pub(super) fn from_config(
        config: &AppConfig,
        named_backdrops: &HashMap<String, usize>,
    ) -> Self {
        Self {
            position: resolve_position(
                config.visuals.status_position(),
                HorizontalAlign::Center,
                VerticalAlign::Center,
                named_backdrops,
            ),
            mode: config.visuals.status_mode(),
            enabled: config.visuals.status_enabled(),
            color: config.visuals.status_color().map(to_color),
            pending_color: config.visuals.status_pending_color().map(to_color),
            rejected_color: config.visuals.status_rejected_color().map(to_color),
        }
    }

    pub(super) fn scale_for_render(&mut self, scale: RenderScale) {
        // Status glyph sizing is fixed or inherited from input; only its anchor scales.
        self.position = self
            .position
            .map(|position| scale_position(position, scale));
    }
}
