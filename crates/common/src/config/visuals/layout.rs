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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WidgetPositionConfig {
    #[serde(default)]
    pub halign: Option<HorizontalAlign>,
    #[serde(default)]
    pub valign: Option<VerticalAlign>,
    #[serde(default)]
    pub x: Option<i16>,
    #[serde(default)]
    pub y: Option<i16>,
}

impl WidgetPositionConfig {
    pub const fn is_specified(&self) -> bool {
        self.halign.is_some() || self.valign.is_some() || self.x.is_some() || self.y.is_some()
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
            .unwrap_or(self.foreground)
    }

    pub fn muted_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.muted)
            .unwrap_or(self.muted)
    }

    pub fn pending_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.pending)
            .unwrap_or(self.pending)
    }

    pub fn rejected_color(&self) -> RgbColor {
        self.palette
            .as_ref()
            .and_then(|palette| palette.rejected)
            .unwrap_or(self.rejected)
    }
}
