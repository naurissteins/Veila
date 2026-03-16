use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum AuthAdmission {
    Allowed,
    Busy,
    RateLimited(Duration),
}

#[derive(Debug, Default)]
pub struct AuthState {
    failed_attempts: u8,
    retry_after: Option<Instant>,
    in_flight: bool,
}

impl AuthState {
    pub fn admit(&self, now: Instant) -> AuthAdmission {
        if self.in_flight {
            return AuthAdmission::Busy;
        }

        if let Some(retry_after) = self.retry_after
            && retry_after > now
        {
            return AuthAdmission::RateLimited(retry_after.saturating_duration_since(now));
        }

        AuthAdmission::Allowed
    }

    pub fn start_attempt(&mut self) {
        self.in_flight = true;
    }

    pub fn finish_success(&mut self) {
        self.in_flight = false;
        self.failed_attempts = 0;
        self.retry_after = None;
    }

    pub fn finish_failure(&mut self, now: Instant) {
        self.in_flight = false;
        self.failed_attempts = self.failed_attempts.saturating_add(1);

        let exponent = u32::from(self.failed_attempts.saturating_sub(1).min(4));
        let seconds = 1u64 << exponent;
        self.retry_after = Some(now + Duration::from_secs(seconds));
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{AuthAdmission, AuthState};

    #[test]
    fn rejects_parallel_attempts() {
        let mut state = AuthState::default();
        state.start_attempt();

        assert!(matches!(state.admit(Instant::now()), AuthAdmission::Busy));
    }

    #[test]
    fn backs_off_after_failures() {
        let mut state = AuthState::default();
        let now = Instant::now();

        state.start_attempt();
        state.finish_failure(now);

        match state.admit(now + Duration::from_millis(500)) {
            AuthAdmission::RateLimited(delay) => assert!(delay <= Duration::from_secs(1)),
            AuthAdmission::Allowed | AuthAdmission::Busy => panic!("attempt should be throttled"),
        }
    }

    #[test]
    fn resets_after_success() {
        let mut state = AuthState::default();
        let now = Instant::now();

        state.start_attempt();
        state.finish_failure(now);
        state.start_attempt();
        state.finish_success();

        assert!(matches!(state.admit(now), AuthAdmission::Allowed));
    }
}
