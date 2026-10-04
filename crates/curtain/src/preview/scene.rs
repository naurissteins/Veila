use super::samples::{PreviewSamples, preview_keyboard_layout_label};
use crate::{CurtainOptions, PreviewClockTime};
use anyhow::{Context, Result};
use std::path::Path;
use time::{OffsetDateTime, UtcOffset};
use veila_common::{AppConfig, ConfigColor};
use veila_renderer::{
    ClearColor, FrameSize, SoftwareBuffer, background::BackgroundAsset, cover::CoverArtAsset,
};
use veila_ui::{
    ShellState, ShellTheme,
    background::{background_generated, background_treatment},
};

pub(super) fn render(
    options: &CurtainOptions,
    config: &AppConfig,
    size: FrameSize,
) -> Result<SoftwareBuffer> {
    let samples = PreviewSamples::resolve(options, config);
    let background = BackgroundAsset::load(
        config.background.resolved_path().as_deref(),
        to_clear_color(config.background.color),
        background_generated(&config.background),
        background_treatment(&config.background),
    )
    .context("failed to load preview background")?;
    let mut buffer = background
        .render(size)
        .context("failed to render preview background")?;
    let mut shell = ShellState::new_with_username_and_widgets(
        ShellTheme::from_config(config),
        Some(config.visuals.input_placeholder()),
        samples.username,
        config.avatar_image_path().map(Path::to_path_buf),
        config.visuals.username_enabled(),
        samples.weather_location,
        samples.weather,
        config.weather.unit,
        samples.battery,
        samples.now_playing,
    );
    if options.force_emergency_ui {
        shell.activate_emergency();
        buffer.clear(ClearColor::opaque(12, 14, 18));
    }
    load_artwork(&mut shell, size);
    if let Some(preview_time) = options.preview_time {
        shell.set_preview_time(preview_clock_datetime(preview_time));
    }
    shell.set_preview_grid_enabled(true);
    shell.set_keyboard_layout_label(preview_keyboard_layout_label(options));
    shell.render_overlay(&mut buffer);
    Ok(buffer)
}

fn load_artwork(shell: &mut ShellState, size: FrameSize) {
    // Emergency previews skip artwork just like the secure emergency scene.
    if !shell.emergency_active()
        && let Some(path) = shell
            .pending_now_playing_artwork_path()
            .map(Path::to_path_buf)
    {
        match CoverArtAsset::load(&path, shell.now_playing_artwork_decode_size(size, 1)) {
            Ok(artwork) => {
                shell.set_now_playing_artwork(&path, artwork);
            }
            Err(error) => {
                tracing::debug!(path = %path.display(), "failed to load preview artwork: {error}")
            }
        }
    }
}

fn to_clear_color(color: ConfigColor) -> ClearColor {
    ClearColor::rgba(color.0, color.1, color.2, color.3)
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
mod tests;
