use std::time::{Duration, Instant};

use smithay_client_toolkit::reexports::client::QueueHandle;
use wayland_protocols_wlr::output_power_management::v1::client::zwlr_output_power_v1;

use super::CurtainApp;

#[derive(Debug, Clone)]
pub(crate) struct ScreenOffState {
    delay: Option<Duration>,
    last_activity_at: Option<Instant>,
    outputs_powered_off: bool,
}

impl ScreenOffState {
    pub(crate) fn new(delay: Option<Duration>) -> Self {
        Self {
            delay,
            last_activity_at: None,
            outputs_powered_off: false,
        }
    }

    pub(crate) fn set_delay(&mut self, delay: Option<Duration>, now: Instant) {
        let preserve_powered_off = self.outputs_powered_off && delay.is_some();
        self.delay = delay;
        self.last_activity_at = delay.map(|_| now);
        self.outputs_powered_off = preserve_powered_off;
    }

    pub(crate) fn arm(&mut self, now: Instant) {
        if self.delay.is_some() {
            self.last_activity_at = Some(now);
            self.outputs_powered_off = false;
        }
    }

    pub(crate) fn note_activity(&mut self, now: Instant) -> bool {
        if self.delay.is_none() {
            return false;
        }

        let was_powered_off = self.outputs_powered_off;
        self.outputs_powered_off = false;
        self.last_activity_at = Some(now);
        was_powered_off
    }

    pub(crate) fn record_visible_activity(&mut self, now: Instant) {
        if self.delay.is_none() || self.outputs_powered_off {
            return;
        }

        self.last_activity_at = Some(now);
    }

    pub(crate) fn due_in(&self, now: Instant, session_locked: bool) -> Option<Duration> {
        if !session_locked || self.outputs_powered_off {
            return None;
        }

        let delay = self.delay?;
        let last_activity_at = self.last_activity_at?;
        Some(
            last_activity_at
                .checked_add(delay)
                .unwrap_or(last_activity_at)
                .saturating_duration_since(now),
        )
    }

    pub(crate) fn should_power_off(&self, now: Instant, session_locked: bool) -> bool {
        if !session_locked || self.outputs_powered_off {
            return false;
        }

        let Some(delay) = self.delay else {
            return false;
        };
        let Some(last_activity_at) = self.last_activity_at else {
            return false;
        };

        now >= last_activity_at
            .checked_add(delay)
            .unwrap_or(last_activity_at)
    }

    pub(crate) fn mark_outputs_powered_off(&mut self) {
        self.outputs_powered_off = true;
    }

    pub(crate) fn outputs_powered_off(&self) -> bool {
        self.outputs_powered_off
    }

    pub(crate) fn enabled(&self) -> bool {
        self.delay.is_some()
    }
}

impl CurtainApp {
    pub(crate) fn refresh_power_status_text(&mut self) -> bool {
        let now = Instant::now();
        self.ui_shell
            .set_power_status_text(self.power_status_text(now))
    }

    pub(crate) fn power_status_poll_interval(&self, now: Instant) -> Option<Duration> {
        self.power_status_text(now).map(|_| Duration::from_secs(1))
    }

    pub(crate) fn record_visible_lock_activity(&mut self) {
        self.screen_off.record_visible_activity(Instant::now());
    }

    pub(crate) fn handle_lock_activity(&mut self, queue_handle: &QueueHandle<Self>) -> bool {
        let woke_outputs = self.screen_off.note_activity(Instant::now());
        if !woke_outputs {
            return false;
        }

        if self.set_outputs_power_mode(zwlr_output_power_v1::Mode::On) {
            tracing::info!("woke locked outputs on deliberate input activity");
        }
        self.render_all_surfaces(queue_handle);
        self.maybe_power_off_secondary_outputs();
        true
    }

    pub(crate) fn advance_output_power(&mut self) {
        if !self
            .screen_off
            .should_power_off(Instant::now(), self.session_locked)
        {
            return;
        }

        if self.set_outputs_power_mode(zwlr_output_power_v1::Mode::Off) {
            self.screen_off.mark_outputs_powered_off();
            tracing::info!("powered off locked outputs after inactivity");
        }
    }

    pub(crate) fn outputs_powered_off(&self) -> bool {
        self.screen_off.outputs_powered_off()
    }

    pub(crate) fn bind_output_power_for_surface(
        &self,
        output: &smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput,
        queue_handle: &QueueHandle<Self>,
    ) -> Option<wayland_protocols_wlr::output_power_management::v1::client::zwlr_output_power_v1::ZwlrOutputPowerV1>
    {
        if !self.output_power_control_enabled() {
            return None;
        }

        let manager = self.output_power_manager.get().ok()?;
        Some(manager.get_output_power(output, queue_handle, output.clone()))
    }

    pub(crate) fn set_outputs_power_mode(&mut self, mode: zwlr_output_power_v1::Mode) -> bool {
        let mut requested = false;
        for surface in &self.lock_surfaces {
            let Some(output_power) = surface.output_power.as_ref() else {
                continue;
            };

            output_power.set_mode(mode);
            requested = true;
        }
        if requested && mode == zwlr_output_power_v1::Mode::On {
            self.secondary_outputs_powered_off = false;
        }
        requested
    }

    pub(crate) fn maybe_power_off_secondary_outputs(&mut self) {
        if self.secondary_outputs_powered_off
            || !self.ready_notified
            || !self.session_locked
            || !self.secondary_output_power_enabled()
        {
            return;
        }

        if self.set_secondary_outputs_power_mode(zwlr_output_power_v1::Mode::Off) {
            self.secondary_outputs_powered_off = true;
            tracing::info!("powered off secondary locked outputs");
        } else if self.output_power_manager.get().is_err() {
            tracing::warn!(
                "output power management is unavailable; secondary locked outputs stay on"
            );
        }
    }

    pub(crate) fn output_power_control_enabled(&self) -> bool {
        self.screen_off.enabled() || self.secondary_output_power_enabled()
    }

    pub(crate) fn secondary_output_power_enabled(&self) -> bool {
        self.power_off_secondary_outputs
            && matches!(self.ui_output_mode, veila_common::OutputUiMode::Single)
    }

    fn set_secondary_outputs_power_mode(&mut self, mode: zwlr_output_power_v1::Mode) -> bool {
        let mut requested = false;
        for index in 0..self.lock_surfaces.len() {
            if !self.output_role_for_surface(index).renders_shell()
                && let Some(output_power) = self.lock_surfaces[index].output_power.as_ref()
            {
                output_power.set_mode(mode);
                requested = true;
            }
        }
        requested
    }

    fn power_status_text(&self, now: Instant) -> Option<String> {
        let screen_off = self
            .screen_off
            .due_in(now, self.session_locked)
            .map(format_countdown_duration);
        let suspend = self
            .remote_power_status
            .as_ref()
            .map(|snapshot| format_countdown_seconds(snapshot.suspend_remaining_seconds));

        match (screen_off, suspend) {
            (Some(screen_off), Some(suspend)) => {
                Some(format!("Off in {screen_off} · Suspend in {suspend}"))
            }
            (Some(screen_off), None) => Some(format!("Off in {screen_off}")),
            (None, Some(suspend)) => Some(format!("Suspend in {suspend}")),
            (None, None) => None,
        }
    }
}

fn format_countdown_duration(duration: Duration) -> String {
    format_countdown_seconds(remaining_seconds(duration))
}

fn format_countdown_seconds(seconds: u64) -> String {
    if seconds < 60 {
        return format!("{seconds}s");
    }

    let minutes = seconds / 60;
    let remainder = seconds % 60;
    if remainder == 0 || minutes >= 10 {
        return format!("{minutes}m");
    }

    format!("{minutes}m {remainder}s")
}

fn remaining_seconds(duration: Duration) -> u64 {
    let seconds = duration.as_secs();
    if duration.subsec_nanos() == 0 {
        seconds.max(1)
    } else {
        seconds.saturating_add(1)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        ScreenOffState, format_countdown_duration, format_countdown_seconds, remaining_seconds,
    };

    #[test]
    fn due_in_counts_down_until_power_off_deadline() {
        let now = Instant::now();
        let mut state = ScreenOffState::new(Some(Duration::from_secs(5)));
        state.arm(now);

        assert_eq!(
            state.due_in(now + Duration::from_secs(2), true),
            Some(Duration::from_secs(3))
        );
        assert!(state.should_power_off(now + Duration::from_secs(5), true));
    }

    #[test]
    fn waking_activity_reports_when_outputs_were_off() {
        let now = Instant::now();
        let mut state = ScreenOffState::new(Some(Duration::from_secs(5)));
        state.arm(now);
        state.mark_outputs_powered_off();

        assert!(state.note_activity(now + Duration::from_secs(6)));
        assert!(!state.outputs_powered_off());
    }

    #[test]
    fn visible_activity_refreshes_deadline_without_waking_outputs() {
        let now = Instant::now();
        let mut state = ScreenOffState::new(Some(Duration::from_secs(5)));
        state.arm(now);
        state.record_visible_activity(now + Duration::from_secs(2));

        assert_eq!(
            state.due_in(now + Duration::from_secs(4), true),
            Some(Duration::from_secs(3))
        );
    }

    #[test]
    fn visible_activity_is_ignored_while_outputs_are_powered_off() {
        let now = Instant::now();
        let mut state = ScreenOffState::new(Some(Duration::from_secs(5)));
        state.arm(now);
        state.mark_outputs_powered_off();
        state.record_visible_activity(now + Duration::from_secs(2));

        assert!(state.outputs_powered_off());
        assert_eq!(state.due_in(now + Duration::from_secs(2), true), None);
    }

    #[test]
    fn countdown_rounds_partial_seconds_up() {
        assert_eq!(remaining_seconds(Duration::from_millis(100)), 1);
        assert_eq!(remaining_seconds(Duration::from_millis(1_100)), 2);
    }

    #[test]
    fn countdown_formats_compact_text() {
        assert_eq!(format_countdown_seconds(9), "9s");
        assert_eq!(format_countdown_seconds(60), "1m");
        assert_eq!(format_countdown_seconds(65), "1m 5s");
        assert_eq!(format_countdown_duration(Duration::from_secs(600)), "10m");
    }
}
