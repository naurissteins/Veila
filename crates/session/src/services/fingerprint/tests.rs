use std::{
    sync::atomic::{AtomicU8, Ordering},
    time::Instant,
};

use tokio::sync::mpsc::unbounded_channel;
use veila_common::FingerprintStatus;

use super::FingerprintMatch;
use super::{FingerprintHandle, FingerprintTaskExit, RETRY_DELAY, record_failed_scan};

#[tokio::test]
async fn inactive_lock_resets_fingerprint_attempt_state() {
    let mut handle = FingerprintHandle::new();
    handle.blocked_for_lock = true;
    handle.schedule_retry();

    handle
        .update(
            false,
            true,
            5,
            "alice",
            None::<tokio::sync::mpsc::UnboundedSender<FingerprintMatch>>,
        )
        .await;

    assert!(!handle.blocked_for_lock);
    assert!(handle.next_retry_at.is_none());
    assert_eq!(handle.retry_delay, RETRY_DELAY);
}

#[tokio::test]
async fn sleep_pause_survives_new_lock_and_resume_reenables_attempts() {
    let mut handle = FingerprintHandle::new();
    handle.blocked_for_lock = true;

    handle.pause_for_sleep().await;
    handle.reset_for_new_lock().await;

    assert!(handle.paused_for_sleep);
    assert!(!handle.blocked_for_lock);
    let (auth_sender, _auth_results) = unbounded_channel::<FingerprintMatch>();
    handle
        .update(true, true, 5, "alice", Some(auth_sender))
        .await;
    assert!(handle.task.is_none());

    handle.resume_after_sleep();
    assert!(!handle.paused_for_sleep);
    assert!(handle.next_retry_at.is_none());
}

#[test]
fn sleep_pause_discards_fingerprint_success_only() {
    let mut handle = FingerprintHandle::new();
    handle.paused_for_sleep = true;
    handle.active_attempt_id = Some(super::FINGERPRINT_ATTEMPT_ID_START + 1);
    let stale_fingerprint = FingerprintMatch {
        attempt_id: super::FINGERPRINT_ATTEMPT_ID_START,
        started_at: Instant::now(),
        elapsed_ms: 1,
    };
    let current_fingerprint = FingerprintMatch {
        attempt_id: super::FINGERPRINT_ATTEMPT_ID_START + 1,
        started_at: Instant::now(),
        elapsed_ms: 1,
    };
    let password = FingerprintMatch {
        attempt_id: 7,
        started_at: Instant::now(),
        elapsed_ms: 1,
    };

    assert!(handle.should_discard_attempt(stale_fingerprint.attempt_id));
    assert!(handle.should_discard_attempt(current_fingerprint.attempt_id));
    assert!(!handle.should_discard_attempt(password.attempt_id));

    handle.paused_for_sleep = false;
    assert!(!handle.should_discard_attempt(current_fingerprint.attempt_id));
}

#[tokio::test]
async fn completed_transient_failure_schedules_retry() {
    let mut handle = FingerprintHandle::new();
    handle.task = Some(tokio::spawn(async {
        FingerprintTaskExit::TransientFailure
    }));
    tokio::task::yield_now().await;

    handle.reap_finished_task().await;

    assert!(handle.task.is_none());
    assert!(handle.next_retry_at.is_some());
    assert_eq!(handle.retry_delay, RETRY_DELAY.saturating_mul(2));
}

#[test]
fn transient_retry_delay_is_capped() {
    let mut handle = FingerprintHandle::new();

    for _ in 0..8 {
        handle.schedule_retry();
    }

    assert_eq!(handle.retry_delay, super::MAX_RETRY_DELAY);
}

#[test]
fn failed_scan_counter_saturates() {
    let failures = AtomicU8::new(0);
    for expected in 1..=5 {
        assert_eq!(record_failed_scan(&failures), expected);
    }
    failures.store(u8::MAX, Ordering::Relaxed);
    assert_eq!(record_failed_scan(&failures), u8::MAX);
}

#[tokio::test]
async fn fingerprint_limit_survives_suspend_until_next_lock() {
    let mut handle = FingerprintHandle::new();
    handle.failed_attempts.store(5, Ordering::Relaxed);
    handle.pause_for_sleep().await;
    handle.resume_after_sleep();

    let (auth_sender, _auth_results) = unbounded_channel::<FingerprintMatch>();
    handle
        .update(true, true, 5, "alice", Some(auth_sender))
        .await;

    assert!(handle.blocked_for_lock);
    assert!(handle.task.is_none());
    assert_eq!(
        handle.status_rx.try_recv(),
        Ok(None),
        "sleep clears the previous status"
    );
    assert_eq!(
        handle.status_rx.try_recv(),
        Ok(Some(FingerprintStatus::AttemptLimitReached))
    );

    handle.reset_for_new_lock().await;
    assert_eq!(handle.failed_attempts.load(Ordering::Relaxed), 0);
    assert!(!handle.blocked_for_lock);
}

#[tokio::test]
async fn disabling_fingerprint_keeps_failures_for_the_current_lock() {
    let mut handle = FingerprintHandle::new();
    handle.failed_attempts.store(3, Ordering::Relaxed);

    handle
        .update(
            true,
            false,
            5,
            "alice",
            None::<tokio::sync::mpsc::UnboundedSender<FingerprintMatch>>,
        )
        .await;

    assert_eq!(handle.failed_attempts.load(Ordering::Relaxed), 3);
}

#[tokio::test]
async fn cancelled_and_sleeping_attempts_revoke_already_queued_authorizations() {
    let mut handle = FingerprintHandle::new();
    let attempt_id = super::FINGERPRINT_ATTEMPT_ID_START;
    let matched = FingerprintMatch {
        attempt_id,
        started_at: Instant::now(),
        elapsed_ms: 1,
    };
    handle.active_attempt_id = Some(attempt_id);
    let queued = handle.authorization(matched).unwrap();
    assert!(queued.is_current());
    handle.pause_for_sleep().await;
    assert!(!queued.is_current());
    assert!(handle.authorization(matched).is_none());
    handle.resume_after_sleep();
    handle.active_attempt_id = Some(attempt_id + 1);
    assert!(handle.authorization(matched).is_none());
    let current = handle
        .authorization(FingerprintMatch {
            attempt_id: attempt_id + 1,
            ..matched
        })
        .unwrap();
    handle.cancel_attempt().await;
    assert!(!current.is_current());
}

#[test]
fn password_ids_cannot_issue_fingerprint_authorizations_and_ids_wrap_in_the_upper_half() {
    let handle = FingerprintHandle::new();
    assert!(
        handle
            .authorization(FingerprintMatch {
                attempt_id: 7,
                started_at: Instant::now(),
                elapsed_ms: 1
            })
            .is_none()
    );
    assert_eq!(
        super::next_fingerprint_attempt_id(u64::MAX),
        super::FINGERPRINT_ATTEMPT_ID_START
    );
}

#[tokio::test(start_paused = true)]
async fn stalled_cleanup_is_aborted_and_preserves_failures() {
    let mut handle = FingerprintHandle::new();
    handle.failed_attempts.store(3, Ordering::Relaxed);
    handle.task = Some(tokio::spawn(std::future::pending()));
    handle.pause_for_sleep().await;
    assert!(handle.task.is_none());
    assert_eq!(handle.failed_attempts.load(Ordering::Relaxed), 3);
}

#[test]
fn dropping_the_handle_revokes_queued_authorizations() {
    let mut handle = FingerprintHandle::new();
    handle.active_attempt_id = Some(super::FINGERPRINT_ATTEMPT_ID_START);
    let authorization = handle
        .authorization(FingerprintMatch {
            attempt_id: super::FINGERPRINT_ATTEMPT_ID_START,
            started_at: Instant::now(),
            elapsed_ms: 1,
        })
        .unwrap();
    drop(handle);
    assert!(!authorization.is_current());
}

#[tokio::test]
async fn cancelling_a_generation_discards_its_old_scan_status() {
    let mut handle = FingerprintHandle::new();
    handle
        .status_tx
        .send(Some(FingerprintStatus::Accepted))
        .unwrap();
    handle.cancel_attempt().await;
    assert_eq!(handle.take_status(), Some(None));
    assert_eq!(handle.take_status(), None);
}
