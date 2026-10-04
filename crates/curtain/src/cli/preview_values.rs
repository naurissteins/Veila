use anyhow::{Context, Result, bail};
use veila_common::WeatherCondition;

use crate::PreviewClockTime;

pub(super) fn parse_preview_size(input: &str) -> Result<veila_renderer::FrameSize> {
    let (width, height) = input
        .split_once('x')
        .ok_or_else(|| anyhow::anyhow!("preview size must use WIDTHxHEIGHT"))?;
    let width = width.parse::<u32>().context("invalid preview width")?;
    let height = height.parse::<u32>().context("invalid preview height")?;

    if width == 0 || height == 0 {
        bail!("preview size must be non-zero");
    }

    Ok(veila_renderer::FrameSize::new(width, height))
}

pub(super) fn parse_preview_weather_condition(input: &str) -> Result<WeatherCondition> {
    let normalized = input.trim().to_ascii_lowercase();
    let condition = match normalized.as_str() {
        "clear-day" | "clear_day" | "clearday" | "sunny" | "day" => WeatherCondition::ClearDay,
        "clear-night" | "clear_night" | "clearnight" | "night" => WeatherCondition::ClearNight,
        "partly-cloudy-day" | "partly_cloudy_day" | "partlycloudyday" => {
            WeatherCondition::PartlyCloudyDay
        }
        "partly-cloudy-night" | "partly_cloudy_night" | "partlycloudynight" => {
            WeatherCondition::PartlyCloudyNight
        }
        "cloudy" => WeatherCondition::Cloudy,
        "overcast" => WeatherCondition::Overcast,
        "fog" => WeatherCondition::Fog,
        "drizzle" => WeatherCondition::Drizzle,
        "rain" => WeatherCondition::Rain,
        "snow" => WeatherCondition::Snow,
        "thunderstorm" | "storm" => WeatherCondition::Thunderstorm,
        "unknown" => WeatherCondition::Unknown,
        _ => bail!("unsupported preview weather condition"),
    };
    Ok(condition)
}

pub(super) fn parse_preview_battery_percent(input: &str) -> Result<u8> {
    let percent = input
        .parse::<u8>()
        .context("invalid preview battery percent")?;
    if percent > 100 {
        bail!("preview battery percent must be between 0 and 100");
    }
    Ok(percent)
}

pub(super) fn parse_preview_bool(input: &str) -> Result<bool> {
    match input.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => bail!("expected true or false"),
    }
}

pub(super) fn parse_preview_clock_time(input: &str) -> Result<PreviewClockTime> {
    let (hour, minute) = input
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("preview time must use HH:MM"))?;
    let hour = hour.parse::<u8>().context("invalid preview hour")?;
    let minute = minute.parse::<u8>().context("invalid preview minute")?;
    if hour > 23 || minute > 59 {
        bail!("preview time must be a valid 24-hour clock value");
    }
    Ok(PreviewClockTime { hour, minute })
}
