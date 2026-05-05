use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use super::RgbColor;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundMode {
    Bundled,
    File,
    Gradient,
    Layered,
    Radial,
    Solid,
}

impl BackgroundMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bundled => "bundled",
            Self::File => "file",
            Self::Gradient => "gradient",
            Self::Layered => "layered",
            Self::Radial => "radial",
            Self::Solid => "solid",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundScaling {
    #[default]
    Fill,
    Fit,
    Center,
    Tile,
    Stretch,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundSlideshowOrder {
    #[default]
    Sequence,
    Random,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundSlideshowMode {
    #[default]
    Timed,
    LockOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundSlideshowConfig {
    #[serde(default = "default_background_slideshow_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub directory: Option<PathBuf>,
    #[serde(default)]
    pub files: Vec<PathBuf>,
    #[serde(default)]
    pub order: BackgroundSlideshowOrder,
    #[serde(default)]
    pub mode: BackgroundSlideshowMode,
    #[serde(default = "default_background_slideshow_change_every_seconds")]
    pub change_every_seconds: u64,
}

impl Default for BackgroundSlideshowConfig {
    fn default() -> Self {
        Self {
            enabled: default_background_slideshow_enabled(),
            directory: None,
            files: Vec::new(),
            order: BackgroundSlideshowOrder::Sequence,
            mode: BackgroundSlideshowMode::Timed,
            change_every_seconds: default_background_slideshow_change_every_seconds(),
        }
    }
}

impl BackgroundSlideshowConfig {
    pub fn has_sources(&self) -> bool {
        self.directory.is_some() || !self.files.is_empty()
    }

    pub fn change_interval(&self) -> Duration {
        Duration::from_secs(self.change_every_seconds.max(1))
    }

    pub const fn rotates_while_locked(&self) -> bool {
        matches!(self.mode, BackgroundSlideshowMode::Timed)
    }

    pub fn candidate_paths(&self) -> io::Result<Vec<PathBuf>> {
        let mut paths = Vec::new();

        for path in &self.files {
            let path = expand_home_path(path);
            if path.is_file() && !paths.contains(&path) {
                paths.push(path);
            }
        }

        if let Some(directory) = self.directory.as_deref().map(expand_home_path) {
            let mut entries: Vec<_> = fs::read_dir(directory)?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_file() && supported_slideshow_path(path))
                .collect();
            entries.sort_unstable();

            for path in entries {
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
        }

        Ok(paths)
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
pub struct BackgroundRadialConfig {
    #[serde(default = "default_radial_center_color")]
    pub center: RgbColor,
    #[serde(default = "default_radial_edge_color")]
    pub edge: RgbColor,
    #[serde(default = "default_radial_center_x")]
    pub center_x: u8,
    #[serde(default = "default_radial_center_y")]
    pub center_y: u8,
    #[serde(default = "default_radial_radius")]
    pub radius: u8,
}

impl Default for BackgroundRadialConfig {
    fn default() -> Self {
        Self {
            center: default_radial_center_color(),
            edge: default_radial_edge_color(),
            center_x: default_radial_center_x(),
            center_y: default_radial_center_y(),
            radius: default_radial_radius(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundLayeredBaseMode {
    Gradient,
    Radial,
    Solid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundLayeredBaseConfig {
    #[serde(default)]
    pub mode: Option<BackgroundLayeredBaseMode>,
    #[serde(default = "default_background_color")]
    pub color: RgbColor,
    #[serde(default)]
    pub gradient: Option<BackgroundGradientConfig>,
    #[serde(default)]
    pub radial: Option<BackgroundRadialConfig>,
}

impl Default for BackgroundLayeredBaseConfig {
    fn default() -> Self {
        Self {
            mode: Some(BackgroundLayeredBaseMode::Gradient),
            color: default_background_color(),
            gradient: Some(BackgroundGradientConfig::default()),
            radial: Some(BackgroundRadialConfig::default()),
        }
    }
}

impl BackgroundLayeredBaseConfig {
    pub fn effective_mode(&self) -> BackgroundLayeredBaseMode {
        self.mode.unwrap_or(BackgroundLayeredBaseMode::Gradient)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundLayeredBlobConfig {
    #[serde(default = "default_layered_blob_color")]
    pub color: RgbColor,
    #[serde(default = "default_layered_blob_opacity")]
    pub opacity: u8,
    #[serde(default = "default_layered_blob_x")]
    pub x: u8,
    #[serde(default = "default_layered_blob_y")]
    pub y: u8,
    #[serde(default = "default_layered_blob_size")]
    pub size: u8,
}

impl Default for BackgroundLayeredBlobConfig {
    fn default() -> Self {
        Self {
            color: default_layered_blob_color(),
            opacity: default_layered_blob_opacity(),
            x: default_layered_blob_x(),
            y: default_layered_blob_y(),
            size: default_layered_blob_size(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BackgroundLayeredConfig {
    #[serde(default)]
    pub base: BackgroundLayeredBaseConfig,
    #[serde(default)]
    pub blobs: Vec<BackgroundLayeredBlobConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundConfig {
    #[serde(default)]
    pub mode: Option<BackgroundMode>,
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub outputs: Vec<BackgroundOutputConfig>,
    #[serde(default)]
    pub slideshow: Option<BackgroundSlideshowConfig>,
    #[serde(default = "default_background_color")]
    pub color: RgbColor,
    #[serde(default)]
    pub scaling: BackgroundScaling,
    #[serde(default)]
    pub gradient: Option<BackgroundGradientConfig>,
    #[serde(default)]
    pub layered: Option<BackgroundLayeredConfig>,
    #[serde(default)]
    pub radial: Option<BackgroundRadialConfig>,
    #[serde(default = "default_background_blur_radius")]
    pub blur_radius: u8,
    #[serde(default = "default_background_dim_strength")]
    pub dim_strength: u8,
    #[serde(default)]
    pub tint: Option<RgbColor>,
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            mode: Some(BackgroundMode::Gradient),
            path: None,
            outputs: Vec::new(),
            slideshow: None,
            color: default_background_color(),
            scaling: BackgroundScaling::Fill,
            gradient: Some(BackgroundGradientConfig::default()),
            layered: Some(BackgroundLayeredConfig::default()),
            radial: Some(BackgroundRadialConfig::default()),
            blur_radius: default_background_blur_radius(),
            dim_strength: default_background_dim_strength(),
            tint: None,
        }
    }
}

impl BackgroundConfig {
    pub fn effective_mode(&self) -> BackgroundMode {
        match self.mode {
            Some(BackgroundMode::Bundled) | Some(BackgroundMode::Gradient) => {
                BackgroundMode::Gradient
            }
            Some(BackgroundMode::Layered) => BackgroundMode::Layered,
            Some(BackgroundMode::Radial) => BackgroundMode::Radial,
            Some(mode) => mode,
            None if self.path.is_some() || self.slideshow_enabled() => BackgroundMode::File,
            None => BackgroundMode::Gradient,
        }
    }

    pub fn resolved_path(&self) -> Option<PathBuf> {
        match self.effective_mode() {
            BackgroundMode::File => self.path.as_deref().map(expand_home_path),
            BackgroundMode::Gradient => None,
            BackgroundMode::Layered => None,
            BackgroundMode::Radial => None,
            BackgroundMode::Solid => None,
            BackgroundMode::Bundled => None,
        }
    }

    pub fn resolved_gradient(&self) -> Option<BackgroundGradientConfig> {
        match self.effective_mode() {
            BackgroundMode::Gradient => Some(self.gradient.clone().unwrap_or_default()),
            _ => None,
        }
    }

    pub fn resolved_radial(&self) -> Option<BackgroundRadialConfig> {
        match self.effective_mode() {
            BackgroundMode::Radial => Some(self.radial.clone().unwrap_or_default()),
            _ => None,
        }
    }

    pub fn resolved_layered(&self) -> Option<BackgroundLayeredConfig> {
        match self.effective_mode() {
            BackgroundMode::Layered => Some(self.layered.clone().unwrap_or_default()),
            _ => None,
        }
    }

    pub fn resolved_path_for_output(&self, output_name: Option<&str>) -> Option<PathBuf> {
        output_name
            .and_then(|name| self.outputs.iter().find(|output| output.name == name))
            .map(|output| expand_home_path(&output.path))
            .or_else(|| self.resolved_path())
    }

    pub fn slideshow_enabled(&self) -> bool {
        self.slideshow
            .as_ref()
            .is_some_and(|slideshow| slideshow.enabled && slideshow.has_sources())
    }

    pub fn resolved_slideshow_paths(&self) -> io::Result<Vec<PathBuf>> {
        self.slideshow
            .as_ref()
            .filter(|slideshow| slideshow.enabled && slideshow.has_sources())
            .map(BackgroundSlideshowConfig::candidate_paths)
            .transpose()
            .map(|paths| paths.unwrap_or_default())
    }

    pub fn resolved_slideshow_initial_path(&self) -> io::Result<Option<PathBuf>> {
        self.resolved_slideshow_initial_path_with_seed(slideshow_seed())
    }

    pub fn resolved_slideshow_initial_path_with_seed(
        &self,
        seed: u64,
    ) -> io::Result<Option<PathBuf>> {
        let paths = self.resolved_slideshow_paths()?;
        if paths.is_empty() {
            return Ok(None);
        }

        let index = match self.slideshow.as_ref().map(|slideshow| slideshow.order) {
            Some(BackgroundSlideshowOrder::Random) => (seed as usize) % paths.len(),
            _ => 0,
        };

        Ok(paths.get(index).cloned())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundOutputConfig {
    pub name: String,
    pub path: PathBuf,
}

const fn default_background_color() -> RgbColor {
    RgbColor::rgb(32, 40, 51)
}

const fn default_background_slideshow_enabled() -> bool {
    true
}

const fn default_background_slideshow_change_every_seconds() -> u64 {
    300
}

const fn default_background_blur_radius() -> u8 {
    0
}

const fn default_background_dim_strength() -> u8 {
    54
}

const fn default_gradient_top_left() -> RgbColor {
    RgbColor::rgb(168, 91, 255)
}

const fn default_gradient_top_right() -> RgbColor {
    RgbColor::rgb(57, 184, 255)
}

const fn default_gradient_bottom_left() -> RgbColor {
    RgbColor::rgb(111, 226, 255)
}

const fn default_gradient_bottom_right() -> RgbColor {
    RgbColor::rgb(111, 76, 255)
}

const fn default_radial_center_color() -> RgbColor {
    RgbColor::rgb(111, 226, 255)
}

const fn default_radial_edge_color() -> RgbColor {
    RgbColor::rgb(111, 76, 255)
}

const fn default_radial_center_x() -> u8 {
    50
}

const fn default_radial_center_y() -> u8 {
    50
}

const fn default_radial_radius() -> u8 {
    100
}

const fn default_layered_blob_color() -> RgbColor {
    RgbColor::rgb(255, 255, 255)
}

const fn default_layered_blob_opacity() -> u8 {
    18
}

const fn default_layered_blob_x() -> u8 {
    50
}

const fn default_layered_blob_y() -> u8 {
    50
}

const fn default_layered_blob_size() -> u8 {
    36
}

fn supported_slideshow_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase()),
        Some(extension) if matches!(extension.as_str(), "jpg" | "jpeg" | "png" | "webp")
    )
}

fn expand_home_path(path: &Path) -> PathBuf {
    if path == Path::new("~") {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| path.to_path_buf());
    }

    if let Ok(rest) = path.strip_prefix("~")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }

    path.to_path_buf()
}

fn slideshow_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0)
}
