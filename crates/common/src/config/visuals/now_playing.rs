use serde::{Deserialize, Serialize};

use super::{input::FontStyle, RgbColor};

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
}
