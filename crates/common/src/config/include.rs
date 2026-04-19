use std::{
    fs, io,
    path::{Path, PathBuf},
};

use toml::Value;

use crate::error::{Result, VeilaError};

pub(super) fn extract_paths(value: &Value, config_dir: Option<&Path>) -> Result<Vec<PathBuf>> {
    let Some(include) = value.get("include") else {
        return Ok(Vec::new());
    };

    match include {
        Value::String(path) => Ok(vec![resolve_path(path, config_dir)]),
        Value::Array(paths) => paths
            .iter()
            .map(|path| {
                path.as_str()
                    .map(|path| resolve_path(path, config_dir))
                    .ok_or_else(|| invalid_include_error("include entries must be strings"))
            })
            .collect(),
        _ => Err(invalid_include_error(
            "include must be a string or array of strings",
        )),
    }
}

pub(super) fn load_value(path: &Path) -> Result<Option<Value>> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            let mut value = super::parse_toml_value(&raw)?;
            super::remove_config_metadata(&mut value);
            Ok(Some(value))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn resolve_path(path: &str, config_dir: Option<&Path>) -> PathBuf {
    let path = expand_home_path(path);
    if path.is_absolute() {
        return path;
    }

    config_dir.map(|dir| dir.join(&path)).unwrap_or(path)
}

fn expand_home_path(path: &str) -> PathBuf {
    if path == "~" {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(path));
    }

    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }

    PathBuf::from(path)
}

fn invalid_include_error(message: &str) -> VeilaError {
    VeilaError::ConfigIo(io::Error::new(io::ErrorKind::InvalidData, message))
}
