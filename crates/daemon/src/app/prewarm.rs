mod layers;
mod report;
use layers::{prewarm_generated_backgrounds, prewarm_layered_backgrounds};
use report::{
    GeneratedPrewarmReport, LayeredPrewarmReport, PrewarmReport, PrewarmResult,
    RenderedPrewarmReport, ScenePrewarmReport, log_generated_prewarm_report, log_prewarm_report,
    log_scene_prewarm_report,
};

use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use veila_common::{AppConfig, BackdropVisualConfig, LayerVisualConfig, RgbColor, elapsed_ms};
use veila_renderer::{
    ClearColor, FrameSize, SoftwareBuffer,
    background::{BackgroundTreatment, GeneratedBackground, prewarm_rendered, prewarm_source},
};
use veila_ui::{
    ShellState, ShellTheme,
    background::{background_generated, background_treatment},
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
    let scene = ScenePrewarmConfig::from_config(&config);
    let started_at = Instant::now();
    let rss_kib_before = memory::current_rss_kib();
    let join_result = tokio::task::spawn_blocking(move || {
        prewarm_backgrounds(background, generated, fallback, treatment, scene)
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
            if let Some(report) = result.scene {
                log_scene_prewarm_report(report, true);
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
        layer: config.visuals.layer.clone(),
        panel: config.visuals.panel,
    }
}

fn prewarm_backgrounds(
    background: veila_common::config::BackgroundConfig,
    generated: Option<GeneratedBackground>,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    scene: ScenePrewarmConfig,
) -> PrewarmResult {
    let outputs = output_probe::current_outputs().unwrap_or_default();
    let scene_shell = scene.into_shell();
    let wallpapers = prewarm_jobs(&background, &outputs)
        .into_iter()
        .map(|job| prewarm_wallpaper(job, fallback, treatment, &scene_shell))
        .collect();
    let generated = generated.and_then(|generated| {
        let sizes = generated_sizes(&background, &outputs);
        prewarm_generated_backgrounds(generated, treatment, &scene_shell, &sizes)
    });
    let scene = prewarm_static_scene(&scene_shell, &outputs);

    PrewarmResult {
        wallpapers,
        generated,
        scene,
    }
}

fn prewarm_wallpaper(
    job: PrewarmJob,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    shell: &ShellState,
) -> Result<PrewarmReport, (PathBuf, anyhow::Error)> {
    let source_started_at = Instant::now();
    match prewarm_source(&job.path) {
        Ok(status) => {
            let source_elapsed_ms = elapsed_ms(source_started_at);
            let rendered =
                prewarm_rendered_backgrounds(&job.path, fallback, treatment, &job.buffer_sizes());
            let layered =
                prewarm_layered_backgrounds(&job.path, fallback, treatment, shell, &job.sizes);
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

fn prewarm_static_scene(
    shell: &ShellState,
    outputs: &[output_probe::ProbedOutput],
) -> Option<ScenePrewarmReport> {
    if outputs.is_empty() {
        return None;
    }

    let started_at = Instant::now();
    let sizes = unique_prewarm_sizes(outputs);
    let mut warmed_sizes = 0usize;

    for size in &sizes {
        let mut buffer = SoftwareBuffer::solid(size.buffer, ClearColor::opaque(0, 0, 0)).ok()?;
        shell.render_static_overlay_scaled(&mut buffer, size.scale.max(1) as u32);
        warmed_sizes += 1;
    }

    Some(ScenePrewarmReport {
        elapsed_ms: elapsed_ms(started_at),
        probed_outputs: outputs.len(),
        warmed_sizes,
    })
}

fn unique_buffer_sizes(sizes: &[PrewarmSize]) -> Vec<FrameSize> {
    let mut unique = Vec::new();
    for size in sizes {
        if !unique.contains(&size.buffer) {
            unique.push(size.buffer);
        }
    }
    unique
}

fn unique_prewarm_sizes(outputs: &[output_probe::ProbedOutput]) -> Vec<PrewarmSize> {
    let mut sizes = Vec::new();
    for output in outputs {
        let size = PrewarmSize::from(output);
        if !sizes.contains(&size) {
            sizes.push(size);
        }
    }
    sizes
}

fn prewarm_jobs(
    background: &veila_common::config::BackgroundConfig,
    outputs: &[output_probe::ProbedOutput],
) -> Vec<PrewarmJob> {
    let mut jobs = Vec::new();
    let all_sizes: Vec<_> = outputs.iter().map(PrewarmSize::from).collect();

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
            .map(PrewarmSize::from)
            .collect();
        merge_prewarm_job(&mut jobs, output_config.path.clone(), &sizes);
    }

    jobs
}

fn merge_prewarm_job(jobs: &mut Vec<PrewarmJob>, path: PathBuf, sizes: &[PrewarmSize]) {
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
) -> Vec<PrewarmSize> {
    let mut sizes = Vec::with_capacity(outputs.len());

    for output in outputs {
        let overridden = output.name.as_deref().is_some_and(|name| {
            background
                .outputs
                .iter()
                .any(|override_config| override_config.name == name)
        });
        let size = PrewarmSize::from(output);
        if overridden || sizes.contains(&size) {
            continue;
        }
        sizes.push(size);
    }

    sizes
}

fn to_clear_color(color: veila_common::RgbColor) -> ClearColor {
    ClearColor::rgba(color.0, color.1, color.2, color.3)
}

struct PrewarmJob {
    path: PathBuf,
    sizes: Vec<PrewarmSize>,
}

impl PrewarmJob {
    fn buffer_sizes(&self) -> Vec<FrameSize> {
        unique_buffer_sizes(&self.sizes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PrewarmSize {
    buffer: FrameSize,
    scale: i32,
}

impl From<&output_probe::ProbedOutput> for PrewarmSize {
    fn from(output: &output_probe::ProbedOutput) -> Self {
        Self {
            buffer: output.size,
            scale: output.scale.max(1),
        }
    }
}

struct ScenePrewarmConfig {
    theme: ShellTheme,
    input_placeholder: Option<String>,
    username_override: Option<String>,
    avatar_path: Option<PathBuf>,
    username_enabled: bool,
    weather_location: Option<String>,
    weather_unit: veila_common::WeatherUnit,
}

impl ScenePrewarmConfig {
    fn from_config(config: &AppConfig) -> Self {
        Self {
            theme: ShellTheme::from_config(config),
            input_placeholder: Some(config.visuals.input_placeholder()),
            username_override: config.visuals.username_text().map(str::to_owned),
            avatar_path: config.avatar_image_path().map(Path::to_path_buf),
            username_enabled: config.visuals.username_enabled(),
            weather_location: config.weather.normalized_location(),
            weather_unit: config.weather.unit,
        }
    }

    fn into_shell(self) -> ShellState {
        ShellState::new_with_username_and_widgets(
            self.theme,
            self.input_placeholder,
            self.username_override,
            self.avatar_path,
            self.username_enabled,
            self.weather_location,
            None,
            self.weather_unit,
            None,
            None,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BackgroundPrewarmInputs {
    background: veila_common::config::BackgroundConfig,
    backdrop: Vec<BackdropVisualConfig>,
    layer: Vec<LayerVisualConfig>,
    panel: RgbColor,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod parity_tests;
