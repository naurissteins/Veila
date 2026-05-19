use std::{path::PathBuf, time::Instant};

use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    task::JoinHandle,
    time::{Duration, sleep},
};
use veila_common::FingerprintStatus;

use crate::{
    adapters::{fprint, process},
    app::runtime::AuthResult,
};

const FINGERPRINT_ATTEMPT_ID: u64 = u64::MAX;
const RETRY_DELAY: Duration = Duration::from_millis(900);

pub(super) struct FingerprintHandle {
    task: Option<JoinHandle<()>>,
    started_for_lock: bool,
    status_rx: UnboundedReceiver<Option<FingerprintStatus>>,
    status_tx: UnboundedSender<Option<FingerprintStatus>>,
}

impl FingerprintHandle {
    pub(super) fn new() -> Self {
        let (status_tx, status_rx) = unbounded_channel();
        Self {
            task: None,
            started_for_lock: false,
            status_rx,
            status_tx,
        }
    }

    pub(super) fn reset_for_new_lock(&mut self) {
        self.stop();
        self.started_for_lock = false;
    }

    pub(super) fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }

    pub(super) fn update(
        &mut self,
        active_lock: bool,
        enabled: bool,
        username: &str,
        auth_sender: Option<UnboundedSender<AuthResult>>,
    ) {
        if !active_lock || !enabled {
            let had_state = self.task.is_some() || self.started_for_lock;
            self.stop();
            if active_lock && had_state {
                let _ = self.status_tx.send(None);
                self.started_for_lock = false;
            }
            if !active_lock {
                self.started_for_lock = false;
            }
            return;
        }

        if self.started_for_lock {
            return;
        }

        let Some(auth_sender) = auth_sender else {
            return;
        };

        self.started_for_lock = true;
        let username = username.to_owned();
        let status_tx = self.status_tx.clone();
        self.task = Some(tokio::spawn(async move {
            run_fingerprint_loop(username, status_tx, auth_sender).await;
        }));
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
) {
    let started_at = Instant::now();
    loop {
        match fprint::verify_once(&username, &status_tx).await {
            Ok(fprint::VerifyOutcome::Matched) => {
                let elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                let _ = auth_sender.send(AuthResult::Succeeded {
                    attempt_id: FINGERPRINT_ATTEMPT_ID,
                    started_at,
                    elapsed_ms,
                });
                return;
            }
            Ok(fprint::VerifyOutcome::NotMatched) => {
                sleep(RETRY_DELAY).await;
            }
            Ok(fprint::VerifyOutcome::Unavailable) => return,
            Err(error) => {
                tracing::warn!("native fingerprint verification failed: {error:#}");
                let _ = status_tx.send(Some(FingerprintStatus::Error));
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FingerprintHandle;

    #[test]
    fn inactive_lock_resets_fingerprint_start_flag() {
        let mut handle = FingerprintHandle::new();
        handle.started_for_lock = true;

        handle.update(false, true, "alice", None);

        assert!(!handle.started_for_lock);
    }
}
