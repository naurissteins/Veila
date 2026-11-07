use anyhow::{Context, Result};
use veila_common::{AppConfig, WeatherSnapshot};
pub(super) fn load_cached_preview_weather_snapshot(
    config: &AppConfig,
    location: Option<&str>,
) -> Result<Option<WeatherSnapshot>> {
    let root = veila_renderer::cache::resolve_cache_root(None)
        .context("failed to resolve XDG cache directory")?;
    veila_common::weather::cache::load_snapshot_from(config, &root.join("weather"), location)
        .map_err(Into::into)
}
