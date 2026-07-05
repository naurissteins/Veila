use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use veila_renderer::SoftwareBuffer;

#[derive(Debug)]
pub(crate) enum SlideshowTransitionPhase {
    Loading,
    Animating { started_at: Instant },
}

#[derive(Debug)]
pub(crate) struct SlideshowTransition {
    pub(crate) from_buffers: Vec<Option<Arc<SoftwareBuffer>>>,
    pub(crate) to_buffers: Vec<Option<Arc<SoftwareBuffer>>>,
    pub(crate) to_path: PathBuf,
    pub(crate) phase: SlideshowTransitionPhase,
    pub(crate) duration: Duration,
}

impl SlideshowTransition {
    pub(crate) fn new(
        from_buffers: Vec<Option<Arc<SoftwareBuffer>>>,
        to_path: PathBuf,
        duration: Duration,
    ) -> Self {
        let surface_count = from_buffers.len();
        Self {
            from_buffers,
            to_buffers: vec![None; surface_count],
            to_path,
            phase: SlideshowTransitionPhase::Loading,
            duration: duration.max(Duration::from_millis(1)),
        }
    }

    pub(crate) fn is_loading(&self) -> bool {
        matches!(self.phase, SlideshowTransitionPhase::Loading)
    }

    pub(crate) fn is_animating(&self) -> bool {
        matches!(self.phase, SlideshowTransitionPhase::Animating { .. })
    }

    pub(crate) fn all_targets_ready(&self) -> bool {
        !self.to_buffers.is_empty() && self.to_buffers.iter().all(|buffer| buffer.is_some())
    }

    pub(crate) fn mark_animating(&mut self, now: Instant) {
        self.phase = SlideshowTransitionPhase::Animating { started_at: now };
    }

    pub(crate) fn fade_progress(&self, now: Instant) -> Option<u8> {
        let SlideshowTransitionPhase::Animating { started_at } = self.phase else {
            return None;
        };

        let elapsed = now.saturating_duration_since(started_at).min(self.duration);
        Some(
            ((elapsed.as_millis() * 100) / self.duration.as_millis().max(1))
                .min(u128::from(u8::MAX)) as u8,
        )
    }

    pub(crate) fn is_complete(&self, now: Instant) -> bool {
        let SlideshowTransitionPhase::Animating { started_at } = self.phase else {
            return false;
        };

        now.saturating_duration_since(started_at) >= self.duration
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use veila_renderer::{ClearColor, FrameSize, SoftwareBuffer};

    use super::SlideshowTransition;

    #[test]
    fn fade_progress_reaches_full_at_duration() {
        let buffer = Arc::new(
            SoftwareBuffer::solid(FrameSize::new(1, 1), ClearColor::opaque(0, 0, 0))
                .expect("buffer"),
        );
        let mut transition = SlideshowTransition::new(
            vec![Some(buffer.clone())],
            "/tmp/next.jpg".into(),
            std::time::Duration::from_millis(100),
        );
        transition.to_buffers = vec![Some(buffer)];
        let started_at = std::time::Instant::now();
        transition.mark_animating(started_at);

        assert_eq!(transition.fade_progress(started_at), Some(0));
        assert_eq!(
            transition.fade_progress(started_at + std::time::Duration::from_millis(50)),
            Some(50)
        );
        assert_eq!(
            transition.fade_progress(started_at + std::time::Duration::from_millis(100)),
            Some(100)
        );
        assert!(transition.is_complete(started_at + std::time::Duration::from_millis(100)));
    }
}
