#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RenderTimingSample {
    pub(crate) first_frame: bool,
    pub(crate) background_prepare_ms: u64,
    pub(crate) background_restore_ms: u64,
    pub(crate) dynamic_overlay_ms: u64,
    pub(crate) shm_pool_prepare_ms: u64,
    pub(crate) commit_ms: u64,
    pub(crate) total_ms: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DirtyRenderTimingSample {
    pub(crate) dynamic_overlay_ms: u64,
    pub(crate) commit_ms: u64,
    pub(crate) total_ms: u64,
    pub(crate) dirty_pixels: u64,
    pub(crate) dirty_bytes: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct StageTimingStats {
    total_ms: u128,
    max_ms: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct CounterStats {
    total: u128,
    max: u64,
}

#[derive(Debug, Default)]
pub(crate) struct RenderProfiler {
    frames_rendered: u64,
    first_frames: u64,
    background_prepare: StageTimingStats,
    background_restore: StageTimingStats,
    dynamic_overlay: StageTimingStats,
    shm_pool_prepare: StageTimingStats,
    commit: StageTimingStats,
    total: StageTimingStats,
    dirty_frames_rendered: u64,
    dirty_dynamic_overlay: StageTimingStats,
    dirty_commit: StageTimingStats,
    dirty_total: StageTimingStats,
    dirty_pixels: CounterStats,
    dirty_bytes: CounterStats,
}

impl StageTimingStats {
    fn record(&mut self, elapsed_ms: u64) {
        self.total_ms = self.total_ms.saturating_add(u128::from(elapsed_ms));
        self.max_ms = self.max_ms.max(elapsed_ms);
    }

    fn average_ms(self, frames: u64) -> u64 {
        if frames == 0 {
            0
        } else {
            (self.total_ms / u128::from(frames)).min(u128::from(u64::MAX)) as u64
        }
    }
}

impl CounterStats {
    fn record(&mut self, value: u64) {
        self.total = self.total.saturating_add(u128::from(value));
        self.max = self.max.max(value);
    }

    fn average(self, frames: u64) -> u64 {
        if frames == 0 {
            0
        } else {
            (self.total / u128::from(frames)).min(u128::from(u64::MAX)) as u64
        }
    }
}

impl RenderProfiler {
    pub(crate) fn record(&mut self, sample: RenderTimingSample) {
        self.frames_rendered = self.frames_rendered.saturating_add(1);
        self.first_frames = self
            .first_frames
            .saturating_add(u64::from(sample.first_frame));
        self.background_prepare.record(sample.background_prepare_ms);
        self.background_restore.record(sample.background_restore_ms);
        self.dynamic_overlay.record(sample.dynamic_overlay_ms);
        self.shm_pool_prepare.record(sample.shm_pool_prepare_ms);
        self.commit.record(sample.commit_ms);
        self.total.record(sample.total_ms);
    }

    pub(crate) fn record_dirty(&mut self, sample: DirtyRenderTimingSample) {
        self.dirty_frames_rendered = self.dirty_frames_rendered.saturating_add(1);
        self.dirty_dynamic_overlay.record(sample.dynamic_overlay_ms);
        self.dirty_commit.record(sample.commit_ms);
        self.dirty_total.record(sample.total_ms);
        self.dirty_pixels.record(sample.dirty_pixels);
        self.dirty_bytes.record(sample.dirty_bytes);
    }

    pub(crate) fn log_summary(&self) {
        if self.frames_rendered == 0 && self.dirty_frames_rendered == 0 {
            return;
        }

        let average_stages = [
            (
                "background_prepare_ms",
                self.background_prepare.average_ms(self.frames_rendered),
            ),
            (
                "background_restore_ms",
                self.background_restore.average_ms(self.frames_rendered),
            ),
            (
                "dynamic_overlay_ms",
                self.dynamic_overlay.average_ms(self.frames_rendered),
            ),
            (
                "shm_pool_prepare_ms",
                self.shm_pool_prepare.average_ms(self.frames_rendered),
            ),
            ("commit_ms", self.commit.average_ms(self.frames_rendered)),
        ];
        let max_stages = [
            ("background_prepare_ms", self.background_prepare.max_ms),
            ("background_restore_ms", self.background_restore.max_ms),
            ("dynamic_overlay_ms", self.dynamic_overlay.max_ms),
            ("shm_pool_prepare_ms", self.shm_pool_prepare.max_ms),
            ("commit_ms", self.commit.max_ms),
        ];
        let slowest_avg_stage = average_stages
            .iter()
            .max_by_key(|(_, elapsed_ms)| *elapsed_ms)
            .copied()
            .unwrap_or(("total_ms", 0));
        let slowest_max_stage = max_stages
            .iter()
            .max_by_key(|(_, elapsed_ms)| *elapsed_ms)
            .copied()
            .unwrap_or(("total_ms", 0));

        tracing::debug!(
            frames_rendered = self.frames_rendered,
            first_frames = self.first_frames,
            total_avg_ms = self.total.average_ms(self.frames_rendered),
            total_max_ms = self.total.max_ms,
            background_prepare_avg_ms = self.background_prepare.average_ms(self.frames_rendered),
            background_prepare_max_ms = self.background_prepare.max_ms,
            background_restore_avg_ms = self.background_restore.average_ms(self.frames_rendered),
            background_restore_max_ms = self.background_restore.max_ms,
            dynamic_overlay_avg_ms = self.dynamic_overlay.average_ms(self.frames_rendered),
            dynamic_overlay_max_ms = self.dynamic_overlay.max_ms,
            shm_pool_prepare_avg_ms = self.shm_pool_prepare.average_ms(self.frames_rendered),
            shm_pool_prepare_max_ms = self.shm_pool_prepare.max_ms,
            commit_avg_ms = self.commit.average_ms(self.frames_rendered),
            commit_max_ms = self.commit.max_ms,
            dirty_frames_rendered = self.dirty_frames_rendered,
            dirty_total_avg_ms = self.dirty_total.average_ms(self.dirty_frames_rendered),
            dirty_total_max_ms = self.dirty_total.max_ms,
            dirty_dynamic_overlay_avg_ms = self
                .dirty_dynamic_overlay
                .average_ms(self.dirty_frames_rendered),
            dirty_dynamic_overlay_max_ms = self.dirty_dynamic_overlay.max_ms,
            dirty_commit_avg_ms = self.dirty_commit.average_ms(self.dirty_frames_rendered),
            dirty_commit_max_ms = self.dirty_commit.max_ms,
            dirty_pixels_avg = self.dirty_pixels.average(self.dirty_frames_rendered),
            dirty_pixels_max = self.dirty_pixels.max,
            dirty_bytes_avg = self.dirty_bytes.average(self.dirty_frames_rendered),
            dirty_bytes_max = self.dirty_bytes.max,
            slowest_avg_stage = slowest_avg_stage.0,
            slowest_avg_stage_ms = slowest_avg_stage.1,
            slowest_max_stage = slowest_max_stage.0,
            slowest_max_stage_ms = slowest_max_stage.1,
            "curtain render timing summary"
        );
    }
}
