use std::{path::PathBuf, time::Instant};

use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
        watch,
    },
    task::JoinHandle,
    time::{Duration, sleep, timeout},
};
use veila_common::FingerprintStatus;

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
        username: &str,
        auth_sender: Option<UnboundedSender<AuthResult>>,
    ) {
        if !active_lock || !enabled {
            let had_state =
                self.task.is_some() || self.blocked_for_lock || self.next_retry_at.is_some();
            self.stop_task().await;
            if active_lock && had_state {
                let _ = self.status_tx.send(None);
            }
            self.reset_attempt_state();
            return;
        }

        if self.paused_for_sleep {
            return;
        }

        self.reap_finished_task().await;
        if self.blocked_for_lock || self.task.is_some() {
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
        let attempt_id = self.next_attempt_id;
        self.next_attempt_id = next_fingerprint_attempt_id(attempt_id);
        self.active_attempt_id = Some(attempt_id);
        let (cancel, cancel_rx) = watch::channel(false);
        self.cancel = Some(cancel);
        self.task = Some(tokio::spawn(async move {
            run_fingerprint_loop(username, status_tx, auth_sender, cancel_rx, attempt_id).await
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
) -> FingerprintTaskExit {
    let started_at = Instant::now();
    loop {
        match fprint::verify_once(&username, &status_tx, &mut cancel).await {
            Ok(fprint::VerifyOutcome::Matched) => {
                if *cancel.borrow() {
                    return FingerprintTaskExit::Cancelled;
                }
                let elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                let _ = auth_sender.send(AuthResult::Succeeded {
                    attempt_id,
                    started_at,
                    elapsed_ms,
                });
                return FingerprintTaskExit::Authenticated;
            }
            Ok(fprint::VerifyOutcome::NotMatched) => {
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
mod tests {
    use std::time::Instant;

    use tokio::sync::mpsc::unbounded_channel;

    use super::{FingerprintHandle, FingerprintTaskExit, RETRY_DELAY};
    use crate::app::runtime::AuthResult;

    #[tokio::test]
    async fn inactive_lock_resets_fingerprint_attempt_state() {
        let mut handle = FingerprintHandle::new();
        handle.blocked_for_lock = true;
        handle.schedule_retry();

        handle.update(false, true, "alice", None).await;

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
        handle.update(true, true, "alice", Some(auth_sender)).await;
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
}
