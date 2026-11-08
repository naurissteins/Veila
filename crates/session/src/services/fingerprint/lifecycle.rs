use super::{FingerprintHandle, RETRY_DELAY, STOP_TIMEOUT};
use tokio::time::timeout;

impl FingerprintHandle {
    pub async fn reset_for_new_lock(&mut self) {
        self.stop_task().await;
        self.reset_attempt_state();
    }

    pub async fn stop(&mut self) {
        self.stop_task().await;
        self.reset_attempt_state();
    }

    pub async fn pause_for_sleep(&mut self) {
        self.paused_for_sleep = true;
        self.stop_task().await;
        self.blocked_for_lock = false;
        self.next_retry_at = None;
        let _ = self.status_tx.send(None);
    }

    pub fn resume_after_sleep(&mut self) {
        self.paused_for_sleep = false;
        self.next_retry_at = None;
        self.retry_delay = RETRY_DELAY;
    }

    pub async fn cancel_attempt(&mut self) {
        self.stop_task().await;
        self.blocked_for_lock = false;
        self.next_retry_at = None;
        while self.status_rx.try_recv().is_ok() {}
        let _ = self.status_tx.send(None);
    }

    pub(super) async fn stop_task(&mut self) {
        self.authority.revoke();
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
}

impl Default for FingerprintHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for FingerprintHandle {
    fn drop(&mut self) {
        self.authority.revoke();
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(true);
        }
        // Dropping the per-attempt connection is the fallback for stalled fprintd cleanup.
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
