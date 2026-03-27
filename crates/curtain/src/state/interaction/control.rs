use smithay_client_toolkit::reexports::client::QueueHandle;

use super::super::{ControlEvent, CurtainApp};

impl CurtainApp {
    pub(crate) fn drain_control_events(&mut self, queue_handle: &QueueHandle<Self>) {
        while let Ok(event) = self.control_events.try_recv() {
            match event {
                ControlEvent::Unlock { attempt_id } => {
                    if let Some(attempt_id) = attempt_id {
                        tracing::info!(attempt_id, "received curtain unlock request from daemon");
                    } else {
                        tracing::info!("received curtain unlock request from daemon");
                    }
                    self.request_exit();
                }
                ControlEvent::Reload => {
                    tracing::info!("received curtain reload request from daemon");
                    self.reload_config(queue_handle);
                }
                ControlEvent::UpdateNowPlaying { snapshot } => {
                    tracing::debug!(
                        has_snapshot = snapshot.is_some(),
                        "received live now playing update from daemon"
                    );
                    self.now_playing_snapshot = snapshot.clone();
                    self.ui_shell.set_now_playing_snapshot(snapshot);
                    self.render_all_surfaces(queue_handle);
                }
            }
        }
    }
}
