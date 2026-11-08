use std::time::{Duration, Instant};

use crate::state::CurtainApp;

impl CurtainApp {
    pub(crate) fn next_event_timeout(&self) -> Option<Duration> {
        let now = Instant::now();
        let lock_deadline =
            (self.lock_acquisition_started && !self.session_locked && !self.session_finished).then(
                || (self.lock_started_at + self.lock_wait_timeout).saturating_duration_since(now),
            );
        [
            self.ui_shell.next_animation_in(now),
            self.backspace_repeat
                .as_ref()
                .map(|repeat| repeat.due_in(now)),
            self.slideshow
                .as_ref()
                .and_then(|slideshow| slideshow.next_due_in(now)),
            self.screen_off.due_in(now, self.session_locked),
            self.power_status_poll_interval(now),
            self.shm_trim_due_in(now),
            lock_deadline,
        ]
        .into_iter()
        .flatten()
        .min()
    }
}
