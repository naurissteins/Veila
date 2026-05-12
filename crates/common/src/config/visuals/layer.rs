use serde::{Deserialize, Serialize};

use super::{FontStyle, RgbColor, layout::WidgetPositionConfig};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    #[default]
    Text,
    Icon,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayerVisualConfig {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub kind: Option<LayerKind>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_weight: Option<u16>,
    #[serde(default)]
    pub font_style: Option<FontStyle>,
    #[serde(default)]
    pub font_size: Option<u16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub background_color: Option<RgbColor>,
    #[serde(default)]
    pub width: Option<u16>,
    #[serde(default)]
    pub height: Option<u16>,
    #[serde(default)]
    pub padding: Option<u16>,
    #[serde(default)]
    pub radius: Option<u16>,
    #[serde(default)]
    pub z: Option<i16>,
    #[serde(flatten)]
    pub position: WidgetPositionConfig,
}

impl Default for LayerVisualConfig {
    fn default() -> Self {
        Self {
            name: None,
            enabled: Some(true),
            kind: Some(LayerKind::Text),
            text: None,
            font_family: None,
            font_weight: Some(400),
            font_style: Some(FontStyle::Normal),
            font_size: Some(24),
            color: Some(RgbColor::rgb(255, 255, 255)),
            background_color: None,
            width: None,
            height: None,
            padding: Some(0),
            radius: Some(0),
            z: Some(0),
            position: WidgetPositionConfig {
                halign: Some(super::HorizontalAlign::Center),
                valign: Some(super::VerticalAlign::Center),
                x: Some(0),
                y: Some(0),
                relative_to: None,
            },
        }
    }
}
