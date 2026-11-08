mod worker;

use std::time::{Duration, Instant};

use veila_auth::policy::{AuthAdmission, AuthPolicy, AuthState};
use veila_common::{config::LockConfig, duration_ms};

use crate::{CurtainOptions, ipc::auth::AuthEvent};

pub(crate) use worker::submit_password;

pub(crate) struct Backend(AuthState);

impl Backend {
    pub(crate) fn new(options: &CurtainOptions, config: &LockConfig) -> Self {
        let policy = AuthPolicy::new(
            Duration::from_millis(config.auth_backoff_base_ms),
            Duration::from_secs(config.auth_backoff_max_seconds),
        );
        Self(AuthState::with_initial_backoff(
            policy,
            Instant::now(),
            Duration::from_millis(options.initial_backoff_ms),
        ))
    }

    pub(crate) fn update_policy(&mut self, config: &LockConfig) {
        self.0.update_policy(AuthPolicy::new(
            Duration::from_millis(config.auth_backoff_base_ms),
            Duration::from_secs(config.auth_backoff_max_seconds),
        ));
    }
    pub(crate) fn admit(&self, now: Instant) -> AuthAdmission {
        self.0.admit(now)
    }
    pub(crate) fn failed_attempts(&self) -> Option<u8> {
        Some(self.0.failed_attempts())
    }
    pub(crate) fn start_attempt(&mut self) {
        self.0.start_attempt();
    }

    pub(crate) fn complete(&mut self, event: &mut AuthEvent, now: Instant) {
        let state = &mut self.0;
        match event {
            AuthEvent::Accepted { .. } => state.finish_success(),
            AuthEvent::Rejected { attempt_id, .. } | AuthEvent::Failed { attempt_id } => {
                let attempt_id = *attempt_id;
                let message = match event {
                    AuthEvent::Rejected { message, .. } => message.take(),
                    _ => None,
                };
                state.finish_failure(now);
                let delay = match state.admit(now) {
                    AuthAdmission::RateLimited(delay) => Some(duration_ms(delay)),
                    _ => None,
                };
                *event = AuthEvent::Rejected {
                    attempt_id,
                    retry_after_ms: delay,
                    failed_attempts: Some(state.failed_attempts()),
                    message,
                };
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
