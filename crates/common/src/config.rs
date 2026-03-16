use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// User-facing configuration shared across Kwylock components.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub background: BackgroundConfig,
    #[serde(default)]
    pub lock: LockConfig,
}

impl AppConfig {
    /// Parses configuration from a TOML string.
    pub fn from_toml_str(input: &str) -> Result<Self> {
        toml::from_str(input).map_err(Into::into)
    }
}

/// Background settings for the secure lock scene.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BackgroundConfig {
    pub path: Option<PathBuf>,
}

/// Lock timing and user-visible prompt settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockConfig {
    pub timeout_seconds: u64,
    pub user_hint: Option<String>,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 300,
            user_hint: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn parses_partial_config_with_defaults() {
        let config = AppConfig::from_toml_str(
            r#"
                [background]
                path = "/tmp/background.png"
            "#,
        )
        .expect("config should parse");

        assert_eq!(config.lock.timeout_seconds, 300);
        assert_eq!(
            config.background.path.as_deref(),
            Some(std::path::Path::new("/tmp/background.png"))
        );
    }
}
