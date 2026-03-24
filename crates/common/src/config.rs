use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::Result;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundConfig {
    pub path: Option<PathBuf>,
    #[serde(default = "default_background_color")]
    pub color: RgbColor,
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            path: None,
            color: default_background_color(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockConfig {
    #[serde(default = "default_lock_acquire_timeout_seconds")]
    pub acquire_timeout_seconds: u64,
    #[serde(default = "default_auth_backoff_base_ms")]
    pub auth_backoff_base_ms: u64,
    #[serde(default = "default_auth_backoff_max_seconds")]
    pub auth_backoff_max_seconds: u64,
    #[serde(default)]
    pub user_hint: Option<String>,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            acquire_timeout_seconds: default_lock_acquire_timeout_seconds(),
            auth_backoff_base_ms: default_auth_backoff_base_ms(),
            auth_backoff_max_seconds: default_auth_backoff_max_seconds(),
            user_hint: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualConfig {
    #[serde(default = "default_panel_color")]
    pub panel: RgbColor,
    #[serde(default = "default_panel_border_color")]
    pub panel_border: RgbColor,
    #[serde(default = "default_input_color")]
    pub input: RgbColor,
    #[serde(default = "default_input_border_color")]
    pub input_border: RgbColor,
    #[serde(default = "default_foreground_color")]
    pub foreground: RgbColor,
    #[serde(default = "default_muted_color")]
    pub muted: RgbColor,
    #[serde(default = "default_focus_color")]
    pub focus: RgbColor,
    #[serde(default = "default_pending_color")]
    pub pending: RgbColor,
    #[serde(default = "default_rejected_color")]
    pub rejected: RgbColor,
}

impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            panel: default_panel_color(),
            panel_border: default_panel_border_color(),
            input: default_input_color(),
            input_border: default_input_border_color(),
            foreground: default_foreground_color(),
            muted: default_muted_color(),
            focus: default_focus_color(),
            pending: default_pending_color(),
            rejected: default_rejected_color(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct RgbColor(pub u8, pub u8, pub u8);

impl RgbColor {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self(red, green, blue)
    }
}

fn default_path() -> Option<PathBuf> {
    let config_root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;

    Some(config_root.join("veila").join("config.toml"))
}

const fn default_background_color() -> RgbColor {
    RgbColor::new(8, 12, 20)
}

const fn default_lock_acquire_timeout_seconds() -> u64 {
    5
}

const fn default_auth_backoff_base_ms() -> u64 {
    1_000
}

const fn default_auth_backoff_max_seconds() -> u64 {
    16
}

const fn default_panel_color() -> RgbColor {
    RgbColor::new(22, 28, 38)
}

const fn default_panel_border_color() -> RgbColor {
    RgbColor::new(74, 86, 110)
}

const fn default_input_color() -> RgbColor {
    RgbColor::new(13, 18, 28)
}

const fn default_input_border_color() -> RgbColor {
    RgbColor::new(92, 108, 146)
}

const fn default_foreground_color() -> RgbColor {
    RgbColor::new(240, 244, 250)
}

const fn default_muted_color() -> RgbColor {
    RgbColor::new(68, 78, 102)
}

const fn default_focus_color() -> RgbColor {
    RgbColor::new(116, 161, 255)
}

const fn default_pending_color() -> RgbColor {
    RgbColor::new(255, 194, 92)
}

const fn default_rejected_color() -> RgbColor {
    RgbColor::new(220, 96, 96)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{AppConfig, RgbColor};

    #[test]
    fn parses_partial_config_with_defaults() {
        let config = AppConfig::from_toml_str(
            r#"
                [background]
                color = [12, 16, 24]
            "#,
        )
        .expect("config should parse");

        assert_eq!(config.lock.acquire_timeout_seconds, 5);
        assert!(config.lock.user_hint.is_none());
        assert_eq!(config.background.color, RgbColor(12, 16, 24));
        assert!(config.background.path.is_none());
    }

    #[test]
    fn loads_config_from_file() {
        let dir = std::env::temp_dir().join(format!("veila-config-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r#"
                [lock]
                acquire_timeout_seconds = 9
                auth_backoff_base_ms = 250
                user_hint = "Type your password"

                [visuals]
                focus = [10, 120, 200]
            "#,
        )
        .expect("config file");

        let loaded = AppConfig::load(Some(&path)).expect("config should load");

        assert_eq!(loaded.path.as_deref(), Some(path.as_path()));
        assert_eq!(loaded.config.lock.acquire_timeout_seconds, 9);
        assert_eq!(loaded.config.lock.auth_backoff_base_ms, 250);
        assert_eq!(
            loaded.config.lock.user_hint.as_deref(),
            Some("Type your password")
        );
        assert_eq!(loaded.config.visuals.focus, RgbColor(10, 120, 200));

        fs::remove_file(path).ok();
        fs::remove_dir(dir).ok();
    }
}
