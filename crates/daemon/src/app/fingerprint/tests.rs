use std::{
    sync::atomic::{AtomicU8, Ordering},
    time::Instant,
};

use tokio::sync::mpsc::unbounded_channel;
use veila_common::FingerprintStatus;

use super::{FingerprintHandle, FingerprintTaskExit, RETRY_DELAY, record_failed_scan};
use crate::app::runtime::AuthResult;

#[tokio::test]
async fn inactive_lock_resets_fingerprint_attempt_state() {
    let mut handle = FingerprintHandle::new();
    handle.blocked_for_lock = true;
    handle.schedule_retry();

    handle.update(false, true, 5, "alice", None).await;

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
    let (auth_sender, _auth_results) = unbounded_channel();
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
    let stale_fingerprint = AuthResult::Succeeded {
        attempt_id: super::FINGERPRINT_ATTEMPT_ID_START,
        started_at: Instant::now(),
        elapsed_ms: 1,
    };
    let current_fingerprint = AuthResult::Succeeded {
        attempt_id: super::FINGERPRINT_ATTEMPT_ID_START + 1,
        started_at: Instant::now(),
        elapsed_ms: 1,
    };
    let password = AuthResult::Succeeded {
        attempt_id: 7,
        started_at: Instant::now(),
        elapsed_ms: 1,
    };

    assert!(handle.should_discard_auth_result(&stale_fingerprint));
    assert!(handle.should_discard_auth_result(&current_fingerprint));
    assert!(!handle.should_discard_auth_result(&password));

    handle.paused_for_sleep = false;
    assert!(!handle.should_discard_auth_result(&current_fingerprint));
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

    let (auth_sender, _auth_results) = unbounded_channel();
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

    handle.update(true, false, 5, "alice", None).await;

    assert_eq!(handle.failed_attempts.load(Ordering::Relaxed), 3);
}
