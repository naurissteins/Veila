mod preview_values;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use veila_common::ipc::LatencyReportMode;

use crate::CurtainOptions;
use preview_values::{
    parse_preview_battery_percent, parse_preview_bool, parse_preview_clock_time,
    parse_preview_size, parse_preview_weather_condition,
};

impl CurtainOptions {
    /// Parses curtain options from an iterator of process arguments.
    pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self::default();
        let mut args = args.into_iter().skip(1);

        while let Some(arg) = args.next() {
            if arg == "--help" || arg == "-h" {
                options.help = true;
                continue;
            }

            if arg == "--lock" {
                options.lock = true;
                continue;
            }

            if arg == "--force-emergency-ui" {
                options.force_emergency_ui = true;
                continue;
            }

            if arg == "--owner-gate" {
                options.owner_gate = true;
                continue;
            }

            if let Some(path) = parse_option_value(&arg, "--owner-record", &mut args)? {
                options.owner_record = Some(PathBuf::from(path));
                continue;
            }

            if let Some(mode) = parse_latency_report_arg(&arg)? {
                options.latency_report = mode;
                continue;
            }

            if let Some(path) = parse_option_value(&arg, "--notify-socket", &mut args)? {
                options.notify_socket = Some(PathBuf::from(path));
                continue;
            }

            if let Some(path) = parse_option_value(&arg, "--daemon-socket", &mut args)? {
                options.daemon_socket = Some(PathBuf::from(path));
                continue;
            }

            if let Some(path) = parse_option_value(&arg, "--control-socket", &mut args)? {
                options.control_socket = Some(PathBuf::from(path));
                continue;
            }

            if let Some(path) = parse_option_value(&arg, "--config", &mut args)? {
                options.config_path = Some(PathBuf::from(path));
                continue;
            }

            if let Some(path) = parse_option_value(&arg, "--initial-background-path", &mut args)? {
                options.initial_background_path = Some(PathBuf::from(path));
                continue;
            }

            if let Some(path) = parse_option_value(&arg, "--preview-png", &mut args)? {
                options.preview_png = Some(PathBuf::from(path));
                continue;
            }

            if let Some(size) = parse_option_value(&arg, "--preview-size", &mut args)? {
                options.preview_size =
                    Some(parse_preview_size(&size).context("failed to parse preview size")?);
                continue;
            }

            if let Some(path) = parse_option_value(&arg, "--preview-artwork", &mut args)? {
                options.preview_artwork = Some(PathBuf::from(path));
                continue;
            }

            if let Some(title) = parse_option_value(&arg, "--preview-title", &mut args)? {
                options.preview_title = Some(title);
                continue;
            }

            if let Some(artist) = parse_option_value(&arg, "--preview-artist", &mut args)? {
                options.preview_artist = Some(artist);
                continue;
            }

            if let Some(username) = parse_option_value(&arg, "--preview-username", &mut args)? {
                options.preview_username = Some(username);
                continue;
            }

            if arg == "--preview-hide-widgets" {
                options.preview_hide_widgets = true;
                continue;
            }

            if arg == "--preview-hide-weather" {
                options.preview_hide_weather = true;
                continue;
            }

            if arg == "--preview-hide-battery" {
                options.preview_hide_battery = true;
                continue;
            }

            if arg == "--preview-hide-now-playing" {
                options.preview_hide_now_playing = true;
                continue;
            }

            if arg == "--preview-hide-keyboard-label" {
                options.preview_hide_keyboard_label = true;
                continue;
            }

            if let Some(location) =
                parse_option_value(&arg, "--preview-weather-location", &mut args)?
            {
                options.preview_weather_location = Some(location);
                continue;
            }

            if let Some(condition) =
                parse_option_value(&arg, "--preview-weather-condition", &mut args)?
            {
                options.preview_weather_condition = Some(
                    parse_preview_weather_condition(&condition)
                        .context("failed to parse preview weather condition")?,
                );
                continue;
            }

            if let Some(temperature) =
                parse_option_value(&arg, "--preview-weather-temperature", &mut args)?
            {
                options.preview_weather_temperature_celsius = Some(
                    temperature
                        .parse::<i16>()
                        .context("invalid preview weather temperature")?,
                );
                continue;
            }

            if let Some(percent) = parse_option_value(&arg, "--preview-battery-percent", &mut args)?
            {
                options.preview_battery_percent = Some(
                    parse_preview_battery_percent(&percent)
                        .context("failed to parse preview battery percent")?,
                );
                continue;
            }

            if let Some(charging) =
                parse_option_value(&arg, "--preview-battery-charging", &mut args)?
            {
                options.preview_battery_charging = Some(
                    parse_preview_bool(&charging)
                        .context("failed to parse preview battery charging state")?,
                );
                continue;
            }

            if let Some(time) = parse_option_value(&arg, "--preview-time", &mut args)? {
                options.preview_time =
                    Some(parse_preview_clock_time(&time).context("failed to parse preview time")?);
                continue;
            }

            bail!("unknown curtain argument: {arg}");
        }

        Ok(options)
    }
}

fn parse_option_value(
    arg: &str,
    flag: &str,
    remaining: &mut impl Iterator<Item = String>,
) -> Result<Option<String>> {
    if let Some(value) = arg.strip_prefix(&format!("{flag}=")) {
        return Ok(Some(value.to_string()));
    }

    if arg != flag {
        return Ok(None);
    }

    let value = remaining
        .next()
        .with_context(|| format!("{flag} requires a value"))?;
    // Negative numbers remain values; a following long flag means the value is missing.
    if value.starts_with("--") {
        bail!("{flag} requires a value");
    }

    Ok(Some(value))
}

fn parse_latency_report_arg(arg: &str) -> Result<Option<LatencyReportMode>> {
    if arg == "--latency-report" {
        return Ok(Some(LatencyReportMode::Basic));
    }

    let Some(mode) = arg.strip_prefix("--latency-report=") else {
        return Ok(None);
    };

    match mode {
        "basic" => Ok(Some(LatencyReportMode::Basic)),
        "verbose" => Ok(Some(LatencyReportMode::Verbose)),
        _ => bail!("unknown latency report mode: {mode}"),
    }
}

#[cfg(test)]
mod tests;
