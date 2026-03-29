use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use veila_common::{AppConfig, LayerAlignment, LayerMode, RgbColor};
use veila_renderer::{
    ClearColor,
    background::{
        BackgroundAsset, BackgroundTreatment, RenderCacheSummary, SourceCacheStatus,
        load_cached_render_variant, prewarm_rendered, prewarm_source, store_cached_render_variant,
    },
    draw::layer::{BackdropLayerMode, BackdropLayerStyle, draw_backdrop_layer},
    shape::Rect,
};

use crate::app::output_probe;

pub(super) fn spawn_background_prewarm(config: &AppConfig) {
    let Some(path) = config.background.resolved_path() else {
        return;
    };
    let fallback = to_clear_color(config.background.color);
    let treatment = background_treatment(&config.background);
    let layer = layer_prewarm_spec(config);

    tokio::spawn(async move {
        let started_at = Instant::now();
        let join_result = tokio::task::spawn_blocking(move || {
            prewarm_wallpaper(path, fallback, treatment, layer)
        })
        .await;

        match join_result {
            Ok(Ok(report)) => {
                tracing::info!(
                    path = %report.path.display(),
                    elapsed_ms = report.source_elapsed_ms,
                    cache_status = match report.source_status {
                        SourceCacheStatus::Hit => "hit",
                        SourceCacheStatus::Warmed => "warmed",
                    },
                    "background source prewarm finished"
                );

                if let Some(rendered) = report.rendered {
                    tracing::info!(
                        path = %report.path.display(),
                        elapsed_ms = rendered.elapsed_ms,
                        probed_outputs = rendered.probed_outputs,
                        cache_hits = rendered.summary.cache_hits,
                        warmed_sizes = rendered.summary.warmed_sizes,
                        "background render prewarm finished"
                    );
                }

                if let Some(layered) = report.layered {
                    tracing::info!(
                        path = %report.path.display(),
                        elapsed_ms = layered.elapsed_ms,
                        probed_outputs = layered.probed_outputs,
                        cache_hits = layered.cache_hits,
                        warmed_sizes = layered.warmed_sizes,
                        "layered background prewarm finished"
                    );
                }
            }
            Ok(Err((path, error))) => {
                tracing::warn!(
                    path = %path.display(),
                    elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                    "background source prewarm failed: {error:#}"
                );
            }
            Err(error) => {
                tracing::warn!("background source prewarm task failed: {error:#}");
            }
        }
    });
}

fn prewarm_wallpaper(
    path: PathBuf,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    layer: Option<LayerPrewarmSpec>,
) -> Result<PrewarmReport, (PathBuf, anyhow::Error)> {
    let source_started_at = Instant::now();
    match prewarm_source(&path) {
        Ok(status) => {
            let source_elapsed_ms = source_started_at
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            let rendered = prewarm_rendered_backgrounds(&path, fallback, treatment);
            let layered = prewarm_layered_backgrounds(&path, fallback, treatment, layer.as_ref());
            Ok(PrewarmReport {
                path,
                source_status: status,
                source_elapsed_ms,
                rendered,
                layered,
            })
        }
        Err(error) => Err((path, anyhow::Error::from(error))),
    }
}

fn prewarm_rendered_backgrounds(
    path: &Path,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
) -> Option<RenderedPrewarmReport> {
    let sizes = output_probe::current_output_sizes().ok()?;
    if sizes.is_empty() {
        return None;
    }

    let started_at = Instant::now();
    let summary = prewarm_rendered(path, fallback, treatment, &sizes).ok()?;
    Some(RenderedPrewarmReport {
        elapsed_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        probed_outputs: sizes.len(),
        summary,
    })
}

fn prewarm_layered_backgrounds(
    path: &Path,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    layer: Option<&LayerPrewarmSpec>,
) -> Option<LayeredPrewarmReport> {
    let layer = layer?;
    let variant = &layer.variant;
    let sizes = output_probe::current_output_sizes().ok()?;
    if sizes.is_empty() {
        return None;
    }

    let started_at = Instant::now();
    let asset = BackgroundAsset::load(Some(path), fallback, treatment).ok()?;
    let mut cache_hits = 0usize;
    let mut warmed_sizes = 0usize;
    let probed_outputs = sizes.len();

    for size in sizes {
        if load_cached_render_variant(path, size, treatment, variant)
            .ok()
            .flatten()
            .is_some()
        {
            cache_hits += 1;
            continue;
        }

        let mut buffer = asset.render(size).ok()?;
        apply_layer_spec(layer, &mut buffer);
        store_cached_render_variant(path, size, treatment, &buffer, variant).ok()?;
        warmed_sizes += 1;
    }

    Some(LayeredPrewarmReport {
        elapsed_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        probed_outputs,
        cache_hits,
        warmed_sizes,
    })
}

fn apply_layer_spec(layer: &LayerPrewarmSpec, buffer: &mut veila_renderer::SoftwareBuffer) {
    let frame_width = buffer.size().width as i32;
    let frame_height = buffer.size().height as i32;
    let width = layer
        .width
        .unwrap_or((frame_width as f32 * 0.36) as i32)
        .clamp(1, frame_width.max(1));
    let offset_x = layer.offset_x;
    let unclamped_x = match layer.alignment {
        LayerAlignment::Left => offset_x,
        LayerAlignment::Center => (frame_width - width) / 2 + offset_x,
        LayerAlignment::Right => frame_width - width + offset_x,
    };
    let x = unclamped_x.clamp(-width + 1, frame_width - 1);
    let mode = match layer.mode {
        LayerMode::Solid => BackdropLayerMode::Solid,
        LayerMode::Blur => BackdropLayerMode::Blur,
    };

    draw_backdrop_layer(
        buffer,
        Rect::new(x, 0, width, frame_height),
        BackdropLayerStyle::new(mode, layer.color, layer.blur_radius),
    );
}

fn layer_prewarm_spec(config: &AppConfig) -> Option<LayerPrewarmSpec> {
    if !config.visuals.layer_enabled() {
        return None;
    }

    let raw_color = config.visuals.layer_color().unwrap_or(config.visuals.panel);
    let color = to_layer_color(raw_color, config.visuals.layer_opacity());
    Some(LayerPrewarmSpec {
        variant: format!(
            "layer:v1:{:?}:{:?}:{:?}:{:?}:{},{},{},{}:{}",
            config.visuals.layer_mode(),
            config.visuals.layer_alignment(),
            config.visuals.layer_width(),
            config.visuals.layer_offset_x(),
            color.red,
            color.green,
            color.blue,
            color.alpha,
            config.visuals.layer_blur_radius().unwrap_or(12)
        ),
        mode: config.visuals.layer_mode(),
        alignment: config.visuals.layer_alignment(),
        width: config.visuals.layer_width().map(i32::from),
        offset_x: i32::from(config.visuals.layer_offset_x().unwrap_or(0)),
        color,
        blur_radius: config.visuals.layer_blur_radius().unwrap_or(12),
    })
}

fn to_layer_color(color: RgbColor, opacity: Option<u8>) -> ClearColor {
    let alpha = opacity
        .map(|percent| ((u16::from(percent.min(100)) * 255 + 50) / 100) as u8)
        .unwrap_or(color.3);
    ClearColor::rgba(color.0, color.1, color.2, alpha)
}

fn to_clear_color(color: veila_common::RgbColor) -> ClearColor {
    ClearColor::rgba(color.0, color.1, color.2, color.3)
}

fn background_treatment(config: &veila_common::config::BackgroundConfig) -> BackgroundTreatment {
    BackgroundTreatment {
        blur_radius: config.blur_radius,
        dim_strength: config.dim_strength,
        tint: config
            .tint
            .map(|color| ClearColor::rgba(color.0, color.1, color.2, color.3)),
        tint_opacity: config.tint_opacity,
    }
}

struct PrewarmReport {
    path: PathBuf,
    source_status: SourceCacheStatus,
    source_elapsed_ms: u64,
    rendered: Option<RenderedPrewarmReport>,
    layered: Option<LayeredPrewarmReport>,
}

struct RenderedPrewarmReport {
    elapsed_ms: u64,
    probed_outputs: usize,
    summary: RenderCacheSummary,
}

struct LayeredPrewarmReport {
    elapsed_ms: u64,
    probed_outputs: usize,
    cache_hits: usize,
    warmed_sizes: usize,
}

#[derive(Clone)]
struct LayerPrewarmSpec {
    variant: String,
    mode: LayerMode,
    alignment: LayerAlignment,
    width: Option<i32>,
    offset_x: i32,
    color: ClearColor,
    blur_radius: u8,
}
