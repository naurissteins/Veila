use std::time::{Duration, Instant};

use smithay_client_toolkit::reexports::client::QueueHandle;

use crate::ipc::auth::AuthEvent;

use super::super::CurtainApp;

const UNLOCK_DELIVERY_GRACE: Duration = Duration::from_secs(10);

impl CurtainApp {
    pub(crate) fn auth_watchdog_due_in(&self, now: Instant) -> Option<Duration> {
        self.auth_accepted_at
            .map(|accepted_at| (accepted_at + UNLOCK_DELIVERY_GRACE).saturating_duration_since(now))
    }

    pub(crate) fn handle_auth_event(&mut self, event: AuthEvent, queue_handle: &QueueHandle<Self>) {
        let event_attempt_id = match &event {
            AuthEvent::Challenge { attempt_id, .. }
            | AuthEvent::Notice { attempt_id, .. }
            | AuthEvent::Accepted { attempt_id }
            | AuthEvent::Rejected { attempt_id, .. }
            | AuthEvent::Busy { attempt_id }
            | AuthEvent::Failed { attempt_id } => *attempt_id,
        };
        if self.auth_attempt_id != Some(event_attempt_id) {
            return;
        }
        match event {
            AuthEvent::Challenge {
                sequence,
                echo,
                text,
                ..
            } => {
                if self.auth_challenge_sequence.is_some() {
                    return;
                }
                self.auth_challenge_sequence = Some(sequence);
                self.ui_shell.authentication_challenge(text, echo);
                self.render_all_surfaces(queue_handle);
            }
            AuthEvent::Notice { text, .. } => {
                self.ui_shell.authentication_notice(text);
                self.render_all_surfaces(queue_handle);
            }
            AuthEvent::Accepted { attempt_id } => {
                self.auth_reply_sender = None;
                self.auth_challenge_sequence = None;
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
                message,
            } => {
                self.auth_in_flight = false;
                self.clear_auth_exchange();
                tracing::info!(attempt_id, "updating UI after authentication rejection");
                self.ui_shell.authentication_rejected_with_message(
                    retry_after_ms,
                    failed_attempts,
                    message,
                );
                self.render_all_surfaces(queue_handle);
            }
            AuthEvent::Busy { attempt_id } => {
                self.auth_in_flight = false;
                self.clear_auth_exchange();
                tracing::debug!(attempt_id, "updating UI after authentication busy response");
                self.ui_shell.authentication_busy();
                self.render_all_surfaces(queue_handle);
            }
            AuthEvent::Failed { attempt_id } => {
                self.auth_in_flight = false;
                self.clear_auth_exchange();
                tracing::warn!(
                    attempt_id,
                    "authentication attempt produced no verdict; releasing the input guard"
                );
                self.ui_shell.authentication_rejected(None, None);
                self.render_all_surfaces(queue_handle);
            }
        }
    }

    fn clear_auth_exchange(&mut self) {
        self.auth_reply_sender = None;
        self.auth_attempt_id = None;
        self.auth_challenge_sequence = None;
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
        self.clear_auth_exchange();
        tracing::error!(
            "daemon accepted authentication but never delivered an unlock; releasing the input guard"
        );
        self.ui_shell.authentication_rejected(None, None);
        self.render_all_surfaces(queue_handle);
    }
}
