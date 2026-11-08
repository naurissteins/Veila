use std::time::Duration;

use super::{CacheKind, CachePrunePolicy, CachePruneReport, prune_cache};
use std::{sync::Once, thread};

const PRUNE_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const MAX_CACHE_AGE: Duration = Duration::from_secs(14 * 24 * 60 * 60);
static STARTED: Once = Once::new();

/// starts at most one best-effort maintenance worker; callers must defer until readiness
pub fn spawn_cache_maintenance(initial_delay: Duration) {
    STARTED.call_once(|| {
        if let Err(error) = thread::Builder::new()
            .name("veila-cache-prune".into())
            .spawn(move || {
                thread::sleep(initial_delay);
                loop {
                    prune_once();
                    thread::sleep(PRUNE_INTERVAL);
                }
            })
        {
            tracing::warn!(%error, "image cache maintenance unavailable");
        }
    });
}

fn prune_once() {
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

#[cfg(test)]
mod tests;
