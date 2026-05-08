use serde::{Deserialize, Serialize};

use super::{RgbColor, WidgetPositionConfig};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvatarVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub background_color: Option<RgbColor>,
    #[serde(default)]
    pub placeholder_padding: Option<u16>,
    #[serde(default)]
    pub ring_color: Option<RgbColor>,
    #[serde(default)]
    pub ring_width: Option<u16>,
    #[serde(default)]
    pub icon_color: Option<RgbColor>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for AvatarVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            size: Some(192),
            background_color: Some(RgbColor::rgba(255, 255, 255, 15)),
            placeholder_padding: Some(28),
            ring_color: Some(RgbColor::rgb(148, 178, 255)),
            ring_width: Some(0),
            icon_color: Some(RgbColor::rgb(255, 255, 255)),
            position: WidgetPositionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsernameVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub font_style: Option<super::input::FontStyle>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub font_size: Option<u16>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for UsernameVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            font_family: Some(super::default_google_sans_flex_font_family()),
            font_weight: Some(400),
            font_style: Some(super::input::FontStyle::Normal),
            color: Some(RgbColor::rgba(255, 255, 255, 214)),
            font_size: Some(28),
            position: WidgetPositionConfig::default(),
        }
    }
}

impl super::VisualConfig {
    pub fn avatar_enabled(&self) -> bool {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.enabled)
            .unwrap_or(true)
    }

    pub fn avatar_background_color(&self) -> Option<RgbColor> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.background_color)
            .or(self.avatar_background_color)
    }

    pub fn avatar_size(&self) -> Option<u16> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.size)
            .or(self.avatar_size)
    }

    pub fn avatar_placeholder_padding(&self) -> Option<u16> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.placeholder_padding)
            .or(self.avatar_placeholder_padding)
    }

    pub fn avatar_ring_color(&self) -> Option<RgbColor> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.ring_color)
            .or(self.avatar_ring_color)
    }

    pub fn avatar_ring_width(&self) -> Option<u16> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.ring_width)
            .or(self.avatar_ring_width)
    }

    pub fn avatar_icon_color(&self) -> Option<RgbColor> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.icon_color)
            .or(self.avatar_icon_color)
    }

    pub fn avatar_position(&self) -> WidgetPositionConfig {
        self.avatar
            .as_ref()
            .map(|avatar| avatar.position.clone())
            .unwrap_or_default()
    }

    pub fn username_color(&self) -> Option<RgbColor> {
        self.username
            .as_ref()
            .and_then(|username| username.color)
            .or(self.username_color)
    }

    pub fn username_enabled(&self) -> bool {
        self.username
            .as_ref()
            .and_then(|username| username.enabled)
            .unwrap_or(true)
    }

    pub fn username_font_family(&self) -> Option<&str> {
        self.username
            .as_ref()
            .and_then(|username| username.font_family.as_deref())
    }

    pub fn username_font_weight(&self) -> Option<u16> {
        self.username
            .as_ref()
            .and_then(|username| username.font_weight)
    }

    pub fn username_font_style(&self) -> Option<super::input::FontStyle> {
        self.username
            .as_ref()
            .and_then(|username| username.font_style)
    }

    pub fn username_font_size(&self) -> Option<u16> {
        self.username
            .as_ref()
            .and_then(|username| username.font_size)
            .or(self.username_font_size)
    }

    pub fn username_position(&self) -> WidgetPositionConfig {
        self.username
            .as_ref()
            .map(|username| username.position.clone())
            .unwrap_or_default()
    }
}
