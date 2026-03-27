use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::RgbColor;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundMode {
    Bundled,
    File,
    Solid,
}

impl BackgroundMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bundled => "bundled",
            Self::File => "file",
            Self::Solid => "solid",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundConfig {
    #[serde(default)]
    pub mode: Option<BackgroundMode>,
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
            mode: None,
            path: None,
            color: default_background_color(),
            blur_radius: default_background_blur_radius(),
            dim_strength: default_background_dim_strength(),
            tint: None,
            tint_opacity: default_background_tint_opacity(),
        }
    }
}

impl BackgroundConfig {
    pub fn effective_mode(&self) -> BackgroundMode {
        match self.mode {
            Some(mode) => mode,
            None if self.path.is_some() => BackgroundMode::File,
            None => BackgroundMode::Bundled,
        }
    }

    pub fn resolved_path(&self) -> Option<PathBuf> {
        match self.effective_mode() {
            BackgroundMode::Bundled => Some(bundled_background_path()),
            BackgroundMode::File => self.path.clone(),
            BackgroundMode::Solid => None,
        }
    }
}

pub fn bundled_background_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/bg/abstract-blur-blue.jpg"
    ))
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
