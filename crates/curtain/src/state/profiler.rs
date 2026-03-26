#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RenderTimingSample {
    pub(crate) first_frame: bool,
    pub(crate) background_prepare_ms: u64,
    pub(crate) static_overlay_prepare_ms: u64,
    pub(crate) background_restore_ms: u64,
    pub(crate) static_overlay_blend_ms: u64,
    pub(crate) dynamic_overlay_ms: u64,
    pub(crate) shm_pool_prepare_ms: u64,
    pub(crate) commit_ms: u64,
    pub(crate) total_ms: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct StageTimingStats {
    total_ms: u128,
    max_ms: u64,
}

#[derive(Debug, Default)]
pub(crate) struct RenderProfiler {
    frames_rendered: u64,
    first_frames: u64,
    background_prepare: StageTimingStats,
    static_overlay_prepare: StageTimingStats,
    background_restore: StageTimingStats,
    static_overlay_blend: StageTimingStats,
    dynamic_overlay: StageTimingStats,
    shm_pool_prepare: StageTimingStats,
    commit: StageTimingStats,
    total: StageTimingStats,
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

impl RenderProfiler {
    pub(crate) fn record(&mut self, sample: RenderTimingSample) {
        self.frames_rendered = self.frames_rendered.saturating_add(1);
        self.first_frames = self
            .first_frames
            .saturating_add(u64::from(sample.first_frame));
        self.background_prepare.record(sample.background_prepare_ms);
        self.static_overlay_prepare
            .record(sample.static_overlay_prepare_ms);
        self.background_restore.record(sample.background_restore_ms);
        self.static_overlay_blend
            .record(sample.static_overlay_blend_ms);
        self.dynamic_overlay.record(sample.dynamic_overlay_ms);
        self.shm_pool_prepare.record(sample.shm_pool_prepare_ms);
        self.commit.record(sample.commit_ms);
        self.total.record(sample.total_ms);
    }

    pub(crate) fn log_summary(&self) {
        if self.frames_rendered == 0 {
            return;
        }

        let average_stages = [
            (
                "background_prepare_ms",
                self.background_prepare.average_ms(self.frames_rendered),
            ),
            (
                "static_overlay_prepare_ms",
                self.static_overlay_prepare.average_ms(self.frames_rendered),
            ),
            (
                "background_restore_ms",
                self.background_restore.average_ms(self.frames_rendered),
            ),
            (
                "static_overlay_blend_ms",
                self.static_overlay_blend.average_ms(self.frames_rendered),
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
            (
                "static_overlay_prepare_ms",
                self.static_overlay_prepare.max_ms,
            ),
            ("background_restore_ms", self.background_restore.max_ms),
            ("static_overlay_blend_ms", self.static_overlay_blend.max_ms),
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
            static_overlay_prepare_avg_ms =
                self.static_overlay_prepare.average_ms(self.frames_rendered),
            static_overlay_prepare_max_ms = self.static_overlay_prepare.max_ms,
            background_restore_avg_ms = self.background_restore.average_ms(self.frames_rendered),
            background_restore_max_ms = self.background_restore.max_ms,
            static_overlay_blend_avg_ms =
                self.static_overlay_blend.average_ms(self.frames_rendered),
            static_overlay_blend_max_ms = self.static_overlay_blend.max_ms,
            dynamic_overlay_avg_ms = self.dynamic_overlay.average_ms(self.frames_rendered),
            dynamic_overlay_max_ms = self.dynamic_overlay.max_ms,
            shm_pool_prepare_avg_ms = self.shm_pool_prepare.average_ms(self.frames_rendered),
            shm_pool_prepare_max_ms = self.shm_pool_prepare.max_ms,
            commit_avg_ms = self.commit.average_ms(self.frames_rendered),
            commit_max_ms = self.commit.max_ms,
            slowest_avg_stage = slowest_avg_stage.0,
            slowest_avg_stage_ms = slowest_avg_stage.1,
            slowest_max_stage = slowest_max_stage.0,
            slowest_max_stage_ms = slowest_max_stage.1,
            "curtain render timing summary"
        );
    }
}
