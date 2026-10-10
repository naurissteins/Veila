mod relock;
mod requests;

use std::{
    future::{Future, pending},
    path::{Path, PathBuf},
    pin::Pin,
};
use veila_common::ipc::{DaemonControlResponse, LatencyReportMode, LockLatencyReport};
use zbus::zvariant::OwnedObjectPath;

use super::{
    connections::ControlConnection,
    runtime::{LockActivation, activate_lock},
    state::AppRuntime,
};
use crate::{adapters::logind, domain::lock_state::LockState};
use veila_auth::policy::AuthState;

pub(super) enum Milestone {
    Secured(Box<LockActivation>),
    Failed {
        state: LockState,
        error: anyhow::Error,
    },
    Ready(Option<Box<LockLatencyReport>>),
}

type Operation = Pin<Box<dyn Future<Output = Milestone>>>;

#[derive(Default)]
pub(super) struct PendingStartup {
    operation: Option<Operation>,
    generation: Option<PathBuf>,
    acquiring: bool,
    waiters: Vec<requests::Waiter>,
    pub(super) relock: Option<relock::DeferredRelock>,
    pub(super) reloads: std::collections::VecDeque<ControlConnection>,
    pub(super) unlock_requested: bool,
    pub(super) deferred_sleep: Option<bool>,
}

impl PendingStartup {
    pub(super) fn is_pending(&self) -> bool {
        self.operation.is_some()
    }
    pub(super) fn is_acquiring(&self) -> bool {
        self.acquiring
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn begin(
        &mut self,
        trigger: &'static str,
        runtime: &mut AppRuntime,
        connection: &zbus::Connection,
        session_path: &OwnedObjectPath,
        config_path: Option<&Path>,
        emergency: bool,
        latency: LatencyReportMode,
    ) -> bool {
        if self.unlock_requested {
            self.defer_relock(trigger, emergency, latency);
            return false;
        }
        if runtime.state.is_active() || runtime.active.is_some() || self.is_pending() {
            return false;
        }
        let connection = connection.clone();
        let session_path = session_path.clone();
        let config_path = config_path.map(Path::to_path_buf);
        let background = runtime.select_initial_background_path();
        let weather = runtime.weather.current_snapshot();
        let battery = runtime.battery.current_snapshot();
        let now_playing = runtime.now_playing.current_snapshot();
        let timeout = runtime.loaded_config.config.lock.acquire_timeout_seconds;
        let config_ms = runtime.daemon_config_load_ms;
        let config_us = runtime.daemon_config_load_us;
        // Mark pending authority before yielding so every trigger shares one owner.
        runtime.state = LockState::Locking;
        runtime.active_latency_report = latency;
        self.acquiring = true;
        self.operation = Some(Box::pin(async move {
            let proxy = match logind::session_proxy(&connection, &session_path).await {
                Ok(proxy) => proxy,
                Err(error) => {
                    return Milestone::Failed {
                        state: LockState::Unlocked,
                        error,
                    };
                }
            };
            let mut state = LockState::Locking;
            match activate_lock(
                trigger,
                &proxy,
                &mut state,
                config_path.as_deref(),
                background.as_deref(),
                weather.as_ref(),
                battery.as_ref(),
                now_playing.as_ref(),
                emergency,
                latency,
                timeout,
                config_ms,
                config_us,
            )
            .await
            {
                Ok(activation) => Milestone::Secured(Box::new(activation)),
                Err(error) => Milestone::Failed { state, error },
            }
        }));
        true
    }

    pub(super) async fn next(&mut self) -> Milestone {
        match self.operation.as_mut() {
            Some(operation) => operation.as_mut().await,
            None => pending().await,
        }
    }

    pub(super) async fn complete(&mut self, milestone: Milestone, runtime: &mut AppRuntime) {
        self.operation = None;
        self.acquiring = false;
        match milestone {
            Milestone::Secured(activation) => {
                let LockActivation { active, readiness } = *activation;
                self.generation = Some(active.auth_socket_path.clone());
                runtime.active = Some(active);
                runtime.state = LockState::Locked;
                runtime.auth_state = AuthState::new(runtime.auth_policy);
                runtime.suspend_state.arm(std::time::Instant::now());
                runtime.last_power_status_snapshot = None;
                runtime.power_status_sent = false;
                runtime.fingerprint.reset_for_new_lock().await;
                self.operation = Some(Box::pin(async move {
                    Milestone::Ready(readiness.wait().await.map(Box::new))
                }));
            }
            Milestone::Ready(report) => {
                self.generation = None;
                self.finish_waiters(Ok(report.map(|report| *report)));
            }
            Milestone::Failed { state, error } => {
                runtime.state = state;
                tracing::error!("failed to activate lock: {error:#}");
                self.finish_waiters(Err(format!("failed to activate lock: {error:#}")));
            }
        }
    }

    pub(super) fn reconcile(&mut self, runtime: &AppRuntime) {
        if let Some(generation) = &self.generation
            && runtime
                .active
                .as_ref()
                .map(|active| &active.auth_socket_path)
                != Some(generation)
        {
            self.operation = None;
            self.generation = None;
            self.finish_waiters(Err("curtain released or exited before readiness".into()));
        }
    }

    fn finish_waiters(&mut self, result: Result<Option<LockLatencyReport>, String>) {
        for waiter in self.waiters.drain(..) {
            let response = match &result {
                Ok(report) => DaemonControlResponse::Locked {
                    already_active: waiter.already_active,
                    latency_report: if waiter.already_active {
                        None
                    } else {
                        report.clone().map(Box::new)
                    },
                },
                Err(reason) => DaemonControlResponse::Error {
                    reason: reason.clone(),
                },
            };
            requests::respond(waiter.stream, response);
        }
    }
}

#[cfg(test)]
mod tests;
