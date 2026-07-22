use std::time::{Duration, Instant};

use smithay_client_toolkit::reexports::client::QueueHandle;

use crate::ipc::auth::AuthEvent;

use super::super::CurtainApp;

const UNLOCK_DELIVERY_GRACE: Duration = Duration::from_secs(10);

impl CurtainApp {
    pub(crate) fn drain_auth_events(&mut self, queue_handle: &QueueHandle<Self>) {
        while let Ok(event) = self.auth_events.try_recv() {
            match event {
                AuthEvent::Accepted { attempt_id } => {
                    tracing::info!(
                        attempt_id,
                        "waiting for daemon-driven unlock after auth success"
                    );
                    self.auth_accepted_at = Some(Instant::now());
                }
                AuthEvent::Rejected {
                    attempt_id,
                    retry_after_ms,
                    failed_attempts,
                } => {
                    self.auth_in_flight = false;
                    tracing::info!(attempt_id, "updating UI after authentication rejection");
                    self.ui_shell
                        .authentication_rejected(retry_after_ms, failed_attempts);
                    self.render_all_surfaces(queue_handle);
                }
                AuthEvent::Busy { attempt_id } => {
                    self.auth_in_flight = false;
                    tracing::debug!(attempt_id, "updating UI after authentication busy response");
                    self.ui_shell.authentication_busy();
                    self.render_all_surfaces(queue_handle);
                }
                AuthEvent::Failed { attempt_id } => {
                    self.auth_in_flight = false;
                    tracing::warn!(
                        attempt_id,
                        "authentication attempt produced no verdict; releasing the input guard"
                    );
                    self.ui_shell.authentication_rejected(None, None);
                    self.render_all_surfaces(queue_handle);
                }
            }
        }
    }

    /// Releases the input guard if an accepted attempt is never followed by the daemon's unlock,
    /// so a failed unlock handoff leaves a retryable prompt rather than a frozen shell.
    pub(crate) fn advance_auth_watchdog(&mut self, queue_handle: &QueueHandle<Self>) {
        let Some(accepted_at) = self.auth_accepted_at else {
            return;
        };
        if accepted_at.elapsed() < UNLOCK_DELIVERY_GRACE {
            return;
        }

        self.auth_accepted_at = None;
        self.auth_in_flight = false;
        tracing::error!(
            "daemon accepted authentication but never delivered an unlock; releasing the input guard"
        );
        self.ui_shell.authentication_rejected(None, None);
        self.render_all_surfaces(queue_handle);
    }
}
