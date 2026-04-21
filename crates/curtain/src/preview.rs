use anyhow::{Context, Result};
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};
use time::{OffsetDateTime, UtcOffset};
use veila_common::{
    AppConfig, BatterySnapshot, ConfigColor, NowPlayingSnapshot, WeatherCondition, WeatherSnapshot,
};
use veila_renderer::{
    ClearColor, FrameSize, SoftwareBuffer,
    background::{BackgroundAsset, BackgroundTreatment},
};
use veila_ui::{ShellState, ShellTheme};

use crate::{CurtainOptions, PreviewClockTime};

const DEFAULT_PREVIEW_SIZE: FrameSize = FrameSize::new(2560, 1440);

pub(crate) fn render_preview(options: CurtainOptions) -> Result<()> {
    let output_path = options
        .preview_png
        .clone()
        .context("preview mode requires --preview-png <path> or --preview-png=<path>")?;
    let preview_size = options.preview_size.unwrap_or(DEFAULT_PREVIEW_SIZE);
    let loaded = AppConfig::load(options.config_path.as_deref())
        .context("failed to load config for preview rendering")?;
    let config = loaded.config;
    let weather_location = preview_weather_location(&options, &config);
    let preview_username = preview_username(&options, &config);
    let weather_snapshot = preview_weather_snapshot(&options, &config, weather_location.as_deref());
    let battery_snapshot = preview_battery_snapshot(&options, &config);
    let now_playing_snapshot = options.now_playing_snapshot.or_else(|| {
        preview_now_playing_snapshot(
            options.preview_title.clone(),
            options.preview_artist.clone(),
            options.preview_artwork.clone(),
        )
    });

    let treatment = BackgroundTreatment {
        blur_radius: config.background.blur_radius,
        dim_strength: config.background.dim_strength,
        tint: config.background.tint.map(to_clear_color),
        tint_opacity: config.background.tint_opacity,
    };
    let background = BackgroundAsset::load(
        config.background.resolved_path().as_deref(),
        to_clear_color(config.background.color),
        treatment,
    )
    .context("failed to load preview background")?;
    let mut buffer = background
        .render(preview_size)
        .context("failed to render preview background")?;

    let shell = ShellState::new_with_username_and_widgets(
        ShellTheme::from_config(&config),
        config.lock.user_hint.clone(),
        preview_username,
        config.lock.avatar_path.clone(),
        config.lock.show_username,
        weather_location,
        weather_snapshot,
        config.weather.unit,
        battery_snapshot,
        now_playing_snapshot,
    );
    let mut shell = shell;
    if let Some(preview_time) = options.preview_time {
        shell.set_preview_time(preview_clock_datetime(preview_time));
    }
    shell.set_keyboard_layout_label(Some(String::from("EN")));
    render_shell(&shell, &mut buffer);
    buffer
        .save_png(&output_path)
        .with_context(|| format!("failed to save preview PNG to {}", output_path.display()))?;

    tracing::info!(
        path = %output_path.display(),
        width = preview_size.width,
        height = preview_size.height,
        "rendered curtain preview PNG"
    );
    Ok(())
}

fn render_shell(shell: &ShellState, buffer: &mut SoftwareBuffer) {
    shell.render_overlay(buffer);
}

fn to_clear_color(color: ConfigColor) -> ClearColor {
    ClearColor::rgba(color.0, color.1, color.2, color.3)
}

fn preview_weather_snapshot(
    options: &CurtainOptions,
    config: &AppConfig,
    location: Option<&str>,
) -> Option<WeatherSnapshot> {
    if let Some(snapshot) = options.weather_snapshot.clone() {
        return Some(snapshot);
    }

    if let Some(snapshot) = preview_weather_override_snapshot(options) {
        return Some(snapshot);
    }

    if !config.weather.enabled && location.is_none() {
        return None;
    }

    location?;
    if let Some(snapshot) = load_cached_preview_weather_snapshot(config, location)
        .ok()
        .flatten()
    {
        return Some(snapshot);
    }
    Some(WeatherSnapshot {
        temperature_celsius: 21,
        condition: preview_weather_condition_now(),
        fetched_at_unix: 0,
    })
}

fn preview_weather_location(options: &CurtainOptions, config: &AppConfig) -> Option<String> {
    options
        .preview_weather_location
        .clone()
        .or_else(|| config.weather.location.clone())
}

fn preview_username(options: &CurtainOptions, config: &AppConfig) -> Option<String> {
    options
        .preview_username
        .clone()
        .or_else(|| config.lock.username.clone())
}

fn preview_weather_override_snapshot(options: &CurtainOptions) -> Option<WeatherSnapshot> {
    let temperature_celsius = options.preview_weather_temperature_celsius?;
    Some(WeatherSnapshot {
        temperature_celsius,
        condition: options
            .preview_weather_condition
            .unwrap_or_else(preview_weather_condition_now),
        fetched_at_unix: 0,
    })
}

fn load_cached_preview_weather_snapshot(
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
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .context("failed to resolve XDG cache directory")?;
    Ok(base.join("veila").join("weather"))
}

fn preview_weather_condition_now() -> WeatherCondition {
    let now = OffsetDateTime::now_utc()
        .to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC));
    preview_weather_condition_for_hour(now.hour())
}

const fn preview_weather_condition_for_hour(hour: u8) -> WeatherCondition {
    if hour >= 6 && hour < 18 {
        WeatherCondition::ClearDay
    } else {
        WeatherCondition::ClearNight
    }
}

fn preview_now_playing_snapshot(
    title: Option<String>,
    artist: Option<String>,
    artwork_path: Option<PathBuf>,
) -> Option<NowPlayingSnapshot> {
    Some(NowPlayingSnapshot {
        title: title.unwrap_or_else(|| String::from("Northern Attitude")),
        artist: artist.or_else(|| Some(String::from("Noah Kahan"))),
        artwork_path,
        fetched_at_unix: 0,
    })
}

fn preview_battery_snapshot(
    options: &CurtainOptions,
    config: &AppConfig,
) -> Option<BatterySnapshot> {
    if let Some(snapshot) = options.battery_snapshot.clone() {
        return Some(snapshot);
    }

    if let Some(percent) = options.preview_battery_percent {
        return Some(BatterySnapshot {
            percent,
            charging: options.preview_battery_charging.unwrap_or(false),
        });
    }

    if let Some(charging) = options.preview_battery_charging {
        return Some(BatterySnapshot {
            percent: 84,
            charging,
        });
    }

    config.battery.mock_snapshot()
}

fn preview_clock_datetime(time: PreviewClockTime) -> OffsetDateTime {
    let now = OffsetDateTime::now_utc()
        .to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC));
    now.replace_hour(time.hour)
        .and_then(|datetime| datetime.replace_minute(time.minute))
        .and_then(|datetime| datetime.replace_second(0))
        .and_then(|datetime| datetime.replace_millisecond(0))
        .unwrap_or(now)
}

#[cfg(test)]
mod tests {
    use super::{
        PreviewGeocodedLocationCache, load_cached_preview_weather_snapshot_from,
        preview_battery_snapshot, preview_clock_datetime, preview_username,
        preview_weather_cache_path_for_coordinates, preview_weather_condition_for_hour,
        preview_weather_location, preview_weather_location_cache_path,
        preview_weather_override_snapshot,
    };
    use std::fs;
    use veila_common::{AppConfig, BatterySnapshot, WeatherCondition, WeatherSnapshot};

    use crate::{CurtainOptions, PreviewClockTime};

    #[test]
    fn uses_day_icon_during_daylight_preview_hours() {
        assert_eq!(
            preview_weather_condition_for_hour(6),
            WeatherCondition::ClearDay
        );
        assert_eq!(
            preview_weather_condition_for_hour(12),
            WeatherCondition::ClearDay
        );
        assert_eq!(
            preview_weather_condition_for_hour(17),
            WeatherCondition::ClearDay
        );
    }

    #[test]
    fn uses_night_icon_outside_daylight_preview_hours() {
        assert_eq!(
            preview_weather_condition_for_hour(0),
            WeatherCondition::ClearNight
        );
        assert_eq!(
            preview_weather_condition_for_hour(5),
            WeatherCondition::ClearNight
        );
        assert_eq!(
            preview_weather_condition_for_hour(18),
            WeatherCondition::ClearNight
        );
        assert_eq!(
            preview_weather_condition_for_hour(23),
            WeatherCondition::ClearNight
        );
    }

    #[test]
    fn prefers_cached_weather_snapshot_for_preview_when_available() {
        let cache_root =
            std::env::temp_dir().join(format!("veila-preview-weather-{}", std::process::id()));
        let weather_root = cache_root.join("veila").join("weather");
        fs::create_dir_all(&weather_root).expect("weather cache dir");

        let config = AppConfig::from_toml_str(
            r#"
                [weather]
                enabled = true
                location = "Seceda"
            "#,
        )
        .expect("config");

        let location_cache = preview_weather_location_cache_path(&weather_root, "Seceda");
        fs::write(
            &location_cache,
            serde_json::to_vec(&PreviewGeocodedLocationCache {
                latitude: 35.6762,
                longitude: 139.6503,
            })
            .expect("location cache"),
        )
        .expect("write location cache");

        let snapshot_cache =
            preview_weather_cache_path_for_coordinates(&weather_root, 35.6762, 139.6503);
        fs::write(
            &snapshot_cache,
            serde_json::to_vec(&WeatherSnapshot {
                temperature_celsius: 12,
                condition: WeatherCondition::Rain,
                fetched_at_unix: 123,
            })
            .expect("snapshot cache"),
        )
        .expect("write snapshot cache");

        let snapshot = load_cached_preview_weather_snapshot_from(&config, &weather_root, None)
            .expect("load cached preview snapshot");

        assert_eq!(
            snapshot,
            Some(WeatherSnapshot {
                temperature_celsius: 12,
                condition: WeatherCondition::Rain,
                fetched_at_unix: 123,
            })
        );

        fs::remove_file(location_cache).ok();
        fs::remove_file(snapshot_cache).ok();
        fs::remove_dir(weather_root).ok();
        fs::remove_dir(cache_root.join("veila")).ok();
        fs::remove_dir(cache_root).ok();
    }

    #[test]
    fn preview_weather_override_uses_requested_condition_and_temperature() {
        let options = CurtainOptions {
            preview_weather_condition: Some(WeatherCondition::Snow),
            preview_weather_temperature_celsius: Some(-4),
            ..CurtainOptions::default()
        };

        assert_eq!(
            preview_weather_override_snapshot(&options),
            Some(WeatherSnapshot {
                temperature_celsius: -4,
                condition: WeatherCondition::Snow,
                fetched_at_unix: 0,
            })
        );
    }

    #[test]
    fn preview_weather_location_prefers_override() {
        let options = CurtainOptions {
            preview_weather_location: Some(String::from("Tokyo")),
            ..CurtainOptions::default()
        };
        let config = AppConfig::from_toml_str(
            r#"
                [weather]
                enabled = true
                location = "Riga"
            "#,
        )
        .expect("config");

        assert_eq!(
            preview_weather_location(&options, &config),
            Some(String::from("Tokyo"))
        );
    }

    #[test]
    fn preview_username_prefers_override() {
        let options = CurtainOptions {
            preview_username: Some(String::from("guest")),
            ..CurtainOptions::default()
        };
        let config = AppConfig::from_toml_str(
            r#"
                [lock]
                username = "ns"
            "#,
        )
        .expect("config");

        assert_eq!(
            preview_username(&options, &config),
            Some(String::from("guest"))
        );
    }

    #[test]
    fn preview_battery_override_uses_requested_percent_and_charging() {
        let options = CurtainOptions {
            preview_battery_percent: Some(91),
            preview_battery_charging: Some(true),
            ..CurtainOptions::default()
        };
        let config = AppConfig::default();

        assert_eq!(
            preview_battery_snapshot(&options, &config),
            Some(BatterySnapshot {
                percent: 91,
                charging: true,
            })
        );
    }

    #[test]
    fn preview_clock_datetime_reuses_local_date_with_requested_time() {
        let datetime = preview_clock_datetime(PreviewClockTime {
            hour: 21,
            minute: 54,
        });

        assert_eq!(datetime.hour(), 21);
        assert_eq!(datetime.minute(), 54);
        assert_eq!(datetime.second(), 0);
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PreviewGeocodedLocationCache {
    latitude: f64,
    longitude: f64,
}
