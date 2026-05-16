use std::{path::Path, process::ExitStatus, time::Instant};

use anyhow::{Context, Result, anyhow};
use tokio::{
    net::UnixStream,
    process::Child,
    sync::mpsc::unbounded_channel,
    time::{Duration, timeout},
};

use crate::{
    adapters::{ipc, logind, process},
    domain::{
        auth::{AuthPolicy, AuthState},
        lock_state::LockState,
    },
};
use veila_common::{
    BatterySnapshot, NowPlayingSnapshot, WeatherSnapshot,
    ipc::{CurtainLatencyReport, LatencyReportMode, LockLatencyReport},
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
    daemon_config_load_ms: u64,
    daemon_config_load_us: u64,
) -> Result<LockActivation> {
    let activation_started_at = Instant::now();

    let socket_setup_started_at = Instant::now();
    let notify_path = process::notify_socket_path()?;
    let auth_socket_path = ipc::auth_socket_path()?;
    let control_socket_path = process::control_socket_path()?;
    let notify_listener = ipc::bind_listener(&notify_path).await?;
    let auth_listener = ipc::bind_listener(&auth_socket_path).await?;
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
            let _ = std::fs::remove_file(&notify_path);
            let _ = std::fs::remove_file(&auth_socket_path);
            let _ = std::fs::remove_file(&control_socket_path);
            *state = LockState::Unlocked;
            return Err(error);
        }
    };
    *state = LockState::Locking;
    let spawn_elapsed_ms = elapsed_ms(spawn_started_at);
    let spawn_elapsed_us = elapsed_us(spawn_started_at);
    let (auth_sender, auth_results) = unbounded_channel();
    let ready_wait_started_at = Instant::now();
    let ready_result = tokio::select! {
        ready = timeout(Duration::from_secs(5), notify_listener.accept()) => ReadyResult::Ready(ready),
        status = child.wait() => ReadyResult::Exited(
            status.context("failed while waiting for curtain exit before readiness")?
        ),
    };
    let ready_wait_elapsed_ms = elapsed_ms(ready_wait_started_at);
    let ready_wait_elapsed_us = elapsed_us(ready_wait_started_at);
    let _ = std::fs::remove_file(&notify_path);

    match ready_result {
        ReadyResult::Ready(Ok(Ok((stream, _addr)))) => {
            let curtain_latency_report = if latency_report.is_enabled() {
                read_curtain_latency_report(stream).await
            } else {
                None
            };
            *state = LockState::Locked;
            let locked_hint_started_at = Instant::now();
            update_locked_hint(session_proxy, true).await;
            let locked_hint_elapsed_ms = elapsed_ms(locked_hint_started_at);
            let activation_elapsed_ms = elapsed_ms(activation_started_at);
            let activation_elapsed_us = elapsed_us(activation_started_at);
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
                locked_hint_elapsed_ms,
                activation_elapsed_ms,
                "curtain ready; session considered locked"
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
        ReadyResult::Ready(Ok(Err(error))) => {
            *state = LockState::Unlocked;
            let _ = std::fs::remove_file(&auth_socket_path);
            let _ = std::fs::remove_file(&control_socket_path);
            process::force_stop_curtain(child).await?;
            update_locked_hint(session_proxy, false).await;
            Err(error).context("failed while waiting for curtain readiness")
        }
        ReadyResult::Ready(Err(_)) => {
            *state = LockState::Unlocked;
            let _ = std::fs::remove_file(&auth_socket_path);
            let _ = std::fs::remove_file(&control_socket_path);
            process::force_stop_curtain(child).await?;
            update_locked_hint(session_proxy, false).await;
            Err(anyhow!("timed out waiting for curtain readiness"))
        }
        ReadyResult::Exited(status) => {
            *state = LockState::Unlocked;
            let _ = std::fs::remove_file(&auth_socket_path);
            let _ = std::fs::remove_file(&control_socket_path);
            update_locked_hint(session_proxy, false).await;
            Err(anyhow!(
                "curtain exited before readiness with status {status}. \
If you ran `cargo run -p veila-daemon` after changing curtain startup arguments or shared runtime wiring, rebuild the workspace with `cargo build --workspace` so `target/debug/veila-curtain` matches the daemon"
            ))
        }
    }
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn elapsed_us(started_at: Instant) -> u64 {
    started_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

async fn read_curtain_latency_report(stream: UnixStream) -> Option<CurtainLatencyReport> {
    let mut stream = stream;
    match ipc::read_ipc_line(&mut stream, "curtain latency report").await {
        Ok(None) => None,
        Ok(Some(line)) => match veila_common::ipc::decode_message(&line) {
            Ok(report) => Some(report),
            Err(error) => {
                tracing::warn!("failed to decode curtain latency report: {error:#}");
                None
            }
        },
        Err(error) => {
            tracing::warn!("failed to read curtain latency report: {error:#}");
            None
        }
    }
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
        stop_active_curtain(child, runtime.control_socket_path.as_deref(), attempt_id).await?;
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

enum ReadyResult {
    Ready(
        std::result::Result<
            std::io::Result<(UnixStream, tokio::net::unix::SocketAddr)>,
            tokio::time::error::Elapsed,
        >,
    ),
    Exited(ExitStatus),
}

async fn stop_active_curtain(
    child: Child,
    control_socket_path: Option<&Path>,
    attempt_id: Option<u64>,
) -> Result<()> {
    let child = if let Some(control_socket_path) = control_socket_path {
        match process::request_curtain_unlock(control_socket_path, attempt_id).await {
            Ok(()) => {
                match process::wait_for_graceful_curtain_exit(child, Duration::from_secs(5)).await?
                {
                    Some(child) => child,
                    None => return Ok(()),
                }
            }
            Err(error) => {
                tracing::warn!("failed to request graceful curtain unlock: {error:#}");
                child
            }
        }
    } else {
        child
    };

    process::force_stop_curtain(child).await
}
