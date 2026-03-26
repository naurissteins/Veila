use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::RgbColor;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundConfig {
    pub path: Option<PathBuf>,
    #[serde(default = "default_background_color")]
    pub color: RgbColor,
    #[serde(default = "default_background_blur_radius")]
    pub blur_radius: u8,
    #[serde(default = "default_background_dim_strength")]
    pub dim_strength: u8,
    #[serde(default)]
    pub tint: Option<RgbColor>,
    #[serde(default = "default_background_tint_opacity")]
    pub tint_opacity: u8,
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            path: None,
            color: default_background_color(),
            blur_radius: default_background_blur_radius(),
            dim_strength: default_background_dim_strength(),
            tint: None,
            tint_opacity: default_background_tint_opacity(),
        }
    }
}

const fn default_background_color() -> RgbColor {
    RgbColor::rgb(8, 12, 20)
}

const fn default_background_blur_radius() -> u8 {
    0
}

const fn default_background_dim_strength() -> u8 {
    34
}

const fn default_background_tint_opacity() -> u8 {
    0
}
