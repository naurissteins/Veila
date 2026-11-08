use super::{FingerprintMatch, RETRY_DELAY};
use crate::adapters::fprint;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::Instant,
};
use tokio::{
    sync::{mpsc::UnboundedSender, watch},
    time::sleep,
};
use veila_common::{FingerprintStatus, elapsed_ms};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FingerprintTaskExit {
    Authenticated,
    AttemptLimitReached,
    NoEnrolledFingers,
    TransientFailure,
    Cancelled,
}

pub(super) async fn run_fingerprint_loop<T: From<FingerprintMatch> + Send>(
    username: String,
    status_tx: UnboundedSender<Option<FingerprintStatus>>,
    auth_sender: UnboundedSender<T>,
    mut cancel: watch::Receiver<bool>,
    attempt_id: u64,
    failed_attempts: Arc<AtomicU8>,
    max_failed_attempts: u8,
) -> FingerprintTaskExit {
    let started_at = Instant::now();
    loop {
        match fprint::verify_once(&username, &status_tx, &mut cancel).await {
            Ok(fprint::VerifyOutcome::Matched) => {
                if *cancel.borrow() {
                    return FingerprintTaskExit::Cancelled;
                }
                let elapsed_ms = elapsed_ms(started_at);
                let _ = auth_sender.send(
                    FingerprintMatch {
                        attempt_id,
                        started_at,
                        elapsed_ms,
                    }
                    .into(),
                );
                return FingerprintTaskExit::Authenticated;
            }
            Ok(fprint::VerifyOutcome::NotMatched) => {
                let failures = record_failed_scan(&failed_attempts);
                if failures >= max_failed_attempts {
                    let _ = status_tx.send(Some(FingerprintStatus::AttemptLimitReached));
                    return FingerprintTaskExit::AttemptLimitReached;
                }
                tokio::select! {
                    biased;
                    _ = cancel.changed() => return FingerprintTaskExit::Cancelled,
                    _ = sleep(RETRY_DELAY) => {}
                }
            }
            Ok(fprint::VerifyOutcome::NoEnrolledFingers) => {
                return FingerprintTaskExit::NoEnrolledFingers;
            }
            Ok(fprint::VerifyOutcome::Unavailable) => {
                return FingerprintTaskExit::TransientFailure;
            }
            Ok(fprint::VerifyOutcome::Cancelled) => return FingerprintTaskExit::Cancelled,
            Err(error) => {
                tracing::warn!("native fingerprint verification failed: {error:#}");
                let _ = status_tx.send(Some(FingerprintStatus::Error));
                return FingerprintTaskExit::TransientFailure;
            }
        }
    }
}

pub(super) fn record_failed_scan(failed_attempts: &AtomicU8) -> u8 {
    // only the current verification task increments this per-lock counter
    let failures = failed_attempts.load(Ordering::Relaxed).saturating_add(1);
    failed_attempts.store(failures, Ordering::Relaxed);
    failures
}
