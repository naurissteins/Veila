use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use veila_common::AppConfig;
use veila_renderer::{
    ClearColor,
    background::{
        BackgroundTreatment, RenderCacheSummary, SourceCacheStatus, prewarm_rendered,
        prewarm_source,
    },
};

use crate::app::output_probe;

pub(super) fn spawn_background_prewarm(config: &AppConfig) {
    let Some(path) = config.background.path.clone() else {
        return;
    };
    let fallback = to_clear_color(config.background.color);
    let treatment = background_treatment(&config.background);

    tokio::spawn(async move {
        let started_at = Instant::now();
        let join_result =
            tokio::task::spawn_blocking(move || prewarm_wallpaper(path, fallback, treatment)).await;

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
) -> Result<PrewarmReport, (PathBuf, anyhow::Error)> {
    let source_started_at = Instant::now();
    match prewarm_source(&path) {
        Ok(status) => {
            let source_elapsed_ms = source_started_at
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            let rendered = prewarm_rendered_backgrounds(&path, fallback, treatment);
            Ok(PrewarmReport {
                path,
                source_status: status,
                source_elapsed_ms,
                rendered,
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
}

struct RenderedPrewarmReport {
    elapsed_ms: u64,
    probed_outputs: usize,
    summary: RenderCacheSummary,
}
