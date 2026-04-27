use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::RgbColor;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundMode {
    Bundled,
    File,
    Gradient,
    Solid,
}

impl BackgroundMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bundled => "bundled",
            Self::File => "file",
            Self::Gradient => "gradient",
            Self::Solid => "solid",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundGradientConfig {
    #[serde(default = "default_gradient_top_left")]
    pub top_left: RgbColor,
    #[serde(default = "default_gradient_top_right")]
    pub top_right: RgbColor,
    #[serde(default = "default_gradient_bottom_left")]
    pub bottom_left: RgbColor,
    #[serde(default = "default_gradient_bottom_right")]
    pub bottom_right: RgbColor,
}

impl Default for BackgroundGradientConfig {
    fn default() -> Self {
        Self {
            top_left: default_gradient_top_left(),
            top_right: default_gradient_top_right(),
            bottom_left: default_gradient_bottom_left(),
            bottom_right: default_gradient_bottom_right(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundConfig {
    #[serde(default)]
    pub mode: Option<BackgroundMode>,
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub outputs: Vec<BackgroundOutputConfig>,
    #[serde(default = "default_background_color")]
    pub color: RgbColor,
    #[serde(default)]
    pub gradient: Option<BackgroundGradientConfig>,
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
            mode: Some(BackgroundMode::Bundled),
            path: None,
            outputs: Vec::new(),
            color: default_background_color(),
            gradient: None,
            blur_radius: default_background_blur_radius(),
            dim_strength: default_background_dim_strength(),
            tint: Some(default_background_tint()),
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
            BackgroundMode::Gradient => None,
            BackgroundMode::Solid => None,
        }
    }

    pub fn resolved_gradient(&self) -> Option<BackgroundGradientConfig> {
        match self.effective_mode() {
            BackgroundMode::Gradient => Some(self.gradient.clone().unwrap_or_default()),
            _ => None,
        }
    }

    pub fn resolved_path_for_output(&self, output_name: Option<&str>) -> Option<PathBuf> {
        output_name
            .and_then(|name| self.outputs.iter().find(|output| output.name == name))
            .map(|output| output.path.clone())
            .or_else(|| self.resolved_path())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundOutputConfig {
    pub name: String,
    pub path: PathBuf,
}

pub fn bundled_background_path() -> PathBuf {
    let background_dir = super::assets::bundled_background_dir();

    for file_name in ["default.jpg", "bg.jpg"] {
        let path = background_dir.join(file_name);
        if path.exists() {
            return path;
        }
    }

    background_dir.join("default.jpg")
}

const fn default_background_color() -> RgbColor {
    RgbColor::rgb(32, 40, 51)
}

const fn default_background_blur_radius() -> u8 {
    12
}

const fn default_background_dim_strength() -> u8 {
    54
}

const fn default_background_tint() -> RgbColor {
    RgbColor::rgba(8, 10, 14, 102)
}

const fn default_background_tint_opacity() -> u8 {
    0
}

const fn default_gradient_top_left() -> RgbColor {
    RgbColor::rgb(171, 92, 255)
}

const fn default_gradient_top_right() -> RgbColor {
    RgbColor::rgb(45, 182, 255)
}

const fn default_gradient_bottom_left() -> RgbColor {
    RgbColor::rgb(99, 219, 255)
}

const fn default_gradient_bottom_right() -> RgbColor {
    RgbColor::rgb(132, 74, 255)
}
