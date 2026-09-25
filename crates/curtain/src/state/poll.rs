use std::time::{Duration, Instant};

use crate::state::CurtainApp;

impl CurtainApp {
    pub(crate) fn animation_poll_interval(&self) -> Duration {
        let shell_interval = self.ui_shell.animation_poll_interval();
        let now = Instant::now();
        let repeat_interval = self
            .backspace_repeat
            .as_ref()
            .map(|backspace_repeat| backspace_repeat.due_in(now))
            .unwrap_or(shell_interval);
        let slideshow_interval = self
            .slideshow
            .as_ref()
            .and_then(|slideshow| slideshow.next_due_in(now))
            .unwrap_or(shell_interval);
        let screen_off_interval = self
            .screen_off
            .due_in(now, self.session_locked)
            .unwrap_or(shell_interval);
        let power_status_interval = self
            .power_status_poll_interval(now)
            .unwrap_or(shell_interval);
        let shm_trim_interval = self.shm_trim_due_in(now).unwrap_or(shell_interval);

        shell_interval
            .min(repeat_interval)
            .min(slideshow_interval)
            .min(screen_off_interval)
            .min(power_status_interval)
            .min(shm_trim_interval)
    }
}
