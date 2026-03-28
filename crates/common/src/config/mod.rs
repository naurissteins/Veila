mod background;
mod battery;
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

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use toml::Value;

use crate::error::{Result, VeilaError};

pub use background::{BackgroundConfig, BackgroundMode};
pub use battery::BatteryConfig;
pub use color::ConfigColor;
pub use lock::LockConfig;
pub use visuals::{
    AvatarVisualConfig, BatteryVisualConfig, ClockFormat, ClockVisualConfig, DateVisualConfig,
    EyeVisualConfig, InputVisualConfig, InputVisualEntry, KeyboardVisualConfig, LayoutVisualConfig,
    NowPlayingVisualConfig, PaletteVisualConfig, PlaceholderVisualConfig, StatusVisualConfig,
    UsernameVisualConfig, VisualConfig, WeatherAlignment, WeatherVisualConfig,
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
    pub battery: BatteryConfig,
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
        Self::from_toml_str_with_theme_support(&content, path.parent())
    }

    fn from_toml_str_with_theme_support(input: &str, config_dir: Option<&Path>) -> Result<Self> {
        let mut config_value = parse_toml_value(input)?;
        let theme_name = extract_theme_name(&config_value)?;

        if let Some(theme_name) = theme_name {
            let mut preset_value = load_theme_value(&theme_name, config_dir)?;
            merge_toml_values(&mut preset_value, config_value);
            config_value = preset_value;
        }

        if let Some(table) = config_value.as_table_mut() {
            table.remove("theme");
        }

        deserialize_toml_value(config_value)
    }
}

pub fn bundled_theme_names() -> Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(bundled_theme_dir())? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("toml") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        validate_theme_name(stem)?;
        names.push(stem.to_owned());
    }
    names.sort_unstable();
    Ok(names)
}

fn default_path() -> Option<PathBuf> {
    let config_root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;

    Some(config_root.join("veila").join("config.toml"))
}

fn bundled_theme_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/themes")
}

fn parse_toml_value(input: &str) -> Result<Value> {
    toml::from_str(input).map_err(Into::into)
}

fn deserialize_toml_value<T>(value: Value) -> Result<T>
where
    T: DeserializeOwned,
{
    value.try_into().map_err(Into::into)
}

fn extract_theme_name(value: &Value) -> Result<Option<String>> {
    let Some(theme) = value.get("theme") else {
        return Ok(None);
    };
    let Some(theme) = theme.as_str() else {
        return Err(VeilaError::InvalidThemeName(String::from("<non-string>")));
    };
    let theme = theme.trim();
    if theme.is_empty() {
        return Ok(None);
    }
    validate_theme_name(theme)?;
    Ok(Some(theme.to_owned()))
}

fn validate_theme_name(theme: &str) -> Result<()> {
    if theme
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || matches!(char, '-' | '_'))
    {
        Ok(())
    } else {
        Err(VeilaError::InvalidThemeName(theme.to_owned()))
    }
}

fn load_theme_value(theme: &str, config_dir: Option<&Path>) -> Result<Value> {
    let file_name = format!("{theme}.toml");

    if let Some(config_dir) = config_dir {
        let user_theme_path = config_dir.join("themes").join(&file_name);
        if user_theme_path.exists() {
            let raw = fs::read_to_string(user_theme_path)?;
            return parse_toml_value(&raw);
        }
    }

    let bundled_theme_path = bundled_theme_dir().join(&file_name);
    if bundled_theme_path.exists() {
        let raw = fs::read_to_string(bundled_theme_path)?;
        return parse_toml_value(&raw);
    }

    Err(VeilaError::ThemeNotFound(theme.to_owned()))
}

fn merge_toml_values(base: &mut Value, override_value: Value) {
    match (base, override_value) {
        (Value::Table(base_table), Value::Table(override_table)) => {
            for (key, override_entry) in override_table {
                match base_table.get_mut(&key) {
                    Some(base_entry) => merge_toml_values(base_entry, override_entry),
                    None => {
                        base_table.insert(key, override_entry);
                    }
                }
            }
        }
        (base_slot, override_value) => *base_slot = override_value,
    }
}
