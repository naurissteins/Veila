use std::{path::Path, time::Instant};

use anyhow::anyhow;
use tokio::{
    sync::mpsc::unbounded_channel,
    time::{Duration, Instant as TokioInstant, sleep_until},
};

use crate::{
    adapters::{
        ipc, logind,
        ownership::{self, OwnerRecord},
        process::{self, CurtainHandle},
    },
    domain::lock_state::LockState,
};
use veila_common::{
    BatterySnapshot, NowPlayingSnapshot, WeatherSnapshot, elapsed_ms, elapsed_us,
    ipc::{CurtainStartupMessage, LatencyReportMode, LockLatencyReport},
};

use super::super::{
    active::ActiveLock,
    state::{LockActivation, update_locked_hint},
};
use super::startup::{
    log_latency_report, read_startup_message, remove_activation_sockets, startup_timeout,
};

pub(super) struct AttemptFailure {
    pub(super) error: anyhow::Error,
    pub(super) session_locked: bool,
    pub(super) retry_safe: bool,
}

impl AttemptFailure {
    fn retryable(error: anyhow::Error, session_locked: bool) -> Self {
        Self {
            error,
            session_locked,
            retry_safe: true,
        }
    }

    fn unsafe_to_retry(error: anyhow::Error, session_locked: bool) -> Self {
        Self {
            error,
            session_locked,
            retry_safe: false,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn activate_lock_attempt(
    trigger: &'static str,
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    config_path: Option<&Path>,
    initial_background_path: Option<&Path>,
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
    force_emergency_ui: bool,
    latency_report: LatencyReportMode,
    acquire_timeout_seconds: u64,
    daemon_config_load_ms: u64,
    daemon_config_load_us: u64,
) -> std::result::Result<LockActivation, AttemptFailure> {
    let activation_started_at = Instant::now();
    let socket_setup_started_at = Instant::now();
    let notify_path =
        process::notify_socket_path().map_err(|error| AttemptFailure::retryable(error, false))?;
    let auth_socket_path =
        ipc::auth_socket_path().map_err(|error| AttemptFailure::retryable(error, false))?;
    let control_socket_path =
        process::control_socket_path().map_err(|error| AttemptFailure::retryable(error, false))?;
    let session = session_proxy.inner().path().as_str();
    let owner_path = ownership::record_path(session)
        .map_err(|error| AttemptFailure::unsafe_to_retry(error, false))?;
    if owner_path.exists() {
        return Err(AttemptFailure::unsafe_to_retry(
            anyhow!(
                "curtain ownership is unresolved at {}",
                owner_path.display()
            ),
            false,
        ));
    }
    let notify_listener = ipc::bind_listener(&notify_path)
        .await
        .map_err(|error| AttemptFailure::retryable(error, false))?;
    let auth_listener = match ipc::bind_listener(&auth_socket_path).await {
        Ok(listener) => listener,
        Err(error) => {
            remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
            return Err(AttemptFailure::retryable(error, false));
        }
    };
    let socket_setup_elapsed_ms = elapsed_ms(socket_setup_started_at);
    let socket_setup_elapsed_us = elapsed_us(socket_setup_started_at);

    let mut owner = OwnerRecord::pending(
        session,
        auth_socket_path.clone(),
        control_socket_path.clone(),
    );
    if let Err(error) = ownership::publish(&owner_path, &owner) {
        remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
        return Err(AttemptFailure::unsafe_to_retry(error, false));
    }

    let spawn_started_at = Instant::now();
    let mut child = match process::spawn_curtain(
        &notify_path,
        &auth_socket_path,
        &control_socket_path,
        &owner_path,
        config_path,
        initial_background_path,
        force_emergency_ui,
        latency_report,
    )
    .await
    {
        Ok(child) => child,
        Err(error) => {
            let _ = ownership::remove(&owner_path);
            remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
            return Err(AttemptFailure::retryable(error, false));
        }
    };
    let identity = child
        .id()
        .ok_or_else(|| anyhow!("spawned curtain has no PID"))
        .and_then(|pid| owner.set_process(pid))
        .and_then(|()| ownership::publish(&owner_path, &owner));
    if let Err(error) = identity {
        let _ = process::force_stop_curtain(child).await;
        let _ = ownership::remove(&owner_path);
        remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
        return Err(AttemptFailure::unsafe_to_retry(error, false));
    }
    owner.mark_gate_opening();
    if let Err(error) = ownership::publish(&owner_path, &owner) {
        if let Err(stop_error) = process::force_stop_curtain(child).await {
            return Err(AttemptFailure::unsafe_to_retry(
                stop_error.context(error),
                false,
            ));
        }
        let _ = ownership::remove(&owner_path);
        remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
        return Err(AttemptFailure::unsafe_to_retry(error, false));
    }
    if let Err(error) = process::release_curtain_owner_gate(
        &mut child,
        weather_snapshot,
        battery_snapshot,
        now_playing_snapshot,
    )
    .await
    {
        if let Err(stop_error) = process::force_stop_curtain(child).await {
            // partial gate write may have let the curtain acquire the lock
            return Err(AttemptFailure::unsafe_to_retry(
                stop_error.context(error),
                false,
            ));
        }
        ownership::remove(&owner_path)
            .map_err(|remove_error| AttemptFailure::unsafe_to_retry(remove_error, false))?;
        remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
        return Err(AttemptFailure::unsafe_to_retry(error, false));
    }
    if !matches!(state, LockState::Locked) {
        *state = LockState::Locking;
    }
    let spawn_elapsed_ms = elapsed_ms(spawn_started_at);
    let spawn_elapsed_us = elapsed_us(spawn_started_at);
    let (auth_sender, auth_results) = unbounded_channel();
    let ready_wait_started_at = Instant::now();
    let deadline = TokioInstant::now() + startup_timeout(acquire_timeout_seconds);
    let mut session_locked = false;
    let mut curtain_latency_report = None;
    let mut ready_received = false;

    loop {
        tokio::select! {
            accepted = notify_listener.accept() => {
                match accepted {
                    Ok((stream, _)) => match read_startup_message(stream).await {
                        Ok(Some(CurtainStartupMessage::SessionLocked)) => {
                            if !session_locked {
                                session_locked = true;
                                *state = LockState::Locked;
                                update_locked_hint(session_proxy, true).await;
                                tracing::info!(trigger, "compositor confirmed the curtain session lock");
                            }
                        }
                        Ok(Some(CurtainStartupMessage::Ready { latency_report: report })) => {
                            if !session_locked {
                                tracing::warn!("ignoring curtain readiness before compositor lock confirmation");
                                continue;
                            }
                            curtain_latency_report = report.map(|report| *report);
                            ready_received = true;
                            break;
                        }
                        Ok(None) => tracing::warn!("curtain closed startup connection without a message"),
                        Err(error) => tracing::warn!("failed to read curtain startup message: {error:#}"),
                    },
                    Err(error) => {
                        let error = anyhow!(error).context("failed to accept curtain startup notification");
                        if session_locked {
                            tracing::warn!("{error:#}; preserving the locked curtain");
                            break;
                        }
                        if let Err(stop_error) = process::force_stop_curtain(child).await {
                            remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
                            return Err(AttemptFailure::unsafe_to_retry(
                                stop_error.context(error),
                                false,
                            ));
                        }
                        let _ = ownership::remove(&owner_path);
                        remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
                        return Err(AttemptFailure::retryable(error, false));
                    }
                }
            }
            status = child.wait() => {
                let status = match status {
                    Ok(status) => status,
                    Err(error) => {
                        tracing::warn!(
                            "failed while waiting for curtain exit before readiness: {error}; retrying wait"
                        );
                        tokio::time::sleep(Duration::from_millis(250)).await;
                        continue;
                    }
                };
                if !session_locked {
                    let _ = ownership::remove(&owner_path);
                    remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
                }
                return Err(AttemptFailure::unsafe_to_retry(
                    anyhow!("curtain exited before readiness with status {status}"),
                    session_locked,
                ));
            }
            () = sleep_until(deadline) => {
                if session_locked {
                    tracing::warn!(
                        acquire_timeout_seconds,
                        "curtain readiness timed out after compositor lock confirmation; preserving the curtain and enabling authentication"
                    );
                    break;
                }

                let timeout_error = anyhow!("timed out waiting for compositor lock confirmation");
                if let Err(error) = process::force_stop_curtain(child).await {
                    remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
                    return Err(AttemptFailure::unsafe_to_retry(
                        error.context(timeout_error),
                        false,
                    ));
                }
                let _ = ownership::remove(&owner_path);
                remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
                return Err(AttemptFailure::retryable(timeout_error, false));
            }
        }
    }

    let ready_wait_elapsed_ms = elapsed_ms(ready_wait_started_at);
    let ready_wait_elapsed_us = elapsed_us(ready_wait_started_at);
    let _ = std::fs::remove_file(&notify_path);
    let activation_elapsed_ms = elapsed_ms(activation_started_at);
    let activation_elapsed_us = elapsed_us(activation_started_at);
    let ready = ready_received;
    let latency_report = latency_report.is_enabled().then_some(LockLatencyReport {
        daemon_config_load_ms,
        daemon_config_load_us,
        socket_setup_ms: socket_setup_elapsed_ms,
        socket_setup_us: socket_setup_elapsed_us,
        curtain_spawn_ms: spawn_elapsed_ms,
        curtain_spawn_us: spawn_elapsed_us,
        curtain_ready_wait_ms: ready_wait_elapsed_ms,
        curtain_ready_wait_us: ready_wait_elapsed_us,
        activation_total_ms: activation_elapsed_ms,
        activation_total_us: activation_elapsed_us,
        curtain: curtain_latency_report,
    });
    if let Some(report) = latency_report.as_ref() {
        log_latency_report(report);
    }
    tracing::info!(
        trigger,
        socket_setup_elapsed_ms,
        spawn_elapsed_ms,
        ready_wait_elapsed_ms,
        activation_elapsed_ms,
        ready,
        "curtain startup completed; session considered locked"
    );
    Ok(LockActivation {
        active: ActiveLock {
            curtain: CurtainHandle::Spawned { child, owner_path },
            auth_listener,
            auth_socket_path,
            control_socket_path,
            auth_results,
            auth_sender,
        },
        latency_report,
    })
}
