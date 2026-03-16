use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum AuthAdmission {
    Allowed,
    Busy,
    RateLimited(Duration),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthPolicy {
    base_delay: Duration,
    max_delay: Duration,
}

impl Default for AuthPolicy {
    fn default() -> Self {
        Self::new(Duration::from_secs(1), Duration::from_secs(16))
    }
}

impl AuthPolicy {
    pub fn new(base_delay: Duration, max_delay: Duration) -> Self {
        let base_delay = base_delay.max(Duration::from_millis(1));
        let max_delay = max_delay.max(base_delay);

        Self {
            base_delay,
            max_delay,
        }
    }

    fn failure_delay(self, failed_attempts: u8) -> Duration {
        let exponent = u32::from(failed_attempts.saturating_sub(1).min(16));
        let multiplier = 1u32.checked_shl(exponent).unwrap_or(u32::MAX);
        let scaled = self.base_delay.saturating_mul(multiplier);
        scaled.min(self.max_delay)
    }
}

#[derive(Debug, Default)]
pub struct AuthState {
    failed_attempts: u8,
    retry_after: Option<Instant>,
    in_flight: bool,
    policy: AuthPolicy,
}

impl AuthState {
    pub fn new(policy: AuthPolicy) -> Self {
        Self {
            failed_attempts: 0,
            retry_after: None,
            in_flight: false,
            policy,
        }
    }
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
        self.retry_after = Some(now + self.policy.failure_delay(self.failed_attempts));
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{AuthAdmission, AuthPolicy, AuthState};

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

    #[test]
    fn clamps_delay_to_policy_maximum() {
        let mut state = AuthState::new(AuthPolicy::new(
            Duration::from_millis(500),
            Duration::from_secs(2),
        ));
        let now = Instant::now();

        for _ in 0..5 {
            state.start_attempt();
            state.finish_failure(now);
        }

        match state.admit(now) {
            AuthAdmission::RateLimited(delay) => assert!(delay <= Duration::from_secs(2)),
            AuthAdmission::Allowed | AuthAdmission::Busy => panic!("attempt should be throttled"),
        }
    }
}
