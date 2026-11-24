use smithay_client_toolkit::reexports::client::QueueHandle;
use veila_ui::WidgetKind;

use super::super::{ControlEvent, CurtainApp};
use crate::background::video::VideoHold;

impl CurtainApp {
    pub(crate) fn prepare_for_sleep(&mut self, queue_handle: &QueueHandle<Self>) {
        let redraw_video = self.update_video_hold(VideoHold::Sleep, true);
        self.handle_control_event(ControlEvent::ArmResumeInputGuard, queue_handle);
        if redraw_video {
            self.render_all_surfaces(queue_handle);
        }
    }

    pub(crate) fn drain_control_events(&mut self, queue_handle: &QueueHandle<Self>) {
        while let Ok(event) = self.control_events.try_recv() {
            self.handle_control_event(event, queue_handle);
        }
    }

    pub(crate) fn handle_control_event(
        &mut self,
        event: ControlEvent,
        queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            ControlEvent::Supervisor(message) => self.supervisor_event(*message, queue_handle),
            ControlEvent::Unlock { attempt_id } => {
                if let Some(attempt_id) = attempt_id {
                    tracing::info!(
                        attempt_id,
                        "received curtain unlock request from supervisor"
                    );
                } else {
                    tracing::info!("received curtain unlock request from supervisor");
                }
                self.authorize_unlock();
                self.request_exit();
            }
            ControlEvent::ArmResumeInputGuard => {
                self.resume_input.arm();
                self.clear_auth_input_for_sleep(queue_handle);
            }
            ControlEvent::MarkResumed => {
                self.resume_input.mark_resumed();
                // input can arrive between arming the guard and entering sleep
                self.clear_auth_input_for_sleep(queue_handle);
                if self.update_video_hold(VideoHold::Sleep, false) {
                    self.render_all_surfaces(queue_handle);
                }
            }
            ControlEvent::UpdateNowPlaying { snapshot } => {
                if self.now_playing_snapshot == snapshot {
                    return;
                }
                tracing::debug!(
                    has_snapshot = snapshot.is_some(),
                    startup_elapsed_us = crate::state::elapsed_us(self.startup_started_at),
                    "received live now playing update from supervisor"
                );
                self.now_playing_snapshot = snapshot.clone();
                self.ui_shell.set_now_playing_snapshot(snapshot);
                self.artwork_last_attempt = None;
                self.maybe_start_artwork_load();
                self.render_widget_surfaces(WidgetKind::Media, queue_handle);
            }
            ControlEvent::UpdatePowerStatus { snapshot } => {
                self.remote_power_status = snapshot;
                if self.refresh_power_status_text() {
                    self.render_widget_surfaces(WidgetKind::Indicators, queue_handle);
                }
            }
            ControlEvent::UpdateFingerprintStatus { status } => {
                if self.ui_shell.set_fingerprint_status(status) {
                    self.render_all_surfaces(queue_handle);
                }
            }
        }
    }
}
