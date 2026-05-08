use serde::{Deserialize, Serialize};

use super::{RgbColor, input::FontStyle, layer::LayerMode, layout::WidgetPositionConfig};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NowPlayingBackgroundConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub mode: Option<LayerMode>,
    #[serde(default)]
    pub color: Option<RgbColor>,
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
    pub border_width: Option<u16>,
}

impl Default for NowPlayingBackgroundConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            mode: Some(LayerMode::Solid),
            color: Some(RgbColor::rgba(0, 0, 0, 61)),
            blur_radius: Some(12),
            radius: Some(18),
            padding_x: Some(18),
            padding_y: Some(12),
            border_color: Some(RgbColor::rgba(255, 255, 255, 0)),
            border_width: Some(0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NowPlayingArtworkVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub radius: Option<u16>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for NowPlayingArtworkVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            size: Some(44),
            radius: Some(8),
            opacity: Some(90),
            position: WidgetPositionConfig {
                halign: Some(super::HorizontalAlign::Right),
                valign: Some(super::VerticalAlign::Bottom),
                x: Some(-388),
                y: Some(-56),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NowPlayingTextVisualConfig {
    #[serde(default)]
    pub width: Option<u16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_size: Option<u16>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub font_style: Option<FontStyle>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NowPlayingVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub fade_duration_ms: Option<u16>,
    #[serde(default)]
    pub artwork: Option<NowPlayingArtworkVisualConfig>,
    #[serde(default)]
    pub artist: Option<NowPlayingTextVisualConfig>,
    #[serde(default)]
    pub title: Option<NowPlayingTextVisualConfig>,
    #[serde(default)]
    pub background: Option<NowPlayingBackgroundConfig>,
}

impl Default for NowPlayingVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            fade_duration_ms: Some(320),
            artwork: Some(NowPlayingArtworkVisualConfig::default()),
            artist: Some(NowPlayingTextVisualConfig {
                width: Some(318),
                color: Some(RgbColor::rgba(255, 255, 255, 99)),
                font_family: Some(super::default_google_sans_flex_font_family()),
                font_size: Some(2),
                font_weight: Some(400),
                font_style: Some(FontStyle::Normal),
                position: WidgetPositionConfig {
                    halign: Some(super::HorizontalAlign::Right),
                    valign: Some(super::VerticalAlign::Bottom),
                    x: Some(-52),
                    y: Some(-88),
                },
            }),
            title: Some(NowPlayingTextVisualConfig {
                width: Some(318),
                color: Some(RgbColor::rgba(255, 255, 255, 175)),
                font_family: Some(super::default_google_sans_flex_font_family()),
                font_size: Some(2),
                font_weight: Some(400),
                font_style: Some(FontStyle::Normal),
                position: WidgetPositionConfig {
                    halign: Some(super::HorizontalAlign::Right),
                    valign: Some(super::VerticalAlign::Bottom),
                    x: Some(-52),
                    y: Some(-56),
                },
            }),
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

    pub fn now_playing_artwork_enabled(&self) -> bool {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artwork.as_ref())
            .and_then(|artwork| artwork.enabled)
            .unwrap_or(true)
    }

    pub fn now_playing_artwork_size(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artwork.as_ref())
            .and_then(|artwork| artwork.size)
    }

    pub fn now_playing_artwork_radius(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artwork.as_ref())
            .and_then(|artwork| artwork.radius)
    }

    pub fn now_playing_artwork_opacity(&self) -> Option<u8> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artwork.as_ref())
            .and_then(|artwork| artwork.opacity)
    }

    pub fn now_playing_artwork_position(&self) -> WidgetPositionConfig {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artwork.as_ref())
            .map(|artwork| artwork.position)
            .unwrap_or_default()
    }

    pub fn now_playing_artist_width(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist.as_ref())
            .and_then(|artist| artist.width)
    }

    pub fn now_playing_artist_color(&self) -> Option<RgbColor> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist.as_ref())
            .and_then(|artist| artist.color)
    }

    pub fn now_playing_artist_font_family(&self) -> Option<&str> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist.as_ref())
            .and_then(|artist| artist.font_family.as_deref())
    }

    pub fn now_playing_artist_font_size(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist.as_ref())
            .and_then(|artist| artist.font_size)
    }

    pub fn now_playing_artist_font_weight(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist.as_ref())
            .and_then(|artist| artist.font_weight)
    }

    pub fn now_playing_artist_font_style(&self) -> Option<FontStyle> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist.as_ref())
            .and_then(|artist| artist.font_style)
    }

    pub fn now_playing_artist_position(&self) -> WidgetPositionConfig {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.artist.as_ref())
            .map(|artist| artist.position)
            .unwrap_or_default()
    }

    pub fn now_playing_title_width(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title.as_ref())
            .and_then(|title| title.width)
    }

    pub fn now_playing_title_color(&self) -> Option<RgbColor> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title.as_ref())
            .and_then(|title| title.color)
    }

    pub fn now_playing_title_font_family(&self) -> Option<&str> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title.as_ref())
            .and_then(|title| title.font_family.as_deref())
    }

    pub fn now_playing_title_font_size(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title.as_ref())
            .and_then(|title| title.font_size)
    }

    pub fn now_playing_title_font_weight(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title.as_ref())
            .and_then(|title| title.font_weight)
    }

    pub fn now_playing_title_font_style(&self) -> Option<FontStyle> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title.as_ref())
            .and_then(|title| title.font_style)
    }

    pub fn now_playing_title_position(&self) -> WidgetPositionConfig {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.title.as_ref())
            .map(|title| title.position)
            .unwrap_or_default()
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
            .or(Some(RgbColor::rgba(0, 0, 0, 61)))
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
            .or(Some(RgbColor::rgba(255, 255, 255, 0)))
    }

    pub fn now_playing_background_border_width(&self) -> Option<u16> {
        self.now_playing
            .as_ref()
            .and_then(|now_playing| now_playing.background.as_ref())
            .and_then(|background| background.border_width)
            .or(Some(0))
    }
}
