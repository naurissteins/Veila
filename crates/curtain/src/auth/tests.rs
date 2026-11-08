use super::Backend;
use crate::{CurtainOptions, ipc::auth::AuthEvent};
use std::time::{Duration, Instant};
use veila_auth::policy::{AuthAdmission, AuthPolicy, AuthState};

fn local() -> Backend {
    Backend(AuthState::new(AuthPolicy::new(
        Duration::from_millis(100),
        Duration::from_millis(400),
    )))
}

#[test]
fn local_attempts_are_serial_and_failures_back_off_to_ceiling() {
    let mut backend = local();
    let now = Instant::now();
    for (index, delay) in [100, 200, 400, 400].into_iter().enumerate() {
        backend.start_attempt();
        assert!(matches!(backend.admit(now), AuthAdmission::Busy));
        let mut event = AuthEvent::Rejected {
            attempt_id: 7,
            retry_after_ms: None,
            failed_attempts: None,
            message: Some("Rejected".into()),
        };
        backend.complete(&mut event, now);
        assert_eq!(
            event,
            AuthEvent::Rejected {
                attempt_id: 7,
                retry_after_ms: Some(delay),
                failed_attempts: Some(index as u8 + 1),
                message: Some("Rejected".into()),
            }
        );
        assert!(matches!(
            backend.admit(now + Duration::from_millis(delay)),
            AuthAdmission::Allowed
        ));
    }
}

#[test]
fn helper_errors_count_as_failures_and_success_clears_backoff() {
    let mut backend = local();
    let now = Instant::now();
    backend.start_attempt();
    let mut event = AuthEvent::Failed { attempt_id: 9 };
    backend.complete(&mut event, now);
    assert!(matches!(
        event,
        AuthEvent::Rejected {
            retry_after_ms: Some(100),
            failed_attempts: Some(1),
            ..
        }
    ));
    backend.complete(&mut AuthEvent::Accepted { attempt_id: 10 }, now);
    assert!(matches!(backend.admit(now), AuthAdmission::Allowed));
    assert_eq!(backend.failed_attempts(), Some(0));
}

#[test]
fn challenge_and_notice_leave_attempt_busy() {
    let mut backend = local();
    let now = Instant::now();
    backend.start_attempt();
    backend.complete(
        &mut AuthEvent::Notice {
            attempt_id: 1,
            text: "Notice".into(),
        },
        now,
    );
    backend.complete(
        &mut AuthEvent::Challenge {
            attempt_id: 1,
            sequence: 1,
            echo: false,
            text: "Token".into(),
        },
        now,
    );
    assert!(matches!(backend.admit(now), AuthAdmission::Busy));
    assert_eq!(backend.failed_attempts(), Some(0));
}

#[test]
fn direct_mode_retains_initial_delay_without_requiring_a_daemon() {
    let options = CurtainOptions {
        lock: true,
        initial_backoff_ms: 3000,
        ..Default::default()
    };
    let backend = Backend::new(&options, &Default::default());
    assert!(
        matches!(backend.admit(Instant::now()), AuthAdmission::RateLimited(delay) if delay > Duration::from_secs(2))
    );
}
