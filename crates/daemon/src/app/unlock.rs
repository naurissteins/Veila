use std::{
    future::{Future, pending},
    path::PathBuf,
    pin::Pin,
    time::Instant,
};

use anyhow::Result;
use tokio::time::{Duration, sleep_until};
use zbus::zvariant::OwnedObjectPath;

use super::{
    runtime::{
        AuthResult, reset_runtime,
        unlock::{GRACEFUL_EXIT_WINDOW, deliver_unlock},
        update_locked_hint,
    },
    state::AppRuntime,
};
use crate::{
    adapters::{logind, ownership, process::CurtainHandle},
    domain::lock_state::LockState,
};

pub(super) enum Progress {
    Delivered(Result<()>),
    Deadline,
    HintCleared,
}

type Operation = Pin<Box<dyn Future<Output = Progress>>>;

#[derive(Default)]
pub(super) struct PendingUnlock {
    operation: Option<Operation>,
    generation: Option<PathBuf>,
    waiting_exit: bool,
    terminating: bool,
    started_at: Option<Instant>,
    auth_result: Option<AuthResult>,
}

impl PendingUnlock {
    pub(super) fn is_pending(&self) -> bool {
        self.operation.is_some()
    }

    pub(super) fn can_wait_for_exit(&self) -> bool {
        !self.is_pending() || self.waiting_exit
    }

    pub(super) fn begin(
        &mut self,
        runtime: &mut AppRuntime,
        auth_result: Option<AuthResult>,
    ) -> bool {
        if self.is_pending() {
            return true;
        }
        let Some(active) = &runtime.active else {
            if runtime.state.is_active() {
                tracing::error!(
                    "curtain ownership is unresolved; refusing to clear the lock state"
                );
            }
            return false;
        };
        let path = active.control_socket_path.clone();
        let attempt_id = auth_result.and_then(|result| match result {
            AuthResult::Succeeded { attempt_id, .. } => Some(attempt_id),
            AuthResult::Rejected { .. } => None,
        });
        self.generation = Some(active.auth_socket_path.clone());
        self.started_at = Some(Instant::now());
        self.auth_result = auth_result;
        runtime.state = LockState::Unlocking;
        // Keep resources installed; only delivery and timers live in the pinned operation.
        self.operation = Some(Box::pin(async move {
            Progress::Delivered(deliver_unlock(&path, attempt_id).await)
        }));
        true
    }

    pub(super) async fn next(&mut self) -> Progress {
        match self.operation.as_mut() {
            Some(operation) => operation.as_mut().await,
            None => pending().await,
        }
    }

    fn matches(&self, runtime: &AppRuntime) -> bool {
        self.generation.as_ref()
            == runtime
                .active
                .as_ref()
                .map(|active| &active.auth_socket_path)
            && self.generation.is_some()
    }

    fn wait_for_exit(&mut self, duration: Duration) {
        self.waiting_exit = true;
        let deadline = tokio::time::Instant::now() + duration;
        self.operation = Some(Box::pin(async move {
            sleep_until(deadline).await;
            Progress::Deadline
        }));
    }

    pub(super) fn fail(&mut self, runtime: &mut AppRuntime) {
        if self.matches(runtime) {
            runtime.state = LockState::Locked;
        }
        tracing::error!("release was not confirmed; preserving curtain ownership");
        *self = Self::default();
    }

    pub(super) async fn advance(&mut self, progress: Progress, runtime: &mut AppRuntime) -> bool {
        match progress {
            Progress::Delivered(Ok(())) if self.matches(runtime) => {
                self.wait_for_exit(GRACEFUL_EXIT_WINDOW)
            }
            Progress::Delivered(result) => {
                if let Err(error) = result {
                    tracing::error!("could not deliver unlock to the curtain: {error:#}");
                }
                self.fail(runtime);
                return true;
            }
            Progress::Deadline => {
                if !self.matches(runtime) || self.terminating {
                    self.fail(runtime);
                    return true;
                }
                let Some(active) = runtime.active.as_ref() else {
                    return true;
                };
                if matches!(active.curtain, CurtainHandle::Adopted { .. }) {
                    self.fail(runtime);
                    return true;
                }
                tracing::warn!(
                    "curtain did not exit after unlock delivery; requesting termination"
                );
                if let Err(error) = active
                    .curtain
                    .signal_if_running(nix::sys::signal::Signal::SIGTERM)
                    .await
                {
                    tracing::error!("failed to signal curtain after unlock delivery: {error:#}");
                    self.fail(runtime);
                    return true;
                }
                self.terminating = true;
                self.wait_for_exit(Duration::from_secs(2));
            }
            Progress::HintCleared => {
                runtime.state = LockState::Unlocked;
                if let Some(started_at) = self.started_at {
                    tracing::info!(
                        elapsed_ms = veila_common::time::elapsed_ms(started_at),
                        "curtain stopped; session considered unlocked"
                    );
                    if let Some(AuthResult::Succeeded {
                        attempt_id,
                        started_at: auth_started,
                        elapsed_ms,
                    }) = self.auth_result
                    {
                        tracing::info!(
                            attempt_id,
                            auth_elapsed_ms = elapsed_ms,
                            unlock_elapsed_ms = veila_common::time::elapsed_ms(started_at),
                            daemon_total_ms = veila_common::time::elapsed_ms(auth_started),
                            "unlock timing summary"
                        );
                    }
                }
                *self = Self::default();
                return true;
            }
        }
        false
    }

    fn confirm_release(&mut self, runtime: &mut AppRuntime) -> bool {
        if !self.waiting_exit || !self.matches(runtime) {
            self.fail(runtime);
            return false;
        }
        if let Some(active) = &runtime.active
            && let Err(error) = ownership::remove(active.curtain.owner_path())
        {
            tracing::warn!("failed to clear released curtain ownership: {error:#}");
        }
        reset_runtime(
            &mut runtime.active,
            runtime.auth_policy,
            &mut runtime.auth_state,
        );
        runtime.suspend_state.clear();
        runtime.last_power_status_snapshot = None;
        runtime.power_status_sent = false;
        self.waiting_exit = false;
        true
    }

    pub(super) fn released(
        &mut self,
        runtime: &mut AppRuntime,
        connection: &zbus::Connection,
        session_path: &OwnedObjectPath,
    ) {
        if !self.confirm_release(runtime) {
            return;
        }
        let connection = connection.clone();
        let session_path = session_path.clone();
        // Keep Unlocking until the ordered hint update finishes, before any queued relock.
        self.operation = Some(Box::pin(async move {
            match logind::session_proxy(&connection, &session_path).await {
                Ok(proxy) => update_locked_hint(&proxy, false).await,
                Err(error) => tracing::warn!("failed to create unlock hint proxy: {error:#}"),
            }
            Progress::HintCleared
        }));
    }
}

#[cfg(test)]
mod tests;
