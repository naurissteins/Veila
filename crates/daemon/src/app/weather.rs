use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::PathBuf,
    time::Duration,
};

use anyhow::{Context, Result};
use serde::Deserialize;
use time::OffsetDateTime;
use tokio::sync::watch;
use veila_common::{WeatherCondition, WeatherConfig, WeatherSnapshot};

#[derive(Clone)]
pub(super) struct WeatherHandle {
    config_tx: watch::Sender<WeatherConfig>,
    snapshot_rx: watch::Receiver<Option<WeatherSnapshot>>,
}

impl WeatherHandle {
    pub(super) fn spawn(config: &WeatherConfig) -> Self {
        let initial_snapshot = load_cached_snapshot(config).ok().flatten();
        let (config_tx, config_rx) = watch::channel(config.clone());
        let (snapshot_tx, snapshot_rx) = watch::channel(initial_snapshot);

        tokio::spawn(async move {
            run_weather_service(config_rx, snapshot_tx).await;
        });

        Self {
            config_tx,
            snapshot_rx,
        }
    }

    pub(super) fn current_snapshot(&self) -> Option<WeatherSnapshot> {
        self.snapshot_rx.borrow().clone()
    }

    pub(super) fn update_config(&self, config: &WeatherConfig) {
        let _ = self.config_tx.send(config.clone());
    }
}

async fn run_weather_service(
    mut config_rx: watch::Receiver<WeatherConfig>,
    snapshot_tx: watch::Sender<Option<WeatherSnapshot>>,
) {
    let mut config = config_rx.borrow().clone();
    let mut needs_refresh = true;

    loop {
        if weather_enabled(&config) {
            if needs_refresh && let Some(snapshot) = fetch_snapshot_async(config.clone()).await {
                snapshot_tx.send_replace(Some(snapshot));
            }

            let refresh = tokio::time::sleep(refresh_interval(&config));
            tokio::pin!(refresh);

            tokio::select! {
                _ = &mut refresh => {
                    needs_refresh = true;
                }
                changed = config_rx.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    config = config_rx.borrow().clone();
                    snapshot_tx.send_replace(load_cached_snapshot(&config).ok().flatten());
                    needs_refresh = true;
                }
            }
        } else {
            snapshot_tx.send_replace(None);
            if config_rx.changed().await.is_err() {
                break;
            }
            config = config_rx.borrow().clone();
            needs_refresh = true;
        }
    }
}

async fn fetch_snapshot_async(config: WeatherConfig) -> Option<WeatherSnapshot> {
    match tokio::task::spawn_blocking(move || fetch_snapshot(&config)).await {
        Ok(Ok(snapshot)) => Some(snapshot),
        Ok(Err(error)) => {
            tracing::warn!("weather refresh failed: {error:#}");
            None
        }
        Err(error) => {
            tracing::warn!("weather refresh task failed: {error:#}");
            None
        }
    }
}

fn fetch_snapshot(config: &WeatherConfig) -> Result<WeatherSnapshot> {
    let (latitude, longitude) = config
        .clone()
        .coordinates()
        .context("weather is missing coordinates")?;
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={latitude:.6}&longitude={longitude:.6}&current=temperature_2m,weather_code,is_day&temperature_unit=celsius"
    );
    let response = ureq::get(&url)
        .set("User-Agent", "Veila/0.1 weather widget")
        .call()
        .context("failed to fetch weather from Open-Meteo")?;
    let payload: OpenMeteoResponse = response
        .into_json()
        .context("failed to decode Open-Meteo response")?;
    let snapshot = WeatherSnapshot {
        temperature_celsius: payload.current.temperature_2m.round() as i16,
        condition: map_weather_code(payload.current.weather_code, payload.current.is_day == 1),
        fetched_at_unix: OffsetDateTime::now_utc().unix_timestamp(),
    };

    store_cached_snapshot(config, &snapshot).context("failed to store cached weather snapshot")?;
    tracing::debug!(
        temperature_celsius = snapshot.temperature_celsius,
        ?snapshot.condition,
        "weather refresh succeeded"
    );
    Ok(snapshot)
}

fn weather_enabled(config: &WeatherConfig) -> bool {
    config.enabled
        && config.normalized_location().is_some()
        && config.clone().coordinates().is_some()
}

fn refresh_interval(config: &WeatherConfig) -> Duration {
    Duration::from_secs(u64::from(config.refresh_minutes.max(5)) * 60)
}

fn load_cached_snapshot(config: &WeatherConfig) -> Result<Option<WeatherSnapshot>> {
    if !weather_enabled(config) {
        return Ok(None);
    }

    let cache_path = cache_path(config)?;
    let Ok(raw) = fs::read_to_string(&cache_path) else {
        return Ok(None);
    };
    serde_json::from_str(&raw)
        .map(Some)
        .context("failed to parse cached weather snapshot")
}

fn store_cached_snapshot(config: &WeatherConfig, snapshot: &WeatherSnapshot) -> Result<()> {
    let cache_path = cache_path(config)?;
    let Some(cache_dir) = cache_path.parent() else {
        anyhow::bail!("weather cache path has no parent");
    };
    fs::create_dir_all(cache_dir).context("failed to create weather cache directory")?;
    let raw = serde_json::to_vec(snapshot).context("failed to encode cached weather snapshot")?;
    fs::write(&cache_path, raw).context("failed to write weather cache file")
}

fn cache_path(config: &WeatherConfig) -> Result<PathBuf> {
    let Some((latitude, longitude)) = config.clone().coordinates() else {
        anyhow::bail!("weather coordinates are not configured");
    };
    let mut hasher = DefaultHasher::new();
    latitude.to_bits().hash(&mut hasher);
    longitude.to_bits().hash(&mut hasher);
    let key = hasher.finish();
    Ok(cache_root()?.join(format!("{key:016x}.json")))
}

fn cache_root() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .context("failed to resolve XDG cache directory")?;
    Ok(base.join("veila").join("weather"))
}

fn map_weather_code(code: u8, is_day: bool) -> WeatherCondition {
    match code {
        0 => {
            if is_day {
                WeatherCondition::ClearDay
            } else {
                WeatherCondition::ClearNight
            }
        }
        1 | 2 => {
            if is_day {
                WeatherCondition::PartlyCloudyDay
            } else {
                WeatherCondition::PartlyCloudyNight
            }
        }
        3 => WeatherCondition::Overcast,
        45 | 48 => WeatherCondition::Fog,
        51 | 53 | 55 | 56 | 57 => WeatherCondition::Drizzle,
        61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 => WeatherCondition::Rain,
        71 | 73 | 75 | 77 | 85 | 86 => WeatherCondition::Snow,
        95 | 96 | 99 => WeatherCondition::Thunderstorm,
        _ => WeatherCondition::Cloudy,
    }
}

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    current: OpenMeteoCurrent,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    temperature_2m: f32,
    weather_code: u8,
    is_day: u8,
}
