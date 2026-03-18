use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use kwylock_common::AppConfig;
use kwylock_renderer::{
    ClearColor,
    background::{RenderCacheSummary, SourceCacheStatus, prewarm_rendered, prewarm_source},
};

use crate::app::output_probe;

pub(super) fn spawn_background_prewarm(config: &AppConfig) {
    let Some(path) = config.background.path.clone() else {
        return;
    };
    let fallback = to_clear_color(config.background.color);

    tokio::spawn(async move {
        let started_at = Instant::now();
        let join_result =
            tokio::task::spawn_blocking(move || prewarm_wallpaper(path, fallback)).await;

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
) -> Result<PrewarmReport, (PathBuf, anyhow::Error)> {
    let source_started_at = Instant::now();
    match prewarm_source(&path) {
        Ok(status) => {
            let source_elapsed_ms = source_started_at
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            let rendered = prewarm_rendered_backgrounds(&path, fallback);
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
) -> Option<RenderedPrewarmReport> {
    let sizes = output_probe::current_output_sizes().ok()?;
    if sizes.is_empty() {
        return None;
    }

    let started_at = Instant::now();
    let summary = prewarm_rendered(path, fallback, &sizes).ok()?;
    Some(RenderedPrewarmReport {
        elapsed_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        probed_outputs: sizes.len(),
        summary,
    })
}

fn to_clear_color(color: kwylock_common::RgbColor) -> ClearColor {
    ClearColor::opaque(color.0, color.1, color.2)
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
