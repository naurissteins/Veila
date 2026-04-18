use serde::{Deserialize, Serialize};

use super::{RgbColor, input::FontStyle, layer::LayerMode};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NowPlayingBackgroundConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub mode: Option<LayerMode>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(default)]
    pub blur_radius: Option<u8>,
    #[serde(default)]
    pub radius: Option<u16>,
    #[serde(default)]
    pub padding_x: Option<u16>,
    #[serde(default)]
    pub padding_y: Option<u16>,
    #[serde(default)]
    pub border_color: Option<RgbColor>,
    #[serde(default)]
    pub border_opacity: Option<u8>,
    #[serde(default)]
    pub border_width: Option<u16>,
}

impl Default for NowPlayingBackgroundConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            mode: Some(LayerMode::Solid),
            color: Some(RgbColor::rgb(0, 0, 0)),
            opacity: Some(24),
            blur_radius: Some(12),
            radius: Some(18),
            padding_x: Some(18),
            padding_y: Some(12),
            border_color: Some(RgbColor::rgb(255, 255, 255)),
            border_opacity: Some(0),
            border_width: Some(0),
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
    pub title_font_style: Option<FontStyle>,
    #[serde(default)]
    pub artist_font_style: Option<FontStyle>,
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
    #[serde(default)]
    pub background: Option<NowPlayingBackgroundConfig>,
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
            title_font_family: Some(super::default_google_sans_flex_font_family()),
            artist_font_family: Some(super::default_google_sans_flex_font_family()),
            title_font_weight: Some(400),
            artist_font_weight: Some(400),
            title_font_style: Some(FontStyle::Normal),
            artist_font_style: Some(FontStyle::Normal),
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
            background: Some(NowPlayingBackgroundConfig::default()),
        }
    }
}

impl super::VisualConfig {
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

    pub fn now_playing_title_color(&self) -> Option<RgbColor> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title_color)
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

    pub fn now_playing_title_font_style(&self) -> Option<FontStyle> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title_font_style)
    }

    pub fn now_playing_artist_font_weight(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist_font_weight)
    }

    pub fn now_playing_artist_font_style(&self) -> Option<FontStyle> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist_font_style)
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

    pub fn now_playing_background_enabled(&self) -> bool {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.background.as_ref())
            .and_then(|background| background.enabled)
            .unwrap_or(false)
    }

    pub fn now_playing_background_mode(&self) -> LayerMode {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.background.as_ref())
            .and_then(|background| background.mode)
            .unwrap_or(LayerMode::Solid)
    }

    pub fn now_playing_background_color(&self) -> Option<RgbColor> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.background.as_ref())
            .and_then(|background| background.color)
            .or(Some(RgbColor::rgb(0, 0, 0)))
    }

    pub fn now_playing_background_opacity(&self) -> Option<u8> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.background.as_ref())
            .and_then(|background| background.opacity)
            .or(Some(24))
    }

    pub fn now_playing_background_blur_radius(&self) -> Option<u8> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.background.as_ref())
            .and_then(|background| background.blur_radius)
            .or(Some(12))
    }

    pub fn now_playing_background_radius(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.background.as_ref())
            .and_then(|background| background.radius)
            .or(Some(18))
    }

    pub fn now_playing_background_padding_x(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.background.as_ref())
            .and_then(|background| background.padding_x)
            .or(Some(18))
    }

    pub fn now_playing_background_padding_y(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.background.as_ref())
            .and_then(|background| background.padding_y)
            .or(Some(12))
    }

    pub fn now_playing_background_border_color(&self) -> Option<RgbColor> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.background.as_ref())
            .and_then(|background| background.border_color)
            .or(Some(RgbColor::rgb(255, 255, 255)))
    }

    pub fn now_playing_background_border_opacity(&self) -> Option<u8> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.background.as_ref())
            .and_then(|background| background.border_opacity)
            .or(Some(0))
    }

    pub fn now_playing_background_border_width(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.background.as_ref())
            .and_then(|background| background.border_width)
            .or(Some(0))
    }
}
