use serde::{Deserialize, Serialize};

use super::RgbColor;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayerAlignment {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayerVerticalAlignment {
    #[default]
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayerMode {
    Solid,
    #[default]
    Blur,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayerStyle {
    #[default]
    Panel,
    Diagonal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum LayerWidth {
    Pixels(u16),
    Keyword(LayerWidthKeyword),
}

impl Default for LayerWidth {
    fn default() -> Self {
        Self::Pixels(560)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LayerWidthKeyword {
    #[serde(rename = "full")]
    Full,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum LayerHeight {
    Pixels(u16),
    Keyword(LayerHeightKeyword),
}

impl Default for LayerHeight {
    fn default() -> Self {
        Self::Keyword(LayerHeightKeyword::Full)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LayerHeightKeyword {
    #[serde(rename = "full")]
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayerVisualConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub mode: Option<LayerMode>,
    #[serde(default)]
    pub style: Option<LayerStyle>,
    #[serde(default)]
    pub alignment: Option<LayerAlignment>,
    #[serde(default)]
    pub width: Option<LayerWidth>,
    #[serde(default)]
    pub height: Option<LayerHeight>,
    #[serde(default)]
    pub vertical_alignment: Option<LayerVerticalAlignment>,
    #[serde(default)]
    pub offset_x: Option<i16>,
    #[serde(default)]
    pub offset_y: Option<i16>,
    #[serde(default)]
    pub left_margin: Option<u16>,
    #[serde(default)]
    pub right_margin: Option<u16>,
    #[serde(default)]
    pub top_margin: Option<u16>,
    #[serde(default)]
    pub bottom_margin: Option<u16>,
    #[serde(default)]
    pub left_padding: Option<u16>,
    #[serde(default)]
    pub right_padding: Option<u16>,
    #[serde(default)]
    pub top_padding: Option<u16>,
    #[serde(default)]
    pub bottom_padding: Option<u16>,
    #[serde(default)]
    pub color: Option<RgbColor>,
    #[serde(default)]
    pub opacity: Option<u8>,
    #[serde(default)]
    pub blur_radius: Option<u8>,
    #[serde(default)]
    pub radius: Option<u16>,
    #[serde(default)]
    pub border_color: Option<RgbColor>,
    #[serde(default)]
    pub border_width: Option<u16>,
}

impl Default for LayerVisualConfig {
    fn default() -> Self {
        Self {
            enabled: Some(false),
            mode: Some(LayerMode::Blur),
            style: Some(LayerStyle::Panel),
            alignment: Some(LayerAlignment::Center),
            width: Some(LayerWidth::default()),
            height: Some(LayerHeight::default()),
            vertical_alignment: Some(LayerVerticalAlignment::Top),
            offset_x: Some(0),
            offset_y: Some(0),
            left_margin: Some(0),
            right_margin: Some(0),
            top_margin: Some(0),
            bottom_margin: Some(0),
            left_padding: Some(0),
            right_padding: Some(0),
            top_padding: Some(0),
            bottom_padding: Some(0),
            color: Some(RgbColor::rgb(8, 10, 14)),
            opacity: Some(42),
            blur_radius: Some(12),
            radius: Some(0),
            border_color: Some(RgbColor::rgb(255, 255, 255)),
            border_width: Some(0),
        }
    }
}

impl super::VisualConfig {
    pub fn layer_enabled(&self) -> bool {
        self.layer
            .as_ref()
            .and_then(|layer| layer.enabled)
            .unwrap_or(false)
    }

    pub fn layer_mode(&self) -> LayerMode {
        self.layer
            .as_ref()
            .and_then(|layer| layer.mode)
            .unwrap_or_default()
    }

    pub fn layer_style(&self) -> LayerStyle {
        self.layer
            .as_ref()
            .and_then(|layer| layer.style)
            .unwrap_or_default()
    }

    pub fn layer_alignment(&self) -> LayerAlignment {
        self.layer
            .as_ref()
            .and_then(|layer| layer.alignment)
            .unwrap_or_default()
    }

    pub fn layer_width(&self) -> Option<u16> {
        match self.layer.as_ref().and_then(|layer| layer.width) {
            Some(LayerWidth::Pixels(width)) => Some(width),
            Some(LayerWidth::Keyword(LayerWidthKeyword::Full)) | None => None,
        }
    }

    pub fn layer_full_width(&self) -> bool {
        matches!(
            self.layer.as_ref().and_then(|layer| layer.width),
            Some(LayerWidth::Keyword(LayerWidthKeyword::Full))
        )
    }

    pub fn layer_height(&self) -> Option<u16> {
        match self.layer.as_ref().and_then(|layer| layer.height) {
            Some(LayerHeight::Pixels(height)) => Some(height),
            Some(LayerHeight::Keyword(LayerHeightKeyword::Full)) | None => None,
        }
    }

    pub fn layer_full_height(&self) -> bool {
        matches!(
            self.layer.as_ref().and_then(|layer| layer.height),
            Some(LayerHeight::Keyword(LayerHeightKeyword::Full))
        )
    }

    pub fn layer_vertical_alignment(&self) -> LayerVerticalAlignment {
        self.layer
            .as_ref()
            .and_then(|layer| layer.vertical_alignment)
            .unwrap_or_default()
    }

    pub fn layer_offset_x(&self) -> Option<i16> {
        self.layer.as_ref().and_then(|layer| layer.offset_x)
    }

    pub fn layer_offset_y(&self) -> Option<i16> {
        self.layer.as_ref().and_then(|layer| layer.offset_y)
    }

    pub fn layer_color(&self) -> Option<RgbColor> {
        self.layer.as_ref().and_then(|layer| layer.color)
    }

    pub fn layer_left_margin(&self) -> Option<u16> {
        self.layer
            .as_ref()
            .and_then(|layer| layer.left_margin.or(layer.left_padding))
    }

    pub fn layer_right_margin(&self) -> Option<u16> {
        self.layer
            .as_ref()
            .and_then(|layer| layer.right_margin.or(layer.right_padding))
    }

    pub fn layer_top_margin(&self) -> Option<u16> {
        self.layer
            .as_ref()
            .and_then(|layer| layer.top_margin.or(layer.top_padding))
    }

    pub fn layer_bottom_margin(&self) -> Option<u16> {
        self.layer
            .as_ref()
            .and_then(|layer| layer.bottom_margin.or(layer.bottom_padding))
    }

    pub fn layer_left_padding(&self) -> Option<u16> {
        self.layer_left_margin()
    }

    pub fn layer_right_padding(&self) -> Option<u16> {
        self.layer_right_margin()
    }

    pub fn layer_top_padding(&self) -> Option<u16> {
        self.layer_top_margin()
    }

    pub fn layer_bottom_padding(&self) -> Option<u16> {
        self.layer_bottom_margin()
    }

    pub fn layer_opacity(&self) -> Option<u8> {
        self.layer.as_ref().and_then(|layer| layer.opacity)
    }

    pub fn layer_blur_radius(&self) -> Option<u8> {
        self.layer.as_ref().and_then(|layer| layer.blur_radius)
    }

    pub fn layer_radius(&self) -> Option<u16> {
        self.layer.as_ref().and_then(|layer| layer.radius)
    }

    pub fn layer_border_color(&self) -> Option<RgbColor> {
        self.layer.as_ref().and_then(|layer| layer.border_color)
    }

    pub fn layer_border_width(&self) -> Option<u16> {
        self.layer.as_ref().and_then(|layer| layer.border_width)
    }
}
