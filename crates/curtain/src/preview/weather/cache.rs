use anyhow::{Context, Result};
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};
use veila_common::{AppConfig, WeatherSnapshot};

pub(super) fn load_cached_preview_weather_snapshot(
    config: &AppConfig,
    location: Option<&str>,
) -> Result<Option<WeatherSnapshot>> {
    let cache_root = preview_weather_cache_root()?;
    load_cached_preview_weather_snapshot_from(config, &cache_root, location)
}

fn load_cached_preview_weather_snapshot_from(
    config: &AppConfig,
    cache_root: &Path,
    location: Option<&str>,
) -> Result<Option<WeatherSnapshot>> {
    let Some((latitude, longitude)) =
        cached_preview_coordinates_from(config, cache_root, location)?
    else {
        return Ok(None);
    };
    let cache_path = preview_weather_cache_path_for_coordinates(cache_root, latitude, longitude);
    let Ok(raw) = fs::read_to_string(&cache_path) else {
        return Ok(None);
    };
    serde_json::from_str(&raw)
        .map(Some)
        .context("failed to parse cached preview weather snapshot")
}

fn cached_preview_coordinates_from(
    config: &AppConfig,
    cache_root: &Path,
    location: Option<&str>,
) -> Result<Option<(f64, f64)>> {
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
) -> Result<Option<(f64, f64)>> {
    let cache_path = preview_weather_location_cache_path(cache_root, location);
    let Ok(raw) = fs::read_to_string(&cache_path) else {
        return Ok(None);
    };
    let entry: PreviewGeocodedLocationCache = serde_json::from_str(&raw)
        .context("failed to parse cached preview geocoded weather coordinates")?;
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

fn preview_weather_cache_root() -> Result<PathBuf> {
    let root = veila_renderer::cache::resolve_cache_root(None)
        .context("failed to resolve XDG cache directory")?;
    Ok(root.join("weather"))
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PreviewGeocodedLocationCache {
    latitude: f64,
    longitude: f64,
}

#[cfg(test)]
mod tests;
