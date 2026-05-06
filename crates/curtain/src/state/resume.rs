use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub(crate) struct ResumeInputState {
    armed: bool,
    swallow_input_pending: bool,
    swallow_until: Option<Instant>,
}

impl ResumeInputState {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn arm(&mut self) {
        self.armed = true;
        self.swallow_input_pending = false;
        self.swallow_until = None;
    }

    pub(crate) fn mark_resumed(&mut self) {
        if self.armed {
            self.armed = false;
            self.swallow_input_pending = true;
            self.swallow_until = None;
        }
    }

    pub(crate) fn swallow_input_pending(&self) -> bool {
        self.swallow_input_pending
    }

    pub(crate) fn clear_swallow_input(&mut self) {
        self.swallow_input_pending = false;
    }

    pub(crate) fn begin_grace_period(&mut self, duration: Duration) {
        self.swallow_until = Some(Instant::now() + duration);
    }

    pub(crate) fn grace_period_active(&mut self) -> bool {
        let Some(deadline) = self.swallow_until else {
            return false;
        };
        if Instant::now() < deadline {
            return true;
        }
        self.swallow_until = None;
        false
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::ResumeInputState;

    #[test]
    fn arm_then_resume_arms_single_swallowed_input() {
        let mut state = ResumeInputState::new();

        state.arm();
        state.mark_resumed();

        assert!(state.swallow_input_pending());
    }

    #[test]
    fn resume_without_arm_does_not_swallow_input() {
        let mut state = ResumeInputState::new();

        state.mark_resumed();

        assert!(!state.swallow_input_pending());
    }

    #[test]
    fn rearming_clears_stale_pending_swallow() {
        let mut state = ResumeInputState::new();

        state.arm();
        state.mark_resumed();
        state.arm();

        assert!(!state.swallow_input_pending());
    }

    #[test]
    fn arming_clears_resume_grace_period() {
        let mut state = ResumeInputState::new();

        state.begin_grace_period(Duration::from_secs(1));
        assert!(state.grace_period_active());

        state.arm();

        assert!(!state.grace_period_active());
    }
}
