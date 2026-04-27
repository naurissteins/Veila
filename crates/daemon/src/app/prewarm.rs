use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use veila_common::{AppConfig, LayerAlignment, LayerMode, LayerStyle, RgbColor};
use veila_renderer::{
    ClearColor, FrameSize,
    background::{
        BackgroundAsset, BackgroundGradient, BackgroundRadial, BackgroundTreatment,
        GeneratedBackground, RenderCacheSummary, SourceCacheStatus,
        load_cached_generated_render_variant, load_cached_render_variant, prewarm_rendered,
        prewarm_rendered_generated, prewarm_source, store_cached_generated_render_variant,
        store_cached_render_variant,
    },
    draw::layer::{
        BackdropLayerAlignment, BackdropLayerMode, BackdropLayerShape, BackdropLayerStyle,
        draw_backdrop_layer,
    },
    shape::Rect,
};

use crate::app::output_probe;

pub(super) fn spawn_background_prewarm(config: &AppConfig) {
    let background = config.background.clone();
    let fallback = to_clear_color(config.background.color);
    let generated = background_generated(&config.background);
    let treatment = background_treatment(&config.background);
    let layer = layer_prewarm_spec(config);

    tokio::spawn(async move {
        let started_at = Instant::now();
        let join_result = tokio::task::spawn_blocking(move || {
            prewarm_backgrounds(background, generated, fallback, treatment, layer)
        })
        .await;

        match join_result {
            Ok(result) => {
                for report in result.wallpapers {
                    match report {
                        Ok(report) => log_prewarm_report(report),
                        Err((path, error)) => {
                            tracing::warn!(
                                path = %path.display(),
                                elapsed_ms = elapsed_ms(started_at),
                                "background source prewarm failed: {error:#}"
                            );
                        }
                    }
                }
                if let Some(report) = result.generated {
                    log_generated_prewarm_report(report, started_at);
                }
            }
            Err(error) => {
                tracing::warn!("background source prewarm task failed: {error:#}");
            }
        }
    });
}

fn log_prewarm_report(report: PrewarmReport) {
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

fn log_generated_prewarm_report(report: GeneratedPrewarmReport, started_at: Instant) {
    tracing::info!(
        elapsed_ms = report.rendered.elapsed_ms,
        probed_outputs = report.rendered.probed_outputs,
        cache_hits = report.rendered.summary.cache_hits,
        warmed_sizes = report.rendered.summary.warmed_sizes,
        generated_mode = report.mode,
        "generated background render prewarm finished"
    );

    if let Some(layered) = report.layered {
        tracing::info!(
            elapsed_ms = layered.elapsed_ms,
            probed_outputs = layered.probed_outputs,
            cache_hits = layered.cache_hits,
            warmed_sizes = layered.warmed_sizes,
            generated_mode = report.mode,
            "generated layered background prewarm finished"
        );
    }

    tracing::debug!(
        total_elapsed_ms = elapsed_ms(started_at),
        generated_mode = report.mode,
        "generated background prewarm completed"
    );
}

fn prewarm_backgrounds(
    background: veila_common::config::BackgroundConfig,
    generated: Option<GeneratedBackground>,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    layer: Option<LayerPrewarmSpec>,
) -> PrewarmResult {
    let outputs = output_probe::current_outputs().unwrap_or_default();
    let wallpapers = prewarm_jobs(&background, &outputs)
        .into_iter()
        .map(|job| prewarm_wallpaper(job, fallback, treatment, layer.as_ref()))
        .collect();
    let generated = generated.and_then(|generated| {
        let sizes = generated_sizes(&background, &outputs);
        prewarm_generated_backgrounds(generated, treatment, layer.as_ref(), &sizes)
    });

    PrewarmResult {
        wallpapers,
        generated,
    }
}

fn prewarm_wallpaper(
    job: PrewarmJob,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    layer: Option<&LayerPrewarmSpec>,
) -> Result<PrewarmReport, (PathBuf, anyhow::Error)> {
    let source_started_at = Instant::now();
    match prewarm_source(&job.path) {
        Ok(status) => {
            let source_elapsed_ms = elapsed_ms(source_started_at);
            let rendered = prewarm_rendered_backgrounds(&job.path, fallback, treatment, &job.sizes);
            let layered =
                prewarm_layered_backgrounds(&job.path, fallback, treatment, layer, &job.sizes);
            Ok(PrewarmReport {
                path: job.path,
                source_status: status,
                source_elapsed_ms,
                rendered,
                layered,
            })
        }
        Err(error) => Err((job.path, anyhow::Error::from(error))),
    }
}

fn prewarm_rendered_backgrounds(
    path: &Path,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    sizes: &[FrameSize],
) -> Option<RenderedPrewarmReport> {
    if sizes.is_empty() {
        return None;
    }

    let started_at = Instant::now();
    let summary = prewarm_rendered(path, fallback, treatment, sizes).ok()?;
    Some(RenderedPrewarmReport {
        elapsed_ms: elapsed_ms(started_at),
        probed_outputs: sizes.len(),
        summary,
    })
}

fn prewarm_layered_backgrounds(
    path: &Path,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    layer: Option<&LayerPrewarmSpec>,
    sizes: &[FrameSize],
) -> Option<LayeredPrewarmReport> {
    let layer = layer?;
    let variant = &layer.variant;
    if sizes.is_empty() {
        return None;
    }

    let started_at = Instant::now();
    let asset = BackgroundAsset::load(Some(path), fallback, None, treatment).ok()?;
    let mut cache_hits = 0usize;
    let mut warmed_sizes = 0usize;

    for size in sizes.iter().copied() {
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
        elapsed_ms: elapsed_ms(started_at),
        probed_outputs: sizes.len(),
        cache_hits,
        warmed_sizes,
    })
}

fn prewarm_generated_backgrounds(
    generated: GeneratedBackground,
    treatment: BackgroundTreatment,
    layer: Option<&LayerPrewarmSpec>,
    sizes: &[FrameSize],
) -> Option<GeneratedPrewarmReport> {
    if sizes.is_empty() {
        return None;
    }

    let started_at = Instant::now();
    let summary = prewarm_rendered_generated(generated, treatment, sizes).ok()?;
    let rendered = RenderedPrewarmReport {
        elapsed_ms: elapsed_ms(started_at),
        probed_outputs: sizes.len(),
        summary,
    };
    let layered = prewarm_generated_layered_backgrounds(generated, treatment, layer, sizes);

    Some(GeneratedPrewarmReport {
        mode: generated.mode_name(),
        rendered,
        layered,
    })
}

fn prewarm_generated_layered_backgrounds(
    generated: GeneratedBackground,
    treatment: BackgroundTreatment,
    layer: Option<&LayerPrewarmSpec>,
    sizes: &[FrameSize],
) -> Option<LayeredPrewarmReport> {
    let layer = layer?;
    let variant = &layer.variant;
    if sizes.is_empty() {
        return None;
    }

    let started_at = Instant::now();
    let asset = BackgroundAsset::load(
        None,
        ClearColor::opaque(0, 0, 0),
        Some(generated),
        treatment,
    )
    .ok()?;
    let mut cache_hits = 0usize;
    let mut warmed_sizes = 0usize;

    for size in sizes.iter().copied() {
        if load_cached_generated_render_variant(generated, size, treatment, variant)
            .ok()
            .flatten()
            .is_some()
        {
            cache_hits += 1;
            continue;
        }

        let mut buffer = asset.render(size).ok()?;
        apply_layer_spec(layer, &mut buffer);
        store_cached_generated_render_variant(generated, size, treatment, &buffer, variant).ok()?;
        warmed_sizes += 1;
    }

    Some(LayeredPrewarmReport {
        elapsed_ms: elapsed_ms(started_at),
        probed_outputs: sizes.len(),
        cache_hits,
        warmed_sizes,
    })
}

fn prewarm_jobs(
    background: &veila_common::config::BackgroundConfig,
    outputs: &[output_probe::ProbedOutput],
) -> Vec<PrewarmJob> {
    let mut jobs = Vec::new();
    let all_sizes: Vec<_> = outputs.iter().map(|output| output.size).collect();

    if let Some(path) = background.resolved_path() {
        merge_prewarm_job(&mut jobs, path, &all_sizes);
    }

    for output_config in &background.outputs {
        let sizes: Vec<_> = outputs
            .iter()
            .filter(|output| output.name.as_deref() == Some(output_config.name.as_str()))
            .map(|output| output.size)
            .collect();
        merge_prewarm_job(&mut jobs, output_config.path.clone(), &sizes);
    }

    jobs
}

fn merge_prewarm_job(jobs: &mut Vec<PrewarmJob>, path: PathBuf, sizes: &[FrameSize]) {
    if let Some(job) = jobs.iter_mut().find(|job| job.path == path) {
        for size in sizes {
            if !job.sizes.contains(size) {
                job.sizes.push(*size);
            }
        }
        return;
    }

    jobs.push(PrewarmJob {
        path,
        sizes: sizes.to_vec(),
    });
}

fn generated_sizes(
    background: &veila_common::config::BackgroundConfig,
    outputs: &[output_probe::ProbedOutput],
) -> Vec<FrameSize> {
    let mut sizes = Vec::with_capacity(outputs.len());

    for output in outputs {
        let overridden = output.name.as_deref().is_some_and(|name| {
            background
                .outputs
                .iter()
                .any(|override_config| override_config.name == name)
        });
        if overridden || sizes.contains(&output.size) {
            continue;
        }
        sizes.push(output.size);
    }

    sizes
}

fn apply_layer_spec(layer: &LayerPrewarmSpec, buffer: &mut veila_renderer::SoftwareBuffer) {
    let frame_width = buffer.size().width as i32;
    let frame_height = buffer.size().height as i32;
    let left_margin = layer.left_margin.clamp(0, frame_width.max(0));
    let right_margin = layer.right_margin.clamp(0, frame_width.max(0));
    let top_margin = layer.top_margin.clamp(0, frame_height.max(0));
    let bottom_margin = layer.bottom_margin.clamp(0, frame_height.max(0));
    let safe_left = left_margin;
    let safe_right = (frame_width - right_margin).max(safe_left + 1);
    let safe_width = (safe_right - safe_left).max(1);
    let width = if layer.full_width {
        safe_width
    } else {
        layer
            .width
            .unwrap_or((frame_width as f32 * 0.36) as i32)
            .clamp(1, safe_width)
    };
    let offset_x = layer.offset_x;
    let unclamped_x = match layer.alignment {
        LayerAlignment::Left => safe_left + offset_x,
        LayerAlignment::Center => safe_left + (safe_width - width) / 2 + offset_x,
        LayerAlignment::Right => safe_right - width + offset_x,
    };
    let x = unclamped_x.clamp(safe_left - width + 1, safe_right - 1);
    let y = top_margin.min(frame_height.saturating_sub(1));
    let height = (frame_height - top_margin - bottom_margin).max(1);
    let mode = match layer.mode {
        LayerMode::Solid => BackdropLayerMode::Solid,
        LayerMode::Blur => BackdropLayerMode::Blur,
    };
    let alignment = match layer.alignment {
        LayerAlignment::Left => BackdropLayerAlignment::Left,
        LayerAlignment::Center => BackdropLayerAlignment::Center,
        LayerAlignment::Right => BackdropLayerAlignment::Right,
    };
    let shape = match layer.style {
        LayerStyle::Panel => BackdropLayerShape::Panel,
        LayerStyle::Diagonal => BackdropLayerShape::Diagonal(alignment),
    };

    draw_backdrop_layer(
        buffer,
        Rect::new(x, y, width, height),
        BackdropLayerStyle::new(
            mode,
            shape,
            layer.color,
            layer.blur_radius,
            layer.radius,
            layer.border_color,
            layer.border_width,
        ),
    );
}

fn layer_prewarm_spec(config: &AppConfig) -> Option<LayerPrewarmSpec> {
    if !config.visuals.layer_enabled() {
        return None;
    }

    let raw_color = config.visuals.layer_color().unwrap_or(config.visuals.panel);
    let color = to_layer_color(raw_color, config.visuals.layer_opacity());
    let border_color = config.visuals.layer_border_color().map(to_clear_color);
    Some(LayerPrewarmSpec {
        variant: format!(
            "layer:v3:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}",
            config.visuals.layer_style(),
            config.visuals.layer_mode(),
            config.visuals.layer_alignment(),
            config.visuals.layer_full_width(),
            config.visuals.layer_width(),
            config.visuals.layer_offset_x(),
            config.visuals.layer_left_margin(),
            config.visuals.layer_right_margin(),
            config.visuals.layer_top_margin(),
            config.visuals.layer_bottom_margin(),
            config.visuals.layer_radius(),
            color.red,
            color.green,
            color.blue,
            color.alpha,
            config.visuals.layer_blur_radius().unwrap_or(12),
            border_color,
            config.visuals.layer_border_width().unwrap_or(0),
        ),
        mode: config.visuals.layer_mode(),
        style: config.visuals.layer_style(),
        alignment: config.visuals.layer_alignment(),
        full_width: config.visuals.layer_full_width(),
        width: config.visuals.layer_width().map(i32::from),
        offset_x: i32::from(config.visuals.layer_offset_x().unwrap_or(0)),
        left_margin: i32::from(config.visuals.layer_left_margin().unwrap_or(0)),
        right_margin: i32::from(config.visuals.layer_right_margin().unwrap_or(0)),
        top_margin: i32::from(config.visuals.layer_top_margin().unwrap_or(0)),
        bottom_margin: i32::from(config.visuals.layer_bottom_margin().unwrap_or(0)),
        color,
        blur_radius: config.visuals.layer_blur_radius().unwrap_or(12),
        radius: i32::from(config.visuals.layer_radius().unwrap_or(0)),
        border_color,
        border_width: i32::from(config.visuals.layer_border_width().unwrap_or(0)),
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
        tint: config.tint.map(to_clear_color),
        tint_opacity: config.tint_opacity,
    }
}

fn background_generated(
    config: &veila_common::config::BackgroundConfig,
) -> Option<GeneratedBackground> {
    if let Some(gradient) = config.resolved_gradient() {
        return Some(GeneratedBackground::Gradient(BackgroundGradient {
            top_left: to_clear_color(gradient.top_left),
            top_right: to_clear_color(gradient.top_right),
            bottom_left: to_clear_color(gradient.bottom_left),
            bottom_right: to_clear_color(gradient.bottom_right),
        }));
    }

    config.resolved_radial().map(|radial| {
        GeneratedBackground::Radial(BackgroundRadial {
            center: to_clear_color(radial.center),
            edge: to_clear_color(radial.edge),
            center_x: radial.center_x,
            center_y: radial.center_y,
            radius: radial.radius,
        })
    })
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

struct PrewarmReport {
    path: PathBuf,
    source_status: SourceCacheStatus,
    source_elapsed_ms: u64,
    rendered: Option<RenderedPrewarmReport>,
    layered: Option<LayeredPrewarmReport>,
}

struct PrewarmResult {
    wallpapers: Vec<Result<PrewarmReport, (PathBuf, anyhow::Error)>>,
    generated: Option<GeneratedPrewarmReport>,
}

struct PrewarmJob {
    path: PathBuf,
    sizes: Vec<FrameSize>,
}

struct RenderedPrewarmReport {
    elapsed_ms: u64,
    probed_outputs: usize,
    summary: RenderCacheSummary,
}

struct GeneratedPrewarmReport {
    mode: &'static str,
    rendered: RenderedPrewarmReport,
    layered: Option<LayeredPrewarmReport>,
}

struct LayerPrewarmSpec {
    variant: String,
    mode: LayerMode,
    style: LayerStyle,
    alignment: LayerAlignment,
    full_width: bool,
    width: Option<i32>,
    offset_x: i32,
    left_margin: i32,
    right_margin: i32,
    top_margin: i32,
    bottom_margin: i32,
    color: ClearColor,
    blur_radius: u8,
    radius: i32,
    border_color: Option<ClearColor>,
    border_width: i32,
}

struct LayeredPrewarmReport {
    elapsed_ms: u64,
    probed_outputs: usize,
    cache_hits: usize,
    warmed_sizes: usize,
}
