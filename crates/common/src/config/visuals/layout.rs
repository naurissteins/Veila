use serde::{Deserialize, Serialize};

use super::RgbColor;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum HorizontalAlign {
    #[default]
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "left")]
    Left,
    #[serde(rename = "right")]
    Right,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerticalAlign {
    #[default]
    #[serde(rename = "top")]
    Top,
    #[serde(rename = "center")]
    Center,
    #[serde(rename = "bottom")]
    Bottom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WidgetPositionConfig {
    #[serde(default)]
    pub halign: Option<HorizontalAlign>,
    #[serde(default)]
    pub valign: Option<VerticalAlign>,
    #[serde(default)]
    pub x: Option<i16>,
    #[serde(default)]
    pub y: Option<i16>,
    #[serde(default)]
    pub relative_to: Option<String>,
}

impl WidgetPositionConfig {
    pub fn is_specified(&self) -> bool {
        self.halign.is_some()
            || self.valign.is_some()
            || self.x.is_some()
            || self.y.is_some()
            || self.relative_to.is_some()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaletteVisualConfig {
    #[serde(default)]
    pub foreground: Option<RgbColor>,
    #[serde(default)]
    pub muted: Option<RgbColor>,
    #[serde(default)]
    pub pending: Option<RgbColor>,
    #[serde(default)]
    pub rejected: Option<RgbColor>,
}

impl super::VisualConfig {
    pub fn foreground_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.foreground)
            .unwrap_or_else(super::default_foreground_color)
    }

    pub fn muted_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.muted)
            .unwrap_or_else(super::default_muted_color)
    }

    pub fn pending_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.pending)
            .unwrap_or_else(super::default_pending_color)
    }

    pub fn rejected_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.rejected)
            .unwrap_or_else(super::default_rejected_color)
    }
}
