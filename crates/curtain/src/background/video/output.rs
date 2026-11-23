use std::time::{Duration, Instant};

use smithay_client_toolkit::reexports::client::{QueueHandle, protocol::wl_output::WlOutput};

use super::{CurtainApp, Video, VideoHold};
use crate::state::SurfaceSize;

const REBIND_WINDOW: Duration = Duration::from_secs(5);

pub(super) struct VideoOutput {
    name: Option<String>,
    size: SurfaceSize,
    rebind_pending: bool,
    rebind_deadline: Option<Instant>,
}

impl VideoOutput {
    pub(super) fn new(name: Option<String>, size: SurfaceSize) -> Self {
        Self {
            name,
            size,
            rebind_pending: false,
            rebind_deadline: None,
        }
    }

    pub(super) fn sleep_changed(&mut self, sleeping: bool, now: Instant) {
        self.rebind_deadline = (!sleeping).then_some(now + REBIND_WINDOW);
    }

    fn can_rebind(&self, sleeping: bool, now: Instant) -> bool {
        self.name.is_some() && (sleeping || self.rebind_deadline.is_some_and(|end| now < end))
    }

    fn confirm_rebind(&mut self) {
        self.rebind_pending = false;
        self.rebind_deadline = None;
    }
}

impl Video {
    pub(crate) fn output_rebind_pending(&self) -> bool {
        self.output
            .as_ref()
            .is_some_and(|output| output.rebind_pending)
    }

    pub(crate) fn output_rebind_due_in(&self, now: Instant) -> Option<Duration> {
        let output = self
            .output
            .as_ref()
            .filter(|output| output.rebind_pending)?;
        output
            .rebind_deadline
            .map(|end| end.saturating_duration_since(now))
    }
}

impl CurtainApp {
    pub(crate) fn configured_video_surface_index(&self) -> Option<usize> {
        if let Some(name) = self.ui_output_name.as_deref() {
            self.lock_surfaces
                .iter()
                .position(|surface| surface.with_output_name(|output| output == Some(name)))
        } else {
            self.selected_ui_surface_index()
        }
    }

    pub(crate) fn video_output_lost(&mut self, lost: &WlOutput) -> bool {
        if !self.video.targets(lost) {
            return false;
        }
        let Some(output) = &mut self.video.output else {
            return false;
        };
        if output.can_rebind(self.video.power.sleeping(), Instant::now()) {
            if !output.rebind_pending {
                tracing::debug!("video prototype waiting for a replacement output");
            }
            output.rebind_pending = true;
            let redraw = self.update_video_hold(VideoHold::Display, true);
            self.video.target = None;
            return redraw;
        }
        self.stop_video_prototype()
    }

    pub(crate) fn rebind_video_output(&mut self, added: &WlOutput, queue: &QueueHandle<Self>) {
        let Some(output) = &self.video.output else {
            return;
        };
        if !output.rebind_pending
            || self.video.target.is_some()
            || !output.can_rebind(self.video.power.sleeping(), Instant::now())
            || self.output_state.info(added).and_then(|info| info.name) != output.name
        {
            return;
        }

        let Some(index) = self
            .lock_surfaces
            .iter()
            .position(|surface| surface.output == *added)
        else {
            return;
        };
        if !self.ui_visible_on_surface(index) {
            self.stop_video_prototype();
            return;
        }
        let Ok(manager) = self.output_power_manager.get() else {
            self.stop_video_prototype();
            return;
        };
        self.lock_surfaces[index]
            .output_power
            .get_or_insert_with(|| manager.get_output_power(added, queue, added.clone()));
        self.video.target = Some(added.clone());
    }

    pub(crate) fn observe_video_output_power(&mut self, index: usize, off: bool) -> bool {
        let surface = &mut self.lock_surfaces[index];
        surface.output_power_off = Some(off);
        if !self.video.targets(&surface.output) {
            return false;
        }
        if let Some(output) = &mut self.video.output
            && output.rebind_pending
        {
            if !output.can_rebind(self.video.power.sleeping(), Instant::now()) {
                return self.stop_video_prototype();
            }
            output.confirm_rebind();
            tracing::debug!(off, "video prototype restored output power observation");
        }
        self.update_video_hold(VideoHold::Display, off)
    }

    pub(crate) fn validate_video_surface(&mut self, index: usize, size: SurfaceSize) {
        if self.video.targets(&self.lock_surfaces[index].output)
            && let Some(output) = &self.video.output
            && !size.same_rendering_as(output.size)
        {
            self.stop_video_prototype();
            tracing::debug!("video prototype stopped after the output rendering size changed");
        }
    }

    pub(crate) fn stop_video_without_prompt(&mut self) -> bool {
        let Some(index) = self.video_surface_index() else {
            return false;
        };
        if self.ui_visible_on_surface(index) {
            return false;
        }
        self.stop_video_prototype();
        self.lock_surfaces[index].background = None;
        true
    }

    pub(crate) fn expire_video_output_rebind(&mut self) {
        if self.video.output_rebind_due_in(Instant::now()) == Some(Duration::ZERO) {
            self.stop_video_prototype();
            tracing::debug!("video prototype timed out restoring output power observation");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_rebind_is_limited_to_sleep_and_the_wake_window() {
        let now = Instant::now();
        let size = SurfaceSize::new_with_fractional_scale(1366, 768, 1, None);
        let mut output = VideoOutput::new(Some("LVDS-1".to_owned()), size);
        assert!(!output.can_rebind(false, now));
        assert!(output.can_rebind(true, now));
        output.sleep_changed(false, now);
        assert!(output.can_rebind(false, now));
        assert!(!output.can_rebind(false, now + REBIND_WINDOW));
        output.confirm_rebind();
        assert!(!output.can_rebind(false, now));
    }
}
