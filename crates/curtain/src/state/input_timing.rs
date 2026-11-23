use nix::{
    sys::time::TimeValLike,
    time::{ClockId, clock_gettime},
};

#[derive(Default)]
pub(crate) struct InputTiming {
    pub(crate) enabled: bool,
    dispatched_ns: Option<i64>,
}

impl InputTiming {
    pub(crate) fn note_key(&mut self, character: bool) {
        if self.enabled {
            self.dispatched_ns = if character { monotonic_ns() } else { None };
        }
    }

    pub(crate) fn note_commit(&mut self, full_frame: bool, video_active: bool) {
        if let Some(dispatched_ns) = self.dispatched_ns.take()
            && let Some(committed_ns) = monotonic_ns()
        {
            tracing::debug!(
                dispatched_ns,
                committed_ns,
                full_frame,
                video_active,
                "prototype password frame committed"
            );
        }
    }
}

fn monotonic_ns() -> Option<i64> {
    clock_gettime(ClockId::CLOCK_MONOTONIC)
        .ok()
        .map(|stamp| stamp.num_nanoseconds())
}
