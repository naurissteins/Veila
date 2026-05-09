use serde::{Deserialize, Serialize};

use super::{RgbColor, layout::WidgetPositionConfig};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackdropMode {
    Solid,
    #[default]
    Blur,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackdropShowWhen {
    #[default]
    Always,
    NowPlaying,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackdropVisualConfig {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub show_when: Option<BackdropShowWhen>,
    #[serde(default)]
    pub mode: Option<BackdropMode>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub blur_strength: Option<u8>,
    #[serde(default)]
    pub radius: Option<u16>,
    #[serde(default)]
    pub border_color: Option<RgbColor>,
    #[serde(default)]
    pub border_width: Option<u16>,
    #[serde(default)]
    pub full_width: Option<bool>,
    #[serde(default)]
    pub full_height: Option<bool>,
    #[serde(default)]
    pub inset_top: Option<u16>,
    #[serde(default)]
    pub inset_bottom: Option<u16>,
    #[serde(default)]
    pub inset_left: Option<u16>,
    #[serde(default)]
    pub inset_right: Option<u16>,
    #[serde(default)]
    pub width: Option<u16>,
    #[serde(default)]
    pub height: Option<u16>,
    #[serde(default)]
    pub z: Option<i16>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for BackdropVisualConfig {
    fn default() -> Self {
        Self {
            name: None,
            enabled: Some(true),
            show_when: Some(BackdropShowWhen::Always),
            mode: Some(BackdropMode::Blur),
            color: Some(RgbColor::rgba(8, 10, 14, 107)),
            blur_strength: Some(12),
            radius: Some(0),
            border_color: Some(RgbColor::rgb(255, 255, 255)),
            border_width: Some(0),
            full_width: Some(false),
            full_height: Some(false),
            inset_top: Some(0),
            inset_bottom: Some(0),
            inset_left: Some(0),
            inset_right: Some(0),
            width: Some(560),
            height: Some(600),
            z: Some(0),
            position: WidgetPositionConfig {
                halign: Some(super::HorizontalAlign::Center),
                valign: Some(super::VerticalAlign::Top),
                x: Some(0),
                y: Some(0),
                relative_to: None,
            },
        }
    }
}
