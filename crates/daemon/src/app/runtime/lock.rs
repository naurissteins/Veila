use std::{path::Path, time::Instant};

use anyhow::{Context, Result, anyhow};
use tokio::{
    process::Child,
    sync::mpsc::unbounded_channel,
    time::{Duration, Instant as TokioInstant, sleep_until, timeout},
};

use crate::{
    adapters::{ipc, logind, process},
    domain::{
        auth::{AuthPolicy, AuthState},
        lock_state::LockState,
    },
};
use veila_common::{
    BatterySnapshot, NowPlayingSnapshot, WeatherSnapshot, elapsed_ms, elapsed_us,
    ipc::{CurtainStartupMessage, LatencyReportMode, LockLatencyReport},
};

use super::state::{ActiveRuntime, LockActivation, reset_runtime, update_locked_hint};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn activate_lock(
    trigger: &'static str,
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    config_path: Option<&std::path::Path>,
    initial_background_path: Option<&std::path::Path>,
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
    force_emergency_ui: bool,
    latency_report: LatencyReportMode,
    acquire_timeout_seconds: u64,
    daemon_config_load_ms: u64,
    daemon_config_load_us: u64,
) -> Result<LockActivation> {
    let mut lock_was_confirmed = false;

    for emergency_retry in [false, true] {
        let result = activate_lock_attempt(
            trigger,
            session_proxy,
            state,
            if emergency_retry { None } else { config_path },
            if emergency_retry {
                None
            } else {
                initial_background_path
            },
            if emergency_retry {
                None
            } else {
                weather_snapshot
            },
            if emergency_retry {
                None
            } else {
                battery_snapshot
            },
            if emergency_retry {
                None
            } else {
                now_playing_snapshot
            },
            force_emergency_ui || emergency_retry,
            if emergency_retry {
                LatencyReportMode::Disabled
            } else {
                latency_report
            },
            acquire_timeout_seconds,
            daemon_config_load_ms,
            daemon_config_load_us,
        )
        .await;

        match result {
            Ok(activation) => return Ok(activation),
            Err(failure) => {
                lock_was_confirmed |= failure.session_locked;
                *state = if lock_was_confirmed {
                    LockState::Locked
                } else {
                    LockState::Unlocked
                };

                if !emergency_retry && failure.retry_safe {
                    tracing::error!(
                        session_locked = failure.session_locked,
                        "curtain failed before readiness: {:#}; retrying with emergency UI",
                        failure.error
                    );
                    continue;
                }

                update_locked_hint(session_proxy, lock_was_confirmed).await;
                return Err(failure.error).context(if emergency_retry {
                    "emergency curtain retry failed"
                } else {
                    "curtain activation failed and could not be retried safely"
                });
            }
        }
    }

    Err(anyhow!("curtain activation attempts exhausted"))
}

const STARTUP_TIMEOUT_MARGIN: Duration = Duration::from_secs(1);
const STARTUP_MESSAGE_TIMEOUT: Duration = Duration::from_secs(2);

struct AttemptFailure {
    error: anyhow::Error,
    session_locked: bool,
    retry_safe: bool,
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
async fn activate_lock_attempt(
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

    let spawn_started_at = Instant::now();
    let mut child = match process::spawn_curtain(
        &notify_path,
        &auth_socket_path,
        &control_socket_path,
        config_path,
        initial_background_path,
        weather_snapshot,
        battery_snapshot,
        now_playing_snapshot,
        force_emergency_ui,
        latency_report,
    )
    .await
    {
        Ok(child) => child,
        Err(error) => {
            remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
            return Err(AttemptFailure::retryable(error, false));
        }
    };
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
                remove_activation_sockets(&notify_path, &auth_socket_path, &control_socket_path);
                return Err(AttemptFailure::retryable(
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
        curtain: child,
        auth_listener,
        auth_socket_path,
        control_socket_path,
        auth_results,
        auth_sender,
        latency_report,
    })
}

async fn read_startup_message(
    mut stream: tokio::net::UnixStream,
) -> Result<Option<CurtainStartupMessage>> {
    let line = timeout(
        STARTUP_MESSAGE_TIMEOUT,
        ipc::read_ipc_line(&mut stream, "curtain startup message"),
    )
    .await
    .context("timed out reading curtain startup message")??;
    line.map(|line| veila_common::ipc::decode_message(&line))
        .transpose()
        .context("invalid curtain startup message")
}

fn remove_activation_sockets(notify_path: &Path, auth_path: &Path, control_path: &Path) {
    let _ = std::fs::remove_file(notify_path);
    let _ = std::fs::remove_file(auth_path);
    let _ = std::fs::remove_file(control_path);
}

fn startup_timeout(acquire_timeout_seconds: u64) -> Duration {
    Duration::from_secs(acquire_timeout_seconds.max(1)) + STARTUP_TIMEOUT_MARGIN
}

fn log_latency_report(report: &LockLatencyReport) {
    let curtain = report.curtain.as_ref();
    tracing::info!(
        daemon_config_load_ms = report.daemon_config_load_ms,
        daemon_config_load_us = report.daemon_config_load_us,
        socket_setup_ms = report.socket_setup_ms,
        socket_setup_us = report.socket_setup_us,
        curtain_spawn_ms = report.curtain_spawn_ms,
        curtain_spawn_us = report.curtain_spawn_us,
        curtain_ready_wait_ms = report.curtain_ready_wait_ms,
        curtain_ready_wait_us = report.curtain_ready_wait_us,
        activation_total_ms = report.activation_total_ms,
        activation_total_us = report.activation_total_us,
        curtain_wayland_connect_ms = curtain.map(|report| report.wayland_connect_ms),
        curtain_wayland_connect_us = curtain.map(|report| report.wayland_connect_us),
        curtain_registry_ms = curtain.map(|report| report.registry_ms),
        curtain_registry_us = curtain.map(|report| report.registry_us),
        curtain_event_loop_ms = curtain.map(|report| report.event_loop_ms),
        curtain_event_loop_us = curtain.map(|report| report.event_loop_us),
        curtain_app_init_ms = curtain.map(|report| report.app_init_ms),
        curtain_app_init_us = curtain.map(|report| report.app_init_us),
        curtain_lock_request_ms = curtain.map(|report| report.lock_request_ms),
        curtain_lock_request_us = curtain.map(|report| report.lock_request_us),
        curtain_startup_prepared_ms = curtain.map(|report| report.startup_prepared_ms),
        curtain_startup_prepared_us = curtain.map(|report| report.startup_prepared_us),
        first_surface_configured_ms = curtain.and_then(|report| report.first_surface_configured_ms),
        first_surface_configured_us = curtain.and_then(|report| report.first_surface_configured_us),
        all_surfaces_configured_ms = curtain.and_then(|report| report.all_surfaces_configured_ms),
        all_surfaces_configured_us = curtain.and_then(|report| report.all_surfaces_configured_us),
        session_locked_ms = curtain.and_then(|report| report.session_locked_ms),
        session_locked_us = curtain.and_then(|report| report.session_locked_us),
        first_frame_ms = curtain.and_then(|report| report.first_frame_ms),
        first_frame_us = curtain.and_then(|report| report.first_frame_us),
        ready_notified_ms = curtain.and_then(|report| report.ready_notified_ms),
        ready_notified_us = curtain.and_then(|report| report.ready_notified_us),
        surface_count = curtain.map(|report| report.surface_count),
        "lock latency report"
    );
}

pub(crate) async fn deactivate_lock(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    runtime: ActiveRuntime<'_>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
    attempt_id: Option<u64>,
) -> Result<()> {
    let started_at = Instant::now();
    if runtime.curtain.is_none() {
        *state = LockState::Unlocked;
        reset_runtime(
            runtime.auth_listener,
            runtime.auth_socket_path,
            runtime.control_socket_path,
            runtime.auth_results,
            runtime.auth_sender,
            auth_policy,
            auth_state,
        );
        update_locked_hint(session_proxy, false).await;
        tracing::info!(
            elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            "deactivate_lock completed without active curtain"
        );
        return Ok(());
    }

    *state = LockState::Unlocking;

    if let Some(child) = runtime.curtain.take() {
        match stop_active_curtain(child, runtime.control_socket_path.as_deref(), attempt_id).await {
            CurtainStop::Released => {}
            CurtainStop::UnlockUndeliverable(child) => {
                *runtime.curtain = Some(child);
                *state = LockState::Locked;
                return Err(anyhow!(
                    "could not deliver the unlock to the curtain; session stays locked"
                ));
            }
        }
    }

    reset_runtime(
        runtime.auth_listener,
        runtime.auth_socket_path,
        runtime.control_socket_path,
        runtime.auth_results,
        runtime.auth_sender,
        auth_policy,
        auth_state,
    );
    *state = LockState::Unlocked;
    update_locked_hint(session_proxy, false).await;

    let elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    if let Some(attempt_id) = attempt_id {
        tracing::info!(
            attempt_id,
            elapsed_ms,
            "curtain stopped; session considered unlocked"
        );
    } else {
        tracing::info!(elapsed_ms, "curtain stopped; session considered unlocked");
    }
    Ok(())
}

pub(crate) enum CurtainStop {
    /// The curtain was told to unlock and is no longer holding the session lock.
    Released,
    /// The unlock never reached the curtain, which therefore still holds the lock.
    UnlockUndeliverable(Child),
}

const GRACEFUL_EXIT_WINDOW: Duration = Duration::from_secs(5);
const UNLOCK_DELIVERY_ATTEMPTS: u32 = 3;
const UNLOCK_RETRY_DELAY: Duration = Duration::from_millis(100);

async fn stop_active_curtain(
    child: Child,
    control_socket_path: Option<&Path>,
    attempt_id: Option<u64>,
) -> CurtainStop {
    let Some(control_socket_path) = control_socket_path else {
        // No control socket means this curtain is not daemon-managed, so it self-authorizes on
        // a termination signal and stopping it is enough.
        force_stop(child).await;
        return CurtainStop::Released;
    };

    if let Err(error) = deliver_unlock(control_socket_path, attempt_id).await {
        tracing::error!("could not deliver unlock to the curtain: {error:#}");
        return CurtainStop::UnlockUndeliverable(child);
    }

    match process::wait_for_graceful_curtain_exit(child, GRACEFUL_EXIT_WINDOW).await {
        Ok(None) => CurtainStop::Released,
        Ok(Some(child)) => {
            // The unlock was delivered, so the curtain has already recorded it as authorized and
            // will release the lock when the signal makes it exit.
            tracing::warn!("curtain did not exit after an authorized unlock; forcing stop");
            force_stop(child).await;
            CurtainStop::Released
        }
        Err(error) => {
            tracing::error!("failed while waiting for curtain to exit: {error:#}");
            CurtainStop::Released
        }
    }
}

async fn deliver_unlock(control_socket_path: &Path, attempt_id: Option<u64>) -> Result<()> {
    let mut last_error = None;

    for attempt in 1..=UNLOCK_DELIVERY_ATTEMPTS {
        match process::request_curtain_unlock(control_socket_path, attempt_id).await {
            Ok(()) => return Ok(()),
            Err(error) => {
                tracing::warn!(attempt, "failed to request curtain unlock: {error:#}");
                last_error = Some(error);
                if attempt < UNLOCK_DELIVERY_ATTEMPTS {
                    tokio::time::sleep(UNLOCK_RETRY_DELAY).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("unlock delivery failed")))
}

async fn force_stop(child: Child) {
    if let Err(error) = process::force_stop_curtain(child).await {
        tracing::error!("failed to force stop the curtain: {error:#}");
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{Instant, SystemTime, UNIX_EPOCH},
    };

    use tokio::{io::AsyncWriteExt, net::UnixListener};
    use veila_common::ipc::{CurtainStartupMessage, encode_message};

    use super::{
        UNLOCK_DELIVERY_ATTEMPTS, UNLOCK_RETRY_DELAY, deliver_unlock, read_startup_message,
        startup_timeout,
    };

    fn unique_socket_path(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "veila-test-{label}-{}-{stamp}.sock",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn retries_before_reporting_an_undeliverable_unlock() {
        let path = unique_socket_path("unlock-missing");
        let started_at = Instant::now();

        deliver_unlock(&path, Some(1))
            .await
            .expect_err("a missing control socket must not report success");

        // Every attempt but the last is followed by a backoff sleep, so the elapsed time is
        // observable evidence that the retries actually ran.
        let minimum = UNLOCK_RETRY_DELAY * (UNLOCK_DELIVERY_ATTEMPTS - 1);
        assert!(
            started_at.elapsed() >= minimum,
            "expected at least {minimum:?} of retry backoff, took {:?}",
            started_at.elapsed()
        );
    }

    #[tokio::test]
    async fn delivers_unlock_to_a_listening_curtain() {
        let path = unique_socket_path("unlock-delivered");
        let listener = UnixListener::bind(&path).expect("bind control socket");

        deliver_unlock(&path, Some(7))
            .await
            .expect("unlock should reach a listening curtain");

        drop(listener);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn startup_timeout_uses_configured_acquisition_window_plus_margin() {
        assert_eq!(startup_timeout(9), std::time::Duration::from_secs(10));
    }

    #[tokio::test]
    async fn reads_session_locked_startup_message() {
        let path = unique_socket_path("startup-session-locked");
        let listener = UnixListener::bind(&path).expect("bind startup socket");
        let mut client = tokio::net::UnixStream::connect(&path)
            .await
            .expect("connect startup socket");
        let (server, _) = listener.accept().await.expect("accept startup connection");
        let mut payload =
            encode_message(&CurtainStartupMessage::SessionLocked).expect("encode message");
        payload.push('\n');
        client
            .write_all(payload.as_bytes())
            .await
            .expect("write startup message");

        let message = read_startup_message(server)
            .await
            .expect("read startup message");

        assert_eq!(message, Some(CurtainStartupMessage::SessionLocked));
        drop(listener);
        std::fs::remove_file(&path).ok();
    }
}
