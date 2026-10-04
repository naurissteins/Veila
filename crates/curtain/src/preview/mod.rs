mod samples;
mod scene;
mod weather;

use anyhow::{Context, Result};
use veila_common::AppConfig;
use veila_renderer::FrameSize;

use crate::CurtainOptions;

const DEFAULT_PREVIEW_SIZE: FrameSize = FrameSize::new(2560, 1440);

pub(crate) fn render_preview(options: CurtainOptions) -> Result<()> {
    let output_path = options
        .preview_png
        .clone()
        .context("preview mode requires --preview-png <path> or --preview-png=<path>")?;
    let preview_size = options.preview_size.unwrap_or(DEFAULT_PREVIEW_SIZE);
    let loaded = AppConfig::load(options.config_path.as_deref())
        .context("failed to load config for preview rendering")?;
    let buffer = scene::render(&options, &loaded.config, preview_size)?;
    buffer
        .save_png(&output_path)
        .with_context(|| format!("failed to save preview PNG to {}", output_path.display()))?;
    tracing::info!(path = %output_path.display(), width = preview_size.width,
        height = preview_size.height, "rendered curtain preview PNG");
    Ok(())
}
