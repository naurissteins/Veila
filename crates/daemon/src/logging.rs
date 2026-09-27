use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use veila_common::AppConfig;

use crate::DaemonOptions;

pub fn daemon_log_file_path(options: &DaemonOptions) -> Result<Option<PathBuf>> {
    if options.help {
        return Ok(None);
    }

    if let Some(path) = options.log_file_path.as_deref() {
        return Ok(Some(normalize_log_file_path(path)));
    }

    let loaded = AppConfig::load(options.config_path.as_deref())
        .context("failed to load daemon config for log setup")?;
    if !loaded.config.lock.log_to_file {
        return Ok(None);
    }

    Ok(Some(normalize_log_file_path(
        loaded.config.lock.log_file_path.as_path(),
    )))
}

pub(crate) fn normalize_log_file_path(path: &Path) -> PathBuf {
    let Some(raw) = path.to_str() else {
        return path.to_path_buf();
    };

    if raw == "~" {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| path.to_path_buf());
    }

    if let Some(rest) = raw.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }

    path.to_path_buf()
}
