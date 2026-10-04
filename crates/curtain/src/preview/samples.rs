use super::weather::{preview_weather_hidden, preview_weather_location, preview_weather_snapshot};
use crate::CurtainOptions;
use std::path::PathBuf;
use veila_common::{AppConfig, BatterySnapshot, NowPlayingSnapshot, WeatherSnapshot};

pub(super) struct PreviewSamples {
    pub(super) username: Option<String>,
    pub(super) weather_location: Option<String>,
    pub(super) weather: Option<WeatherSnapshot>,
    pub(super) battery: Option<BatterySnapshot>,
    pub(super) now_playing: Option<NowPlayingSnapshot>,
}

impl PreviewSamples {
    pub(super) fn resolve(options: &CurtainOptions, config: &AppConfig) -> Self {
        let weather_hidden = preview_weather_hidden(options, config);
        let battery_hidden = preview_battery_hidden(options);
        let now_playing_hidden = preview_now_playing_hidden(options);
        let weather_location = preview_weather_location(options, config);
        let username = preview_username(options, config);
        let weather = if weather_hidden {
            None
        } else {
            preview_weather_snapshot(options, config, weather_location.as_deref())
        };
        let battery = if battery_hidden {
            None
        } else {
            preview_battery_snapshot(options, config)
        };
        let now_playing = if now_playing_hidden {
            None
        } else {
            options.now_playing_snapshot.clone().or_else(|| {
                preview_now_playing_snapshot(
                    options.preview_title.clone(),
                    options.preview_artist.clone(),
                    options.preview_artwork.clone(),
                )
            })
        };
        Self {
            username,
            weather_location: if weather_hidden {
                None
            } else {
                weather_location
            },
            weather,
            battery,
            now_playing,
        }
    }
}

fn preview_username(options: &CurtainOptions, config: &AppConfig) -> Option<String> {
    options
        .preview_username
        .clone()
        .or_else(|| config.visuals.username_text().map(str::to_owned))
}

pub(super) fn preview_keyboard_layout_label(options: &CurtainOptions) -> Option<String> {
    if options.preview_hide_widgets || options.preview_hide_keyboard_label {
        None
    } else {
        Some(String::from("EN"))
    }
}

fn preview_battery_hidden(options: &CurtainOptions) -> bool {
    options.preview_hide_widgets || options.preview_hide_battery
}

fn preview_now_playing_hidden(options: &CurtainOptions) -> bool {
    options.preview_hide_widgets || options.preview_hide_now_playing
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

#[cfg(test)]
mod tests;
