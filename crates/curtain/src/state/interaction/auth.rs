use std::time::Instant;

use smithay_client_toolkit::reexports::client::QueueHandle;

use crate::ipc::auth::AuthEvent;
use veila_auth::policy::AuthAdmission;
use veila_common::Secret;

use super::super::CurtainApp;

impl CurtainApp {
    pub(crate) fn submit_auth_attempt(&mut self, secret: Secret, queue: &QueueHandle<Self>) {
        match self.authentication.admit(Instant::now()) {
            AuthAdmission::RateLimited(delay) => {
                self.ui_shell.authentication_rejected(
                    Some(veila_common::duration_ms(delay)),
                    self.authentication.failed_attempts(),
                );
                return;
            }
            AuthAdmission::Busy => {
                self.ui_shell.authentication_busy();
                return;
            }
            AuthAdmission::Allowed => {}
        }
        let attempt_id = self.next_auth_attempt_id;
        self.next_auth_attempt_id = self.next_auth_attempt_id.saturating_add(1);
        tracing::info!(attempt_id, "submitting password attempt");
        self.auth_in_flight = true;
        self.notify_supervisor(veila_common::session::CurtainMessage::AuthInFlight(true));
        self.auth_attempt_id = Some(attempt_id);
        self.authentication.start_attempt();
        match crate::auth::submit_password(attempt_id, secret, self.auth_sender.clone()) {
            Ok(sender) => self.auth_reply_sender = Some(sender),
            Err(error) => {
                tracing::warn!(attempt_id, %error, "failed to start curtain PAM worker");
                self.handle_auth_event(AuthEvent::Failed { attempt_id }, queue);
            }
        }
    }

    pub(crate) fn handle_auth_event(
        &mut self,
        mut event: AuthEvent,
        queue_handle: &QueueHandle<Self>,
    ) {
        let event_attempt_id = match &event {
            AuthEvent::Challenge { attempt_id, .. }
            | AuthEvent::Notice { attempt_id, .. }
            | AuthEvent::Accepted { attempt_id }
            | AuthEvent::Rejected { attempt_id, .. }
            | AuthEvent::Failed { attempt_id } => *attempt_id,
        };
        if self.auth_attempt_id != Some(event_attempt_id) {
            return;
        }
        self.authentication.complete(&mut event, Instant::now());
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
                tracing::debug!(event_attempt_id, sequence, echo, "presenting PAM challenge");
                self.auth_challenge_sequence = Some(sequence);
                self.ui_shell.authentication_challenge(text, echo);
                self.render_all_surfaces(queue_handle);
            }
            AuthEvent::Notice { text, .. } => {
                self.ui_shell.authentication_notice(text);
                self.render_all_surfaces(queue_handle);
            }
            AuthEvent::Accepted { attempt_id } => {
                self.notify_supervisor(veila_common::session::CurtainMessage::AuthInFlight(false));
                self.auth_reply_sender = None;
                self.auth_challenge_sequence = None;
                tracing::info!(attempt_id, "curtain PAM accepted the current attempt");
                self.authorize_unlock();
                self.request_exit();
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
        self.notify_supervisor(veila_common::session::CurtainMessage::AuthInFlight(false));
        self.auth_reply_sender = None;
        self.auth_attempt_id = None;
        self.auth_challenge_sequence = None;
    }
}
