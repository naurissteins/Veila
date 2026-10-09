use std::collections::HashMap;

use veila_common::{AppConfig, FontStyle, HorizontalAlign, VerticalAlign};
use veila_renderer::{ClearColor, RenderScale};

use super::{
    WidgetPosition, color::to_color, resolve_position, scale_i32_opt, scale_position, scale_u32_opt,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NowPlayingTheme {
    pub enabled: bool,
    pub fade_duration_ms: Option<u64>,
    pub artwork_enabled: bool,
    pub artist_enabled: bool,
    pub title_enabled: bool,
    pub artwork_position: Option<WidgetPosition>,
    pub artwork_size: Option<i32>,
    pub artwork_radius: Option<i32>,
    pub artwork_opacity: Option<u8>,
    pub artist_position: Option<WidgetPosition>,
    pub artist_width: Option<i32>,
    pub artist_color: Option<ClearColor>,
    pub artist_font_family: Option<String>,
    pub artist_font_size: Option<u32>,
    pub artist_font_weight: Option<u16>,
    pub artist_font_style: Option<FontStyle>,
    pub title_position: Option<WidgetPosition>,
    pub title_width: Option<i32>,
    pub title_color: Option<ClearColor>,
    pub title_font_family: Option<String>,
    pub title_font_size: Option<u32>,
    pub title_font_weight: Option<u16>,
    pub title_font_style: Option<FontStyle>,
}

impl NowPlayingTheme {
    pub(super) fn from_config(
        config: &AppConfig,
        named_backdrops: &HashMap<String, usize>,
    ) -> Self {
        Self {
            enabled: config.visuals.now_playing_enabled(),
            fade_duration_ms: config.visuals.now_playing_fade_duration_ms().map(u64::from),
            artwork_enabled: config.visuals.now_playing_artwork_enabled(),
            artist_enabled: config.visuals.now_playing_artist_enabled(),
            title_enabled: config.visuals.now_playing_title_enabled(),
            artwork_position: resolve_position(
                config.visuals.now_playing_artwork_position(),
                HorizontalAlign::Right,
                VerticalAlign::Bottom,
                named_backdrops,
            ),
            artwork_size: config.visuals.now_playing_artwork_size().map(i32::from),
            artwork_radius: config.visuals.now_playing_artwork_radius().map(i32::from),
            artwork_opacity: config.visuals.now_playing_artwork_opacity(),
            artist_position: resolve_position(
                config.visuals.now_playing_artist_position(),
                HorizontalAlign::Right,
                VerticalAlign::Bottom,
                named_backdrops,
            ),
            artist_width: config.visuals.now_playing_artist_width().map(i32::from),
            artist_color: config.visuals.now_playing_artist_color().map(to_color),
            artist_font_family: config
                .visuals
                .now_playing_artist_font_family()
                .map(str::to_owned),
            artist_font_size: config.visuals.now_playing_artist_font_size().map(u32::from),
            artist_font_weight: config.visuals.now_playing_artist_font_weight(),
            artist_font_style: config.visuals.now_playing_artist_font_style(),
            title_position: resolve_position(
                config.visuals.now_playing_title_position(),
                HorizontalAlign::Right,
                VerticalAlign::Bottom,
                named_backdrops,
            ),
            title_width: config.visuals.now_playing_title_width().map(i32::from),
            title_color: config.visuals.now_playing_title_color().map(to_color),
            title_font_family: config
                .visuals
                .now_playing_title_font_family()
                .map(str::to_owned),
            title_font_size: config.visuals.now_playing_title_font_size().map(u32::from),
            title_font_weight: config.visuals.now_playing_title_font_weight(),
            title_font_style: config.visuals.now_playing_title_font_style(),
        }
    }

    pub(super) fn scale_for_render(&mut self, scale: RenderScale) {
        // Fade timing and opacity are independent of render scale.
        self.artwork_position = self
            .artwork_position
            .map(|position| scale_position(position, scale));
        self.artwork_size = scale_i32_opt(self.artwork_size, scale);
        self.artwork_radius = scale_i32_opt(self.artwork_radius, scale);
        self.artist_position = self
            .artist_position
            .map(|position| scale_position(position, scale));
        self.artist_width = scale_i32_opt(self.artist_width, scale);
        self.artist_font_size = scale_u32_opt(self.artist_font_size, scale);
        self.title_position = self
            .title_position
            .map(|position| scale_position(position, scale));
        self.title_width = scale_i32_opt(self.title_width, scale);
        self.title_font_size = scale_u32_opt(self.title_font_size, scale);
    }
}
