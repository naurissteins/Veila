use serde::{Deserialize, Serialize};

use super::RgbColor;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvatarVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub offset_y: Option<i16>,
    #[serde(default)]
    pub gap: Option<u16>,
    #[serde(default)]
    pub background_color: Option<RgbColor>,
    #[serde(default)]
    pub background_opacity: Option<u8>,
    #[serde(default)]
    pub placeholder_padding: Option<u16>,
    #[serde(default)]
    pub ring_color: Option<RgbColor>,
    #[serde(default)]
    pub ring_width: Option<u16>,
    #[serde(default)]
    pub icon_color: Option<RgbColor>,
}

impl Default for AvatarVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            size: Some(192),
            offset_y: Some(0),
            gap: Some(24),
            background_color: Some(RgbColor::rgb(255, 255, 255)),
            background_opacity: Some(6),
            placeholder_padding: Some(28),
            ring_color: Some(RgbColor::rgb(148, 178, 255)),
            ring_width: Some(0),
            icon_color: Some(RgbColor::rgb(255, 255, 255)),
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
    pub opacity: Option<u8>,
    #[serde(default)]
    pub size: Option<u16>,
    #[serde(default)]
    pub offset_y: Option<i16>,
    #[serde(default)]
    pub gap: Option<u16>,
}

impl Default for UsernameVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            font_family: Some(super::default_google_sans_flex_font_family()),
            font_weight: Some(400),
            font_style: Some(super::input::FontStyle::Normal),
            color: Some(RgbColor::rgb(255, 255, 255)),
            opacity: Some(84),
            size: Some(4),
            offset_y: Some(0),
            gap: Some(28),
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

    pub fn avatar_gap(&self) -> Option<u16> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.gap)
            .or(self.avatar_gap)
    }

    pub fn avatar_offset_y(&self) -> Option<i16> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.offset_y)
            .or(self.avatar_offset_y)
    }

    pub fn avatar_background_opacity(&self) -> Option<u8> {
        self.avatar
            .as_ref()
            .and_then(|avatar| avatar.background_opacity)
            .or(self.avatar_background_opacity)
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

    pub fn username_opacity(&self) -> Option<u8> {
        self.username
            .as_ref()
            .and_then(|username| username.opacity)
            .or(self.username_opacity)
    }

    pub fn username_size(&self) -> Option<u16> {
        self.username
            .as_ref()
            .and_then(|username| username.size)
            .or(self.username_size)
    }

    pub fn username_gap(&self) -> Option<u16> {
        self.username
            .as_ref()
            .and_then(|username| username.gap)
            .or(self.username_gap)
    }

    pub fn username_offset_y(&self) -> Option<i16> {
        self.username
            .as_ref()
            .and_then(|username| username.offset_y)
            .or(self.username_offset_y)
    }
}
