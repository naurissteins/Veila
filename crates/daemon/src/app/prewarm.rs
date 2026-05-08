use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use veila_common::{
    AppConfig, BackdropMode, BackdropVisualConfig, HorizontalAlign, RgbColor, VerticalAlign,
    config::{
        BackgroundLayeredBaseMode, BackgroundLayeredConfig,
        BackgroundScaling as ConfigBackgroundScaling,
    },
};
use veila_renderer::{
    ClearColor, FrameSize,
    background::{
        BackgroundAsset, BackgroundGradient, BackgroundLayered, BackgroundLayeredBase,
        BackgroundLayeredBlob, BackgroundRadial, BackgroundScaling, BackgroundTreatment,
        GeneratedBackground, RenderCacheSummary, SourceCacheStatus,
        load_cached_generated_render_variant, load_cached_render_variant, prewarm_rendered,
        prewarm_rendered_generated, prewarm_source, store_cached_generated_render_variant,
        store_cached_render_variant,
    },
    draw::layer::{BackdropLayerMode, BackdropLayerShape, BackdropLayerStyle, draw_backdrop_layer},
    shape::Rect,
};

use crate::app::output_probe;

use super::memory;
use crate::adapters::process;

pub(super) fn spawn_background_prewarm(config_path: Option<&Path>) {
    let config_path = config_path.map(Path::to_path_buf);
    let rss_kib_before_spawn = memory::current_rss_kib();

    tokio::spawn(async move {
        match process::spawn_background_prewarm_helper(config_path.as_deref()).await {
            Ok(mut child) => match child.wait().await {
                Ok(status) => {
                    tracing::debug!(
                        ?status,
                        rss_kib_before_spawn,
                        rss_kib_after = memory::current_rss_kib(),
                        "background prewarm helper finished"
                    );
                }
                Err(error) => {
                    tracing::warn!("failed while waiting for background prewarm helper: {error:#}");
                }
            },
            Err(error) => {
                tracing::warn!("failed to spawn background prewarm helper: {error:#}");
            }
        }
    });
}

pub(super) async fn run_background_prewarm_once(config: AppConfig) {
    let background = config.background.clone();
    let fallback = to_clear_color(config.background.color);
    let generated = background_generated(&config.background);
    let treatment = background_treatment(&config.background);
    let backdrops = backdrop_prewarm_specs(&config);
    let started_at = Instant::now();
    let rss_kib_before = memory::current_rss_kib();
    let join_result = tokio::task::spawn_blocking(move || {
        prewarm_backgrounds(background, generated, fallback, treatment, backdrops)
    })
    .await;

    match join_result {
        Ok(result) => {
            for report in result.wallpapers {
                match report {
                    Ok(report) => log_prewarm_report(report, true),
                    Err((path, error)) => {
                        tracing::warn!(
                            prewarm_helper = true,
                            path = %path.display(),
                            elapsed_ms = elapsed_ms(started_at),
                            "background source prewarm failed: {error:#}"
                        );
                    }
                }
            }
            if let Some(report) = result.generated {
                log_generated_prewarm_report(report, started_at, true);
            }
            tracing::debug!(
                prewarm_helper = true,
                elapsed_ms = elapsed_ms(started_at),
                rss_kib_before,
                rss_kib_after = memory::current_rss_kib(),
                "background prewarm helper task completed"
            );
        }
        Err(error) => {
            tracing::warn!(
                prewarm_helper = true,
                "background source prewarm helper task failed: {error:#}"
            );
        }
    }
}

pub(super) fn prewarm_inputs_changed(current: &AppConfig, next: &AppConfig) -> bool {
    prewarm_inputs(current) != prewarm_inputs(next)
}

fn prewarm_inputs(config: &AppConfig) -> BackgroundPrewarmInputs {
    BackgroundPrewarmInputs {
        background: config.background.clone(),
        backdrop: config.visuals.backdrop.clone(),
        panel: config.visuals.panel,
    }
}

fn log_prewarm_report(report: PrewarmReport, prewarm_helper: bool) {
    tracing::info!(
        prewarm_helper,
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
            prewarm_helper,
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
            prewarm_helper,
            path = %report.path.display(),
            elapsed_ms = layered.elapsed_ms,
            probed_outputs = layered.probed_outputs,
            cache_hits = layered.cache_hits,
            warmed_sizes = layered.warmed_sizes,
            "layered background prewarm finished"
        );
    }
}

fn log_generated_prewarm_report(
    report: GeneratedPrewarmReport,
    started_at: Instant,
    prewarm_helper: bool,
) {
    tracing::info!(
        prewarm_helper,
        elapsed_ms = report.rendered.elapsed_ms,
        probed_outputs = report.rendered.probed_outputs,
        cache_hits = report.rendered.summary.cache_hits,
        warmed_sizes = report.rendered.summary.warmed_sizes,
        generated_mode = report.mode,
        "generated background render prewarm finished"
    );

    if let Some(layered) = report.layered {
        tracing::info!(
            prewarm_helper,
            elapsed_ms = layered.elapsed_ms,
            probed_outputs = layered.probed_outputs,
            cache_hits = layered.cache_hits,
            warmed_sizes = layered.warmed_sizes,
            generated_mode = report.mode,
            "generated layered background prewarm finished"
        );
    }

    tracing::debug!(
        prewarm_helper,
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
    backdrops: Vec<BackdropPrewarmSpec>,
) -> PrewarmResult {
    let outputs = output_probe::current_outputs().unwrap_or_default();
    let wallpapers = prewarm_jobs(&background, &outputs)
        .into_iter()
        .map(|job| prewarm_wallpaper(job, fallback, treatment, &backdrops))
        .collect();
    let generated = generated.and_then(|generated| {
        let sizes = generated_sizes(&background, &outputs);
        prewarm_generated_backgrounds(generated, treatment, &backdrops, &sizes)
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
    backdrops: &[BackdropPrewarmSpec],
) -> Result<PrewarmReport, (PathBuf, anyhow::Error)> {
    let source_started_at = Instant::now();
    match prewarm_source(&job.path) {
        Ok(status) => {
            let source_elapsed_ms = elapsed_ms(source_started_at);
            let rendered = prewarm_rendered_backgrounds(&job.path, fallback, treatment, &job.sizes);
            let layered =
                prewarm_layered_backgrounds(&job.path, fallback, treatment, backdrops, &job.sizes);
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
    backdrops: &[BackdropPrewarmSpec],
    sizes: &[FrameSize],
) -> Option<LayeredPrewarmReport> {
    if backdrops.is_empty() || sizes.is_empty() {
        return None;
    }

    let variant = backdrop_variant(backdrops);
    let started_at = Instant::now();
    let asset = BackgroundAsset::load(Some(path), fallback, None, treatment).ok()?;
    let mut cache_hits = 0usize;
    let mut warmed_sizes = 0usize;

    for size in sizes.iter().copied() {
        if load_cached_render_variant(path, size, treatment, &variant)
            .ok()
            .flatten()
            .is_some()
        {
            cache_hits += 1;
            continue;
        }

        let mut buffer = asset.render(size).ok()?;
        apply_backdrop_specs(backdrops, &mut buffer);
        store_cached_render_variant(path, size, treatment, &buffer, &variant).ok()?;
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
    backdrops: &[BackdropPrewarmSpec],
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
    let layered = prewarm_generated_layered_backgrounds(generated, treatment, backdrops, sizes);

    Some(GeneratedPrewarmReport {
        mode: generated.mode_name(),
        rendered,
        layered,
    })
}

fn prewarm_generated_layered_backgrounds(
    generated: GeneratedBackground,
    treatment: BackgroundTreatment,
    backdrops: &[BackdropPrewarmSpec],
    sizes: &[FrameSize],
) -> Option<LayeredPrewarmReport> {
    if backdrops.is_empty() || sizes.is_empty() {
        return None;
    }

    let variant = backdrop_variant(backdrops);
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
        if load_cached_generated_render_variant(generated, size, treatment, &variant)
            .ok()
            .flatten()
            .is_some()
        {
            cache_hits += 1;
            continue;
        }

        let mut buffer = asset.render(size).ok()?;
        apply_backdrop_specs(backdrops, &mut buffer);
        store_cached_generated_render_variant(generated, size, treatment, &buffer, &variant)
            .ok()?;
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

    if background.slideshow_enabled() {
        if let Ok(Some(path)) = background.resolved_slideshow_initial_path() {
            merge_prewarm_job(&mut jobs, path, &all_sizes);
        }
        return jobs;
    }

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

fn apply_backdrop_specs(
    backdrops: &[BackdropPrewarmSpec],
    buffer: &mut veila_renderer::SoftwareBuffer,
) {
    let frame_width = buffer.size().width as i32;
    let frame_height = buffer.size().height as i32;

    for backdrop in backdrops {
        let mode = match backdrop.mode {
            BackdropMode::Solid => BackdropLayerMode::Solid,
            BackdropMode::Blur => BackdropLayerMode::Blur,
        };

        draw_backdrop_layer(
            buffer,
            Rect::new(
                anchored_block_x(frame_width, backdrop.width, backdrop.halign, backdrop.x),
                anchored_block_y(frame_height, backdrop.height, backdrop.valign, backdrop.y),
                backdrop.width,
                backdrop.height,
            ),
            BackdropLayerStyle::new(
                mode,
                BackdropLayerShape::Panel,
                backdrop.color,
                backdrop.blur_strength,
                backdrop.radius,
                backdrop.border_color,
                backdrop.border_width,
            ),
        );
    }
}

fn backdrop_prewarm_specs(config: &AppConfig) -> Vec<BackdropPrewarmSpec> {
    let mut backdrops = config
        .visuals
        .backdrop
        .iter()
        .filter(|backdrop| backdrop.enabled.unwrap_or(true))
        .map(|backdrop| BackdropPrewarmSpec {
            mode: backdrop.mode.unwrap_or_default(),
            color: to_clear_color(backdrop.color.unwrap_or(config.visuals.panel)),
            blur_strength: backdrop.blur_strength.unwrap_or(12).min(24),
            radius: i32::from(backdrop.radius.unwrap_or(0)).clamp(0, 160),
            border_color: backdrop.border_color.map(to_clear_color),
            border_width: i32::from(backdrop.border_width.unwrap_or(0)).clamp(0, 16),
            width: i32::from(backdrop.width.unwrap_or(560)).max(1),
            height: i32::from(backdrop.height.unwrap_or(600)).max(1),
            halign: backdrop.position.halign.unwrap_or(HorizontalAlign::Center),
            valign: backdrop.position.valign.unwrap_or(VerticalAlign::Top),
            x: i32::from(backdrop.position.x.unwrap_or(0)),
            y: i32::from(backdrop.position.y.unwrap_or(0)),
            z: i32::from(backdrop.z.unwrap_or(0)),
        })
        .collect::<Vec<_>>();
    backdrops.sort_by_key(|backdrop| backdrop.z);
    backdrops
}

fn backdrop_variant(backdrops: &[BackdropPrewarmSpec]) -> String {
    let mut variant = String::from("backdrop:v1");
    for backdrop in backdrops {
        use std::fmt::Write as _;

        let _ = write!(
            &mut variant,
            "|{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}",
            backdrop.mode,
            backdrop.halign,
            backdrop.valign,
            backdrop.width,
            backdrop.height,
            backdrop.x,
            backdrop.y,
            backdrop.z,
            backdrop.color,
            backdrop.blur_strength,
            backdrop.radius,
            backdrop.border_color,
            backdrop.border_width,
        );
    }
    variant
}

fn anchored_block_x(frame_width: i32, width: i32, halign: HorizontalAlign, x: i32) -> i32 {
    match halign {
        HorizontalAlign::Left => x,
        HorizontalAlign::Center => (frame_width - width) / 2 + x,
        HorizontalAlign::Right => frame_width - width + x,
    }
}

fn anchored_block_y(frame_height: i32, height: i32, valign: VerticalAlign, y: i32) -> i32 {
    match valign {
        VerticalAlign::Top => y,
        VerticalAlign::Center => (frame_height - height) / 2 + y,
        VerticalAlign::Bottom => frame_height - height + y,
    }
}

fn to_clear_color(color: veila_common::RgbColor) -> ClearColor {
    ClearColor::rgba(color.0, color.1, color.2, color.3)
}

fn background_treatment(config: &veila_common::config::BackgroundConfig) -> BackgroundTreatment {
    BackgroundTreatment {
        blur_radius: config.blur_radius,
        dim_strength: config.dim_strength,
        tint: config.tint.map(to_clear_color),
        scaling: to_background_scaling(config.scaling),
    }
}

fn to_background_scaling(scaling: ConfigBackgroundScaling) -> BackgroundScaling {
    match scaling {
        ConfigBackgroundScaling::Fill => BackgroundScaling::Fill,
        ConfigBackgroundScaling::Fit => BackgroundScaling::Fit,
        ConfigBackgroundScaling::Center => BackgroundScaling::Center,
        ConfigBackgroundScaling::Tile => BackgroundScaling::Tile,
        ConfigBackgroundScaling::Stretch => BackgroundScaling::Stretch,
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

    if let Some(radial) = config.resolved_radial() {
        return Some(GeneratedBackground::Radial(BackgroundRadial {
            center: to_clear_color(radial.center),
            edge: to_clear_color(radial.edge),
            center_x: radial.center_x,
            center_y: radial.center_y,
            radius: radial.radius,
        }));
    }

    config
        .resolved_layered()
        .map(|layered| GeneratedBackground::Layered(to_layered_background(&layered)))
}

fn to_layered_background(config: &BackgroundLayeredConfig) -> BackgroundLayered {
    let base = match config.base.effective_mode() {
        BackgroundLayeredBaseMode::Gradient => {
            let gradient = config.base.gradient.clone().unwrap_or_default();
            BackgroundLayeredBase::Gradient(BackgroundGradient {
                top_left: to_clear_color(gradient.top_left),
                top_right: to_clear_color(gradient.top_right),
                bottom_left: to_clear_color(gradient.bottom_left),
                bottom_right: to_clear_color(gradient.bottom_right),
            })
        }
        BackgroundLayeredBaseMode::Radial => {
            let radial = config.base.radial.clone().unwrap_or_default();
            BackgroundLayeredBase::Radial(BackgroundRadial {
                center: to_clear_color(radial.center),
                edge: to_clear_color(radial.edge),
                center_x: radial.center_x,
                center_y: radial.center_y,
                radius: radial.radius,
            })
        }
        BackgroundLayeredBaseMode::Solid => {
            BackgroundLayeredBase::Solid(to_clear_color(config.base.color))
        }
    };

    let mut blobs = [None; 3];
    for (slot, blob) in blobs.iter_mut().zip(config.blobs.iter().take(3)) {
        *slot = Some(BackgroundLayeredBlob {
            color: blob_color(blob.color, blob.opacity),
            x: blob.x,
            y: blob.y,
            size: blob.size,
        });
    }

    BackgroundLayered { base, blobs }
}

fn blob_color(color: RgbColor, opacity: u8) -> ClearColor {
    let alpha = ((u16::from(color.3) * u16::from(opacity.min(100)) + 50) / 100) as u8;
    ClearColor::rgba(color.0, color.1, color.2, alpha)
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

struct BackdropPrewarmSpec {
    mode: BackdropMode,
    width: i32,
    height: i32,
    halign: HorizontalAlign,
    valign: VerticalAlign,
    x: i32,
    y: i32,
    z: i32,
    color: ClearColor,
    blur_strength: u8,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct BackgroundPrewarmInputs {
    background: veila_common::config::BackgroundConfig,
    backdrop: Vec<BackdropVisualConfig>,
    panel: RgbColor,
}

#[cfg(test)]
mod tests {
    use veila_common::AppConfig;

    use super::prewarm_inputs_changed;

    #[test]
    fn detects_background_related_prewarm_changes() {
        let current = AppConfig::from_toml_str(
            r#"
                [background]
                mode = "file"
                path = "/tmp/one.jpg"
            "#,
        )
        .expect("current config");
        let next = AppConfig::from_toml_str(
            r#"
                [background]
                mode = "file"
                path = "/tmp/two.jpg"
            "#,
        )
        .expect("next config");

        assert!(prewarm_inputs_changed(&current, &next));
    }

    #[test]
    fn ignores_unrelated_reload_changes_for_prewarm() {
        let current = AppConfig::from_toml_str(
            r##"
                [background]
                mode = "gradient"

                [visuals.clock]
                color = "#FFFFFF"
            "##,
        )
        .expect("current config");
        let next = AppConfig::from_toml_str(
            r##"
                [background]
                mode = "gradient"

                [visuals.clock]
                color = "#FF5353"
            "##,
        )
        .expect("next config");

        assert!(!prewarm_inputs_changed(&current, &next));
    }
}
