mod color;

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::Result;

pub use color::ConfigColor;
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
    #[serde(default)]
    pub avatar_path: Option<PathBuf>,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            acquire_timeout_seconds: default_lock_acquire_timeout_seconds(),
            auth_backoff_base_ms: default_auth_backoff_base_ms(),
            auth_backoff_max_seconds: default_auth_backoff_max_seconds(),
            user_hint: None,
            avatar_path: None,
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
    #[serde(default = "default_input_radius")]
    pub input_radius: u16,
    #[serde(default = "default_foreground_color")]
    pub foreground: RgbColor,
    #[serde(default = "default_muted_color")]
    pub muted: RgbColor,
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
            input_radius: default_input_radius(),
            foreground: default_foreground_color(),
            muted: default_muted_color(),
            pending: default_pending_color(),
            rejected: default_rejected_color(),
        }
    }
}

fn default_path() -> Option<PathBuf> {
    let config_root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;

    Some(config_root.join("veila").join("config.toml"))
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
    RgbColor::rgb(22, 28, 38)
}

const fn default_panel_border_color() -> RgbColor {
    RgbColor::rgb(74, 86, 110)
}

const fn default_input_color() -> RgbColor {
    RgbColor::rgb(13, 18, 28)
}

const fn default_input_border_color() -> RgbColor {
    RgbColor::rgb(92, 108, 146)
}

const fn default_input_radius() -> u16 {
    32
}

const fn default_foreground_color() -> RgbColor {
    RgbColor::rgb(240, 244, 250)
}

const fn default_muted_color() -> RgbColor {
    RgbColor::rgb(68, 78, 102)
}

const fn default_pending_color() -> RgbColor {
    RgbColor::rgb(255, 194, 92)
}

const fn default_rejected_color() -> RgbColor {
    RgbColor::rgb(220, 96, 96)
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
        assert!(config.lock.avatar_path.is_none());
        assert_eq!(config.background.color, RgbColor::rgb(12, 16, 24));
        assert!(config.background.path.is_none());
        assert_eq!(config.background.blur_radius, 0);
        assert_eq!(config.background.dim_strength, 34);
        assert!(config.background.tint.is_none());
        assert_eq!(config.background.tint_opacity, 0);
        assert_eq!(config.visuals.input_radius, 32);
    }

    #[test]
    fn loads_config_from_file() {
        let dir = std::env::temp_dir().join(format!("veila-config-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r##"
                [background]
                blur_radius = 6
                dim_strength = 40
                tint = "#080A0E99"
                tint_opacity = 12

                [lock]
                acquire_timeout_seconds = 9
                auth_backoff_base_ms = 250
                user_hint = "Type your password"
                avatar_path = "/tmp/avatar.png"

                [visuals]
                panel = "rgba(24, 30, 42, 0.82)"
                input_radius = 20
            "##,
        )
        .expect("config file");

        let loaded = AppConfig::load(Some(&path)).expect("config should load");

        assert_eq!(loaded.path.as_deref(), Some(path.as_path()));
        assert_eq!(loaded.config.lock.acquire_timeout_seconds, 9);
        assert_eq!(loaded.config.lock.auth_backoff_base_ms, 250);
        assert_eq!(
            loaded.config.lock.avatar_path.as_deref(),
            Some(std::path::Path::new("/tmp/avatar.png"))
        );
        assert_eq!(
            loaded.config.lock.user_hint.as_deref(),
            Some("Type your password")
        );
        assert_eq!(loaded.config.background.blur_radius, 6);
        assert_eq!(loaded.config.background.dim_strength, 40);
        assert_eq!(
            loaded.config.background.tint,
            Some(RgbColor::rgba(8, 10, 14, 153))
        );
        assert_eq!(loaded.config.background.tint_opacity, 12);
        assert_eq!(loaded.config.visuals.panel, RgbColor::rgba(24, 30, 42, 209));
        assert_eq!(loaded.config.visuals.input_radius, 20);

        fs::remove_file(path).ok();
        fs::remove_dir(dir).ok();
    }
}
