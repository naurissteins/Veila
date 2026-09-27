use std::time::Duration;

use tokio::{task, time};
use veila_renderer::cache::{CacheKind, CachePrunePolicy, CachePruneReport, prune_cache};

const INITIAL_PRUNE_DELAY: Duration = Duration::from_secs(60);
const PRUNE_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const MAX_CACHE_AGE: Duration = Duration::from_secs(14 * 24 * 60 * 60);

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
    let result = task::spawn_blocking(|| {
        for (kind, max_bytes) in [
            (CacheKind::RenderedBackground, 768 * 1024 * 1024),
            (CacheKind::SourceImage, 512 * 1024 * 1024),
            (CacheKind::Avatar, 512 * 1024 * 1024),
        ] {
            let policy = CachePrunePolicy {
                max_bytes,
                max_age: MAX_CACHE_AGE,
            };
            match prune_cache(kind, policy) {
                Ok(report) => log_report(kind, policy, report),
                Err(error) => tracing::warn!(
                    cache = kind.directory(),
                    "failed to prune image cache: {error}"
                ),
            }
        }
    })
    .await;
    if let Err(error) = result {
        tracing::warn!("image cache prune task failed: {error}");
    }
}

fn log_report(kind: CacheKind, policy: CachePrunePolicy, report: CachePruneReport) {
    if report.removed_files > 0 {
        tracing::info!(
            cache = kind.directory(),
            scanned_files = report.scanned_files,
            removed_files = report.removed_files,
            removed_bytes = report.removed_bytes,
            retained_bytes = report.retained_bytes,
            max_bytes = policy.max_bytes,
            max_age_seconds = policy.max_age.as_secs(),
            "pruned image cache"
        );
    } else {
        tracing::debug!(
            cache = kind.directory(),
            scanned_files = report.scanned_files,
            retained_bytes = report.retained_bytes,
            max_bytes = policy.max_bytes,
            "image cache prune completed without removals"
        );
    }
}
