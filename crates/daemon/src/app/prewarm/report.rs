use std::{path::PathBuf, time::Instant};
use veila_common::elapsed_ms;
use veila_renderer::background::{RenderCacheSummary, SourceCacheStatus};

pub(super) struct PrewarmReport {
    pub(super) path: PathBuf,
    pub(super) source_status: SourceCacheStatus,
    pub(super) source_elapsed_ms: u64,
    pub(super) rendered: Option<RenderedPrewarmReport>,
    pub(super) layered: Option<LayeredPrewarmReport>,
}

pub(super) struct PrewarmResult {
    pub(super) wallpapers: Vec<Result<PrewarmReport, (PathBuf, anyhow::Error)>>,
    pub(super) generated: Option<GeneratedPrewarmReport>,
    pub(super) scene: Option<ScenePrewarmReport>,
}

pub(super) struct RenderedPrewarmReport {
    pub(super) elapsed_ms: u64,
    pub(super) probed_outputs: usize,
    pub(super) summary: RenderCacheSummary,
}

pub(super) struct GeneratedPrewarmReport {
    pub(super) mode: &'static str,
    pub(super) rendered: RenderedPrewarmReport,
    pub(super) layered: Option<LayeredPrewarmReport>,
}

pub(super) struct LayeredPrewarmReport {
    pub(super) elapsed_ms: u64,
    pub(super) probed_outputs: usize,
    pub(super) cache_hits: usize,
    pub(super) warmed_sizes: usize,
}

pub(super) struct ScenePrewarmReport {
    pub(super) elapsed_ms: u64,
    pub(super) probed_outputs: usize,
    pub(super) warmed_sizes: usize,
}

pub(super) fn log_prewarm_report(report: PrewarmReport, prewarm_helper: bool) {
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

pub(super) fn log_generated_prewarm_report(
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

pub(super) fn log_scene_prewarm_report(report: ScenePrewarmReport, prewarm_helper: bool) {
    tracing::info!(
        prewarm_helper,
        elapsed_ms = report.elapsed_ms,
        probed_outputs = report.probed_outputs,
        warmed_sizes = report.warmed_sizes,
        "static scene prewarm finished"
    );
}
