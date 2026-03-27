mod background;
mod color;
mod lock;
#[cfg(test)]
mod tests;
mod visuals;
mod weather;

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::Result;

pub use background::{BackgroundConfig, BackgroundMode};
pub use color::ConfigColor;
pub use lock::LockConfig;
pub use visuals::{
    AvatarVisualConfig, ClockVisualConfig, DateVisualConfig, EyeVisualConfig, InputVisualConfig,
    InputVisualEntry, KeyboardVisualConfig, LayoutVisualConfig, PaletteVisualConfig,
    PlaceholderVisualConfig, StatusVisualConfig, UsernameVisualConfig, VisualConfig,
    WeatherAlignment, WeatherVisualConfig,
};
pub use weather::{GeoCoordinate, WeatherConfig, WeatherUnit};

pub type RgbColor = ConfigColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    pub path: Option<PathBuf>,
    pub config: AppConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub background: BackgroundConfig,
    #[serde(default)]
    pub lock: LockConfig,
    #[serde(default)]
    pub weather: WeatherConfig,
    #[serde(default)]
    pub visuals: VisualConfig,
}

impl AppConfig {
    pub fn from_toml_str(input: &str) -> Result<Self> {
        toml::from_str(input).map_err(Into::into)
    }

    pub fn load(explicit_path: Option<&Path>) -> Result<LoadedConfig> {
        let path = match explicit_path {
            Some(path) => Some(path.to_path_buf()),
            None => default_path(),
        };

        let Some(path) = path else {
            return Ok(LoadedConfig {
                path: None,
                config: Self::default(),
            });
        };

        if !path.exists() {
            if explicit_path.is_some() {
                let _ = fs::File::open(&path)?;
            }

            return Ok(LoadedConfig {
                path: None,
                config: Self::default(),
            });
        }

        let config = Self::load_from_file(&path)?;
        Ok(LoadedConfig {
            path: Some(path),
            config,
        })
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_toml_str(&content)
    }
}

fn default_path() -> Option<PathBuf> {
    let config_root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;

    Some(config_root.join("veila").join("config.toml"))
}
