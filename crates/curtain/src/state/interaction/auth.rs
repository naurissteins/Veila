use smithay_client_toolkit::reexports::client::QueueHandle;

use crate::ipc::auth::AuthEvent;

use super::super::CurtainApp;

impl CurtainApp {
    pub(crate) fn drain_auth_events(&mut self, queue_handle: &QueueHandle<Self>) {
        while let Ok(event) = self.auth_events.try_recv() {
            match event {
                AuthEvent::Accepted { attempt_id } => {
                    tracing::info!(
                        attempt_id,
                        "waiting for daemon-driven unlock after auth success"
                    );
                }
                AuthEvent::Rejected {
                    attempt_id,
                    retry_after_ms,
                } => {
                    self.auth_in_flight = false;
                    tracing::info!(attempt_id, "updating UI after authentication rejection");
                    self.ui_shell.authentication_rejected(retry_after_ms);
                    self.render_all_surfaces(queue_handle);
                }
                AuthEvent::Busy { attempt_id } => {
                    self.auth_in_flight = false;
                    tracing::debug!(attempt_id, "updating UI after authentication busy response");
                    self.ui_shell.authentication_busy();
                    self.render_all_surfaces(queue_handle);
                }
            }
        }
    }
}
