mod cache;
use crate::CurtainOptions;
use cache::load_cached_preview_weather_snapshot;
use time::{OffsetDateTime, UtcOffset};
use veila_common::{AppConfig, WeatherCondition, WeatherSnapshot};

pub(super) fn preview_weather_snapshot(
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

pub(super) fn preview_weather_location(
    options: &CurtainOptions,
    config: &AppConfig,
) -> Option<String> {
    options
        .preview_weather_location
        .clone()
        .or_else(|| config.weather.location.clone())
}

pub(super) fn preview_weather_hidden(options: &CurtainOptions, config: &AppConfig) -> bool {
    if options.preview_hide_widgets || options.preview_hide_weather {
        return true;
    }

    if preview_weather_forced(options) {
        return false;
    }

    !config.weather.enabled || !config.visuals.weather_enabled()
}

fn preview_weather_forced(options: &CurtainOptions) -> bool {
    options.weather_snapshot.is_some()
        || options.preview_weather_location.is_some()
        || options.preview_weather_condition.is_some()
        || options.preview_weather_temperature_celsius.is_some()
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

#[cfg(test)]
mod tests;
