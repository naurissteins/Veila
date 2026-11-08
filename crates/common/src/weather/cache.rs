use crate::{AppConfig, WeatherSnapshot};
use std::io::Read;
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};
fn bounded_read(path: &Path) -> std::io::Result<String> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NONBLOCK)
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > 4096 {
        return Err(std::io::Error::other("invalid weather cache file"));
    }
    let mut text = String::new();
    file.take(4097).read_to_string(&mut text)?;
    if text.len() > 4096 {
        return Err(std::io::Error::other("weather cache exceeds limit"));
    }
    Ok(text)
}

pub fn load_snapshot_from(
    config: &AppConfig,
    cache_root: &Path,
    location: Option<&str>,
) -> std::io::Result<Option<WeatherSnapshot>> {
    let Some((latitude, longitude)) =
        cached_preview_coordinates_from(config, cache_root, location)?
    else {
        return Ok(None);
    };
    let cache_path = preview_weather_cache_path_for_coordinates(cache_root, latitude, longitude);
    let Ok(raw) = bounded_read(&cache_path) else {
        return Ok(None);
    };
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(std::io::Error::other)
}

fn cached_preview_coordinates_from(
    config: &AppConfig,
    cache_root: &Path,
    location: Option<&str>,
) -> std::io::Result<Option<(f64, f64)>> {
    if location.is_none()
        && let Some((latitude, longitude)) = config.weather.clone().coordinates()
    {
        return Ok(Some((latitude, longitude)));
    }

    let Some(location) = location
        .map(normalize_preview_location)
        .or_else(|| config.weather.normalized_location())
    else {
        return Ok(None);
    };
    load_cached_preview_coordinates(cache_root, &location)
}

fn normalize_preview_location(location: &str) -> String {
    location.trim().to_string()
}

fn load_cached_preview_coordinates(
    cache_root: &Path,
    location: &str,
) -> std::io::Result<Option<(f64, f64)>> {
    let cache_path = preview_weather_location_cache_path(cache_root, location);
    let Ok(raw) = bounded_read(&cache_path) else {
        return Ok(None);
    };
    let entry: PreviewGeocodedLocationCache =
        serde_json::from_str(&raw).map_err(std::io::Error::other)?;
    Ok(Some((entry.latitude, entry.longitude)))
}

fn preview_weather_cache_path_for_coordinates(
    cache_root: &Path,
    latitude: f64,
    longitude: f64,
) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    latitude.to_bits().hash(&mut hasher);
    longitude.to_bits().hash(&mut hasher);
    cache_root.join(format!("{:016x}.json", hasher.finish()))
}

fn preview_weather_location_cache_path(cache_root: &Path, location: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    location.trim().to_lowercase().hash(&mut hasher);
    cache_root.join(format!("location-{:016x}.json", hasher.finish()))
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PreviewGeocodedLocationCache {
    latitude: f64,
    longitude: f64,
}
#[cfg(test)]
mod tests;
