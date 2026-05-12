use std::time::Duration;

use tokio::{task, time};
use veila_renderer::background::{RenderCachePrunePolicy, prune_render_cache};

const INITIAL_PRUNE_DELAY: Duration = Duration::from_secs(60);
const PRUNE_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const MAX_RENDER_CACHE_BYTES: u64 = 768 * 1024 * 1024;
const MAX_RENDER_CACHE_AGE: Duration = Duration::from_secs(14 * 24 * 60 * 60);

pub(super) fn spawn_background_cache_pruner() {
    tokio::spawn(async {
        time::sleep(INITIAL_PRUNE_DELAY).await;

        loop {
            prune_once().await;
            time::sleep(PRUNE_INTERVAL).await;
        }
    });
}

async fn prune_once() {
    let policy = RenderCachePrunePolicy {
        max_bytes: MAX_RENDER_CACHE_BYTES,
        max_age: MAX_RENDER_CACHE_AGE,
    };

    match task::spawn_blocking(move || prune_render_cache(policy)).await {
        Ok(Ok(report)) if report.removed_files > 0 => {
            tracing::info!(
                scanned_files = report.scanned_files,
                removed_files = report.removed_files,
                removed_bytes = report.removed_bytes,
                retained_bytes = report.retained_bytes,
                max_bytes = MAX_RENDER_CACHE_BYTES,
                max_age_seconds = MAX_RENDER_CACHE_AGE.as_secs(),
                "pruned rendered background cache"
            );
        }
        Ok(Ok(report)) => {
            tracing::debug!(
                scanned_files = report.scanned_files,
                retained_bytes = report.retained_bytes,
                max_bytes = MAX_RENDER_CACHE_BYTES,
                max_age_seconds = MAX_RENDER_CACHE_AGE.as_secs(),
                "rendered background cache prune completed without removals"
            );
        }
        Ok(Err(error)) => {
            tracing::warn!("failed to prune rendered background cache: {error}");
        }
        Err(error) => {
            tracing::warn!("rendered background cache prune task failed: {error}");
        }
    }
}
