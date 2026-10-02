use crate::adapters::ipc;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::time::{Duration, timeout};
use veila_common::ipc::{CurtainStartupMessage, LockLatencyReport};

const STARTUP_TIMEOUT_MARGIN: Duration = Duration::from_secs(1);
const STARTUP_MESSAGE_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) async fn read_startup_message(
    mut stream: tokio::net::UnixStream,
) -> Result<Option<CurtainStartupMessage>> {
    ipc::verify_peer_uid(&stream).context("curtain startup socket peer rejected")?;
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

pub(super) fn remove_activation_sockets(notify_path: &Path, auth_path: &Path, control_path: &Path) {
    let _ = std::fs::remove_file(notify_path);
    let _ = std::fs::remove_file(auth_path);
    let _ = std::fs::remove_file(control_path);
}

pub(super) fn startup_timeout(acquire_timeout_seconds: u64) -> Duration {
    Duration::from_secs(acquire_timeout_seconds.max(1)) + STARTUP_TIMEOUT_MARGIN
}

pub(super) fn log_latency_report(report: &LockLatencyReport) {
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
        placeholder_committed_us = curtain.and_then(|report| report.placeholder_committed_us),
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

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
