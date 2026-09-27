use std::time::{Duration, Instant};

use crate::state::CurtainApp;

impl CurtainApp {
    pub(crate) fn shm_trim_due_in(&self, now: Instant) -> Option<Duration> {
        self.lock_surfaces
            .iter()
            .filter_map(|surface| surface.shm_pool.as_ref()?.trim_due_in(now))
            .min()
    }

    pub(crate) fn advance_shm_trim(&mut self) {
        let now = Instant::now();
        if self.shm_trim_due_in(now) != Some(Duration::ZERO) {
            return;
        }

        let mut trimmed_bytes = 0_usize;
        for surface in &mut self.lock_surfaces {
            let Some(pool) = surface.shm_pool.as_mut() else {
                continue;
            };
            match pool.trim_released(now) {
                Ok(bytes) => trimmed_bytes = trimmed_bytes.saturating_add(bytes),
                Err(error) => {
                    tracing::warn!(%error, "disabled buffer slot trimming for surface")
                }
            }
        }
        if trimmed_bytes == 0 {
            return;
        }

        tracing::debug!(
            trimmed_kib = trimmed_bytes / 1024,
            "returned released curtain buffer slots"
        );
        if self.ready_notified && !self.post_ready_trim_logged {
            self.log_memory_snapshot("post-ready-trim");
            self.post_ready_trim_logged = true;
        }
    }
}
