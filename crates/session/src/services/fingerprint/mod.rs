use std::{
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
    time::Duration,
};
use veila_common::FingerprintStatus;

pub(crate) mod authorization;
mod task;
use authorization::Authority;
pub use authorization::Authorization;
#[cfg(test)]
use task::record_failed_scan;
use task::{FingerprintTaskExit, run_fingerprint_loop};
mod lifecycle;

#[derive(Debug, Clone, Copy)]
pub struct FingerprintMatch {
    pub attempt_id: u64,
    pub started_at: Instant,
    pub elapsed_ms: u64,
}

const FINGERPRINT_ATTEMPT_ID_START: u64 = 1 << 63;
const RETRY_DELAY: Duration = Duration::from_millis(900);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(8);
const STOP_TIMEOUT: Duration = Duration::from_secs(1);

pub struct FingerprintHandle {
    authority: Authority,
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
    pub fn new() -> Self {
        let (status_tx, status_rx) = unbounded_channel();
        Self {
            authority: Authority::default(),
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

    pub fn should_discard_attempt(&self, attempt_id: u64) -> bool {
        is_fingerprint_attempt(attempt_id)
            && (self.paused_for_sleep || self.active_attempt_id != Some(attempt_id))
    }

    pub fn authorization(&self, matched: FingerprintMatch) -> Option<Authorization> {
        (is_fingerprint_attempt(matched.attempt_id)
            && !self.should_discard_attempt(matched.attempt_id))
        .then(|| self.authority.issue())
    }

    pub fn take_status(&mut self) -> Option<Option<FingerprintStatus>> {
        self.status_rx.try_recv().ok()
    }

    pub async fn update<T: From<FingerprintMatch> + Send + 'static>(
        &mut self,
        active_lock: bool,
        enabled: bool,
        max_failed_attempts: u8,
        username: &str,
        auth_sender: Option<UnboundedSender<T>>,
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
mod tests;
