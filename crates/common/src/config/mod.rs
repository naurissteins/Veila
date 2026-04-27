mod assets;
mod background;
mod battery;
mod color;
mod include;
mod lock;
mod now_playing;
#[cfg(test)]
mod tests;
mod visuals;
mod weather;

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use toml::Value;

use crate::error::{Result, VeilaError};
use assets::bundled_theme_dir;

const DEFAULT_THEME_NAME: &str = "default";

pub use background::{
    BackgroundConfig, BackgroundGradientConfig, BackgroundMode, BackgroundOutputConfig,
    BackgroundRadialConfig,
};
pub use battery::BatteryConfig;
pub use color::ConfigColor;
pub use lock::LockConfig;
pub use now_playing::NowPlayingConfig;
pub use visuals::{
    AvatarVisualConfig, BatteryVisualConfig, CapsLockVisualConfig, CenterStackOrder,
    CenterStackStyle, ClockAlignment, ClockFormat, ClockStyle, ClockVisualConfig, DateVisualConfig,
    EyeVisualConfig, FontStyle, InputAlignment, InputVisualConfig, InputVisualEntry,
    KeyboardVisualConfig, LayerAlignment, LayerHeight, LayerHeightKeyword, LayerMode, LayerStyle,
    LayerVerticalAlignment, LayerVisualConfig, LayerWidth, LayerWidthKeyword, LayoutVisualConfig,
    NowPlayingBackgroundConfig, NowPlayingVisualConfig, PaletteVisualConfig,
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
    pub battery: BatteryConfig,
    #[serde(default)]
    pub now_playing: NowPlayingConfig,
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
                config: Self::from_default_layers()?,
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

    fn from_default_layers() -> Result<Self> {
        let mut config_value = default_config_value()?;
        remove_config_metadata(&mut config_value);
        deserialize_toml_value(config_value)
    }

    fn from_toml_str_with_theme_support(input: &str, config_dir: Option<&Path>) -> Result<Self> {
        let mut user_value = parse_toml_value(input)?;
        let theme_name = extract_theme_name(&user_value)?;
        let include_paths = include::extract_paths(&user_value, config_dir)?;
        let mut config_value = default_config_value()?;

        if let Some(theme_name) = theme_name {
            let preset_value = load_theme_value(&theme_name, config_dir)?;
            merge_config_layer(&mut config_value, preset_value);
        }

        for include_path in include_paths {
            if let Some(include_value) = include::load_value(&include_path)? {
                merge_config_layer(&mut config_value, include_value);
            }
        }

        remove_config_metadata(&mut user_value);
        merge_config_layer(&mut config_value, user_value);

        remove_config_metadata(&mut config_value);

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

pub fn default_config_path() -> Option<PathBuf> {
    default_path()
}

pub fn active_theme_name(explicit_config_path: Option<&Path>) -> Result<Option<String>> {
    let path = match explicit_config_path {
        Some(path) => path.to_path_buf(),
        None => match default_path() {
            Some(path) => path,
            None => return Ok(None),
        },
    };

    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)?;
    let value = parse_toml_value(&raw)?;
    extract_theme_name(&value)
}

pub fn active_theme_source_path(explicit_config_path: Option<&Path>) -> Result<Option<PathBuf>> {
    let path = match explicit_config_path {
        Some(path) => path.to_path_buf(),
        None => match default_path() {
            Some(path) => path,
            None => return Ok(None),
        },
    };

    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)?;
    let value = parse_toml_value(&raw)?;
    let Some(theme_name) = extract_theme_name(&value)? else {
        return Ok(None);
    };

    resolve_theme_path(&theme_name, path.parent()).map(Some)
}

pub fn read_theme_source(
    explicit_config_path: Option<&Path>,
    theme: &str,
) -> Result<(PathBuf, String)> {
    validate_theme_name(theme)?;
    let config_dir = config_dir_for_theme_lookup(explicit_config_path);
    let path = resolve_theme_path(theme, config_dir.as_deref())?;
    let raw = fs::read_to_string(&path)?;
    Ok((path, raw))
}

pub fn set_theme_in_config(explicit_path: Option<&Path>, theme: &str) -> Result<PathBuf> {
    validate_theme_name(theme)?;

    let path = match explicit_path {
        Some(path) => path.to_path_buf(),
        None => default_path().ok_or_else(|| {
            VeilaError::ConfigIo(io::Error::new(
                io::ErrorKind::NotFound,
                "failed to resolve default config path",
            ))
        })?,
    };

    resolve_theme_path(theme, path.parent())?;

    let mut config_value = if path.exists() {
        let raw = fs::read_to_string(&path)?;
        parse_toml_value(&raw)?
    } else {
        Value::Table(Default::default())
    };

    let Some(table) = config_value.as_table_mut() else {
        return Err(VeilaError::ConfigIo(io::Error::new(
            io::ErrorKind::InvalidData,
            "top-level config must be a TOML table",
        )));
    };

    table.insert(String::from("theme"), Value::String(theme.to_owned()));

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let encoded = toml::to_string_pretty(&config_value).map_err(|error| {
        VeilaError::ConfigIo(io::Error::other(format!(
            "failed to encode config after setting theme: {error}"
        )))
    })?;
    fs::write(&path, encoded)?;
    Ok(path)
}

pub fn unset_theme_in_config(explicit_path: Option<&Path>) -> Result<(PathBuf, bool)> {
    let path = match explicit_path {
        Some(path) => path.to_path_buf(),
        None => default_path().ok_or_else(|| {
            VeilaError::ConfigIo(io::Error::new(
                io::ErrorKind::NotFound,
                "failed to resolve default config path",
            ))
        })?,
    };

    if !path.exists() {
        return Ok((path, false));
    }

    let raw = fs::read_to_string(&path)?;
    let mut config_value = parse_toml_value(&raw)?;

    let Some(table) = config_value.as_table_mut() else {
        return Err(VeilaError::ConfigIo(io::Error::new(
            io::ErrorKind::InvalidData,
            "top-level config must be a TOML table",
        )));
    };

    if table.remove("theme").is_none() {
        return Ok((path, false));
    }

    let encoded = if table.is_empty() {
        String::new()
    } else {
        toml::to_string_pretty(&config_value).map_err(|error| {
            VeilaError::ConfigIo(io::Error::other(format!(
                "failed to encode config after unsetting theme: {error}"
            )))
        })?
    };
    fs::write(&path, encoded)?;
    Ok((path, true))
}

fn default_path() -> Option<PathBuf> {
    let config_root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;

    Some(config_root.join("veila").join("config.toml"))
}

pub fn active_include_source_paths(explicit_config_path: Option<&Path>) -> Result<Vec<PathBuf>> {
    let path = match explicit_config_path {
        Some(path) => path.to_path_buf(),
        None => match default_path() {
            Some(path) => path,
            None => return Ok(Vec::new()),
        },
    };

    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(&path)?;
    let value = parse_toml_value(&raw)?;
    include::extract_paths(&value, path.parent())
}

fn config_dir_for_theme_lookup(explicit_config_path: Option<&Path>) -> Option<PathBuf> {
    explicit_config_path
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .or_else(|| default_path().and_then(|path| path.parent().map(Path::to_path_buf)))
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
    let path = resolve_theme_path(theme, config_dir)?;
    let raw = fs::read_to_string(path)?;
    parse_toml_value(&raw)
}

fn default_config_value() -> Result<Value> {
    let mut config_value = hardcoded_config_value()?;
    if let Ok(default_theme_value) = load_bundled_theme_value(DEFAULT_THEME_NAME) {
        merge_config_layer(&mut config_value, default_theme_value);
    }
    Ok(config_value)
}

fn hardcoded_config_value() -> Result<Value> {
    Value::try_from(AppConfig::default()).map_err(|error| {
        VeilaError::ConfigIo(io::Error::other(format!(
            "failed to encode hardcoded defaults: {error}"
        )))
    })
}

fn load_bundled_theme_value(theme: &str) -> Result<Value> {
    let path = resolve_bundled_theme_path(theme)?;
    let raw = fs::read_to_string(path)?;
    parse_toml_value(&raw)
}

fn resolve_theme_path(theme: &str, config_dir: Option<&Path>) -> Result<PathBuf> {
    let file_name = format!("{theme}.toml");

    if let Some(config_dir) = config_dir {
        let user_theme_path = config_dir.join("themes").join(&file_name);
        if user_theme_path.exists() {
            return Ok(user_theme_path);
        }
    }

    resolve_bundled_theme_path(theme)
}

fn resolve_bundled_theme_path(theme: &str) -> Result<PathBuf> {
    let bundled_theme_path = bundled_theme_dir().join(format!("{theme}.toml"));
    if bundled_theme_path.exists() {
        return Ok(bundled_theme_path);
    }

    Err(VeilaError::ThemeNotFound(theme.to_owned()))
}

fn merge_config_layer(base: &mut Value, override_value: Value) {
    apply_legacy_visual_override_precedence(base, &override_value);
    merge_toml_values(base, override_value);
}

pub(super) fn remove_config_metadata(value: &mut Value) {
    let Some(table) = value.as_table_mut() else {
        return;
    };
    table.remove("theme");
    table.remove("include");
}

fn apply_legacy_visual_override_precedence(base: &mut Value, override_value: &Value) {
    let Some(override_visuals) = override_value.get("visuals").and_then(Value::as_table) else {
        return;
    };
    let Some(base_visuals) = base.get_mut("visuals").and_then(Value::as_table_mut) else {
        return;
    };

    for (flat_key, section, nested_key) in LEGACY_VISUAL_MAPPINGS {
        if override_visuals.contains_key(*flat_key) {
            remove_nested_visual_value(base_visuals, section, nested_key);
        }
    }
}

fn remove_nested_visual_value(base_visuals: &mut toml::Table, section: &str, nested_key: &str) {
    let Some(nested) = base_visuals.get_mut(section).and_then(Value::as_table_mut) else {
        return;
    };
    nested.remove(nested_key);
}

const LEGACY_VISUAL_MAPPINGS: &[(&str, &str, &str)] = &[
    ("input_opacity", "input", "background_opacity"),
    ("input_font_family", "input", "font_family"),
    ("input_font_weight", "input", "font_weight"),
    ("input_font_style", "input", "font_style"),
    ("input_font_size", "input", "font_size"),
    ("input_center_in_layer", "input", "center_in_layer"),
    ("input_border", "input", "border_color"),
    ("input_border_opacity", "input", "border_opacity"),
    ("input_width", "input", "width"),
    ("input_height", "input", "height"),
    ("input_radius", "input", "radius"),
    ("input_border_width", "input", "border_width"),
    ("input_mask_color", "input", "mask_color"),
    ("avatar_background_color", "avatar", "background_color"),
    ("avatar_size", "avatar", "size"),
    ("avatar_offset_y", "avatar", "offset_y"),
    (
        "avatar_placeholder_padding",
        "avatar",
        "placeholder_padding",
    ),
    ("avatar_icon_color", "avatar", "icon_color"),
    ("avatar_ring_color", "avatar", "ring_color"),
    ("avatar_ring_width", "avatar", "ring_width"),
    ("avatar_background_opacity", "avatar", "background_opacity"),
    ("avatar_gap", "avatar", "gap"),
    ("username_color", "username", "color"),
    ("username_opacity", "username", "opacity"),
    ("username_size", "username", "size"),
    ("username_offset_y", "username", "offset_y"),
    ("username_gap", "username", "gap"),
    ("clock_gap", "clock", "gap"),
    ("clock_font_family", "clock", "font_family"),
    ("clock_font_weight", "clock", "font_weight"),
    ("clock_font_style", "clock", "font_style"),
    ("clock_style", "clock", "style"),
    ("clock_center_in_layer", "clock", "center_in_layer"),
    ("clock_offset_x", "clock", "offset_x"),
    ("clock_offset_y", "clock", "offset_y"),
    ("clock_format", "clock", "format"),
    ("clock_meridiem_size", "clock", "meridiem_size"),
    ("clock_meridiem_offset_x", "clock", "meridiem_offset_x"),
    ("clock_meridiem_offset_y", "clock", "meridiem_offset_y"),
    ("clock_color", "clock", "color"),
    ("clock_opacity", "clock", "opacity"),
    ("clock_size", "clock", "size"),
    ("date_color", "date", "color"),
    ("date_opacity", "date", "opacity"),
    ("date_size", "date", "size"),
    ("placeholder_color", "placeholder", "color"),
    ("placeholder_opacity", "placeholder", "opacity"),
    ("eye_icon_color", "eye", "color"),
    ("eye_icon_opacity", "eye", "opacity"),
    ("keyboard_color", "keyboard", "color"),
    ("keyboard_background_size", "keyboard", "background_size"),
    ("keyboard_opacity", "keyboard", "opacity"),
    ("keyboard_size", "keyboard", "size"),
    ("keyboard_top_offset", "keyboard", "top_offset"),
    ("keyboard_right_offset", "keyboard", "right_offset"),
    ("battery_color", "battery", "color"),
    ("battery_background_color", "battery", "background_color"),
    ("battery_background_size", "battery", "background_size"),
    ("battery_opacity", "battery", "opacity"),
    ("battery_size", "battery", "size"),
    ("battery_top_offset", "battery", "top_offset"),
    ("battery_right_offset", "battery", "right_offset"),
    ("battery_gap", "battery", "gap"),
    ("weather_size", "weather", "size"),
    ("status_color", "status", "color"),
    ("status_opacity", "status", "opacity"),
    ("status_gap", "status", "gap"),
    ("auth_stack_offset", "layout", "auth_stack_offset"),
    ("header_top_offset", "layout", "header_top_offset"),
    ("identity_gap", "layout", "identity_gap"),
    ("center_stack_order", "layout", "center_stack_order"),
    ("center_stack_style", "layout", "center_stack_style"),
    ("foreground", "palette", "foreground"),
    ("muted", "palette", "muted"),
    ("pending", "palette", "pending"),
    ("rejected", "palette", "rejected"),
];

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
