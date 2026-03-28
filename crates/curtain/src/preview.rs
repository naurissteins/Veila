use anyhow::{Context, Result};
use std::path::PathBuf;
use veila_common::{AppConfig, ConfigColor, NowPlayingSnapshot, WeatherCondition, WeatherSnapshot};
use veila_renderer::{
    ClearColor, FrameSize, SoftwareBuffer,
    background::{BackgroundAsset, BackgroundTreatment},
};
use veila_ui::{ShellState, ShellTheme};

use crate::CurtainOptions;

const DEFAULT_PREVIEW_SIZE: FrameSize = FrameSize::new(2560, 1440);

pub(crate) fn render_preview(options: CurtainOptions) -> Result<()> {
    let output_path = options
        .preview_png
        .clone()
        .context("preview mode requires --preview-png=PATH")?;
    let preview_size = options.preview_size.unwrap_or(DEFAULT_PREVIEW_SIZE);
    let loaded = AppConfig::load(options.config_path.as_deref())
        .context("failed to load config for preview rendering")?;
    let config = loaded.config;
    let weather_snapshot = options
        .weather_snapshot
        .or_else(|| preview_weather_snapshot(&config));
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
        config.lock.username.clone(),
        config.lock.avatar_path.clone(),
        config.lock.show_username,
        config.weather.location.clone(),
        weather_snapshot,
        config.weather.unit,
        options
            .battery_snapshot
            .or_else(|| config.battery.mock_snapshot()),
        now_playing_snapshot,
    );
    let mut shell = shell;
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

fn preview_weather_snapshot(config: &AppConfig) -> Option<WeatherSnapshot> {
    if !config.weather.enabled {
        return None;
    }

    config.weather.location.as_ref()?;
    Some(WeatherSnapshot {
        temperature_celsius: 21,
        condition: WeatherCondition::ClearDay,
        fetched_at_unix: 0,
    })
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
