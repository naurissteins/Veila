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
    pub(crate) fn handle_lock_activity(&mut self, queue_handle: &QueueHandle<Self>) -> bool {
        let woke_outputs = self.screen_off.note_activity(Instant::now());
        if !woke_outputs {
            return false;
        }

        if self.set_outputs_power_mode(zwlr_output_power_v1::Mode::On) {
            tracing::info!("woke locked outputs on input activity");
        }
        self.render_all_surfaces(queue_handle);
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
        if !self.screen_off.enabled() {
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
        requested
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::ScreenOffState;

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
}
