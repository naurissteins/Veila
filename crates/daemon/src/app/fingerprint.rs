use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::Instant,
};

use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
        watch,
    },
    task::JoinHandle,
    time::{Duration, sleep, timeout},
};
use veila_common::{FingerprintStatus, elapsed_ms};

use crate::{
    adapters::{fprint, process},
    app::runtime::AuthResult,
};

const FINGERPRINT_ATTEMPT_ID_START: u64 = 1 << 63;
const RETRY_DELAY: Duration = Duration::from_millis(900);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(8);
const STOP_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FingerprintTaskExit {
    Authenticated,
    AttemptLimitReached,
    NoEnrolledFingers,
    TransientFailure,
    Cancelled,
}

pub(super) struct FingerprintHandle {
    task: Option<JoinHandle<FingerprintTaskExit>>,
    cancel: Option<watch::Sender<bool>>,
    paused_for_sleep: bool,
    blocked_for_lock: bool,
    active_attempt_id: Option<u64>,
    next_attempt_id: u64,
    next_retry_at: Option<Instant>,
    retry_delay: Duration,
    failed_attempts: Arc<AtomicU8>,
    status_rx: UnboundedReceiver<Option<FingerprintStatus>>,
    status_tx: UnboundedSender<Option<FingerprintStatus>>,
}

impl FingerprintHandle {
    pub(super) fn new() -> Self {
        let (status_tx, status_rx) = unbounded_channel();
        Self {
            task: None,
            cancel: None,
            paused_for_sleep: false,
            blocked_for_lock: false,
            active_attempt_id: None,
            next_attempt_id: FINGERPRINT_ATTEMPT_ID_START,
            next_retry_at: None,
            retry_delay: RETRY_DELAY,
            failed_attempts: Arc::new(AtomicU8::new(0)),
            status_rx,
            status_tx,
        }
    }

    pub(super) async fn reset_for_new_lock(&mut self) {
        self.stop_task().await;
        self.reset_attempt_state();
    }

    pub(super) async fn stop(&mut self) {
        self.stop_task().await;
        self.reset_attempt_state();
    }

    pub(super) async fn pause_for_sleep(&mut self) {
        self.paused_for_sleep = true;
        self.stop_task().await;
        self.blocked_for_lock = false;
        self.next_retry_at = None;
        let _ = self.status_tx.send(None);
    }

    pub(super) fn resume_after_sleep(&mut self) {
        self.paused_for_sleep = false;
        self.next_retry_at = None;
        self.retry_delay = RETRY_DELAY;
    }

    pub(super) fn should_discard_auth_result(&self, result: &AuthResult) -> bool {
        matches!(
            result,
            AuthResult::Succeeded { attempt_id, .. }
                if is_fingerprint_attempt(*attempt_id)
                    && (self.paused_for_sleep
                        || self.active_attempt_id != Some(*attempt_id))
        )
    }

    async fn stop_task(&mut self) {
        self.active_attempt_id = None;
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(true);
        }

        let Some(mut task) = self.task.take() else {
            return;
        };
        if timeout(STOP_TIMEOUT, &mut task).await.is_err() {
            tracing::warn!("fingerprint verification did not stop cleanly; aborting task");
            task.abort();
            let _ = task.await;
        }
    }

    pub(super) async fn update(
        &mut self,
        active_lock: bool,
        enabled: bool,
        max_failed_attempts: u8,
        username: &str,
        auth_sender: Option<UnboundedSender<AuthResult>>,
    ) {
        if !active_lock {
            let had_state =
                self.task.is_some() || self.blocked_for_lock || self.next_retry_at.is_some();
            self.stop_task().await;
            if had_state {
                let _ = self.status_tx.send(None);
            }
            self.reset_attempt_state();
            return;
        }

        if !enabled {
            let had_state =
                self.task.is_some() || self.blocked_for_lock || self.next_retry_at.is_some();
            self.stop_task().await;
            self.blocked_for_lock = false;
            self.next_retry_at = None;
            if had_state {
                let _ = self.status_tx.send(None);
            }
            return;
        }

        if self.paused_for_sleep {
            return;
        }

        self.reap_finished_task().await;
        if self.blocked_for_lock || self.task.is_some() {
            return;
        }
        // failed scans remain counted across reader retries and suspend within one lock
        if self.failed_attempts.load(Ordering::Relaxed) >= max_failed_attempts {
            self.blocked_for_lock = true;
            self.next_retry_at = None;
            let _ = self
                .status_tx
                .send(Some(FingerprintStatus::AttemptLimitReached));
            return;
        }
        if self
            .next_retry_at
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return;
        }

        let Some(auth_sender) = auth_sender else {
            return;
        };

        self.next_retry_at = None;
        let username = username.to_owned();
        let status_tx = self.status_tx.clone();
        let failed_attempts = Arc::clone(&self.failed_attempts);
        let attempt_id = self.next_attempt_id;
        self.next_attempt_id = next_fingerprint_attempt_id(attempt_id);
        self.active_attempt_id = Some(attempt_id);
        let (cancel, cancel_rx) = watch::channel(false);
        self.cancel = Some(cancel);
        self.task = Some(tokio::spawn(async move {
            run_fingerprint_loop(
                username,
                status_tx,
                auth_sender,
                cancel_rx,
                attempt_id,
                failed_attempts,
                max_failed_attempts,
            )
            .await
        }));
    }

    async fn reap_finished_task(&mut self) {
        if !self.task.as_ref().is_some_and(JoinHandle::is_finished) {
            return;
        }

        self.cancel = None;
        let Some(task) = self.task.take() else {
            return;
        };
        match task.await {
            Ok(FingerprintTaskExit::Authenticated) => {
                self.blocked_for_lock = true;
                self.next_retry_at = None;
            }
            Ok(FingerprintTaskExit::AttemptLimitReached) => {
                self.active_attempt_id = None;
                self.blocked_for_lock = true;
                self.next_retry_at = None;
            }
            Ok(FingerprintTaskExit::NoEnrolledFingers) => {
                self.active_attempt_id = None;
                self.blocked_for_lock = true;
                self.next_retry_at = None;
            }
            Ok(FingerprintTaskExit::TransientFailure) => self.schedule_retry(),
            Ok(FingerprintTaskExit::Cancelled) => {
                self.active_attempt_id = None;
            }
            Err(error) => {
                tracing::warn!("fingerprint verification task failed: {error}");
                self.schedule_retry();
            }
        }
    }

    fn schedule_retry(&mut self) {
        self.active_attempt_id = None;
        self.next_retry_at = Some(Instant::now() + self.retry_delay);
        self.retry_delay = self.retry_delay.saturating_mul(2).min(MAX_RETRY_DELAY);
    }

    fn reset_attempt_state(&mut self) {
        self.active_attempt_id = None;
        self.blocked_for_lock = false;
        self.next_retry_at = None;
        self.retry_delay = RETRY_DELAY;
        self.failed_attempts.store(0, Ordering::Relaxed);
    }

    pub(super) async fn forward_status_updates(&mut self, control_socket_path: Option<&PathBuf>) {
        let Some(control_socket_path) = control_socket_path else {
            while self.status_rx.try_recv().is_ok() {}
            return;
        };

        while let Ok(status) = self.status_rx.try_recv() {
            if let Err(error) = process::request_curtain_fingerprint_status_update(
                control_socket_path,
                status.as_ref(),
            )
            .await
            {
                tracing::warn!("failed to forward fingerprint status to curtain: {error:#}");
            }
        }
    }
}

async fn run_fingerprint_loop(
    username: String,
    status_tx: UnboundedSender<Option<FingerprintStatus>>,
    auth_sender: UnboundedSender<AuthResult>,
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
                let _ = auth_sender.send(AuthResult::Succeeded {
                    attempt_id,
                    started_at,
                    elapsed_ms,
                });
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

fn record_failed_scan(failed_attempts: &AtomicU8) -> u8 {
    // only the current verification task increments this per-lock counter
    let failures = failed_attempts.load(Ordering::Relaxed).saturating_add(1);
    failed_attempts.store(failures, Ordering::Relaxed);
    failures
}

fn is_fingerprint_attempt(attempt_id: u64) -> bool {
    attempt_id >= FINGERPRINT_ATTEMPT_ID_START
}

fn next_fingerprint_attempt_id(attempt_id: u64) -> u64 {
    if attempt_id == u64::MAX {
        FINGERPRINT_ATTEMPT_ID_START
    } else {
        attempt_id + 1
    }
}

#[cfg(test)]
#[path = "fingerprint/tests.rs"]
mod tests;
