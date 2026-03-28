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
use veila_common::{BatterySnapshot, NowPlayingSnapshot, WeatherSnapshot};

use super::state::{ActiveRuntime, LockActivation, reset_runtime, update_locked_hint};

pub(crate) async fn activate_lock(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    config_path: Option<&std::path::Path>,
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
) -> Result<LockActivation> {
    *state = LockState::Locking;

    let notify_path = process::notify_socket_path();
    let auth_socket_path = ipc::auth_socket_path();
    let control_socket_path = process::control_socket_path();
    let notify_listener = ipc::bind_listener(&notify_path).await?;
    let auth_listener = ipc::bind_listener(&auth_socket_path).await?;
    let mut child = process::spawn_curtain(
        &notify_path,
        &auth_socket_path,
        &control_socket_path,
        config_path,
        weather_snapshot,
        battery_snapshot,
        now_playing_snapshot,
    )
    .await?;
    let (auth_sender, auth_results) = unbounded_channel();
    let ready_result = tokio::select! {
        ready = timeout(Duration::from_secs(5), notify_listener.accept()) => ReadyResult::Ready(ready),
        status = child.wait() => ReadyResult::Exited(
            status.context("failed while waiting for curtain exit before readiness")?
        ),
    };
    let _ = std::fs::remove_file(&notify_path);

    match ready_result {
        ReadyResult::Ready(Ok(Ok((_stream, _addr)))) => {
            *state = LockState::Locked;
            update_locked_hint(session_proxy, true).await;
            tracing::info!("curtain ready; session considered locked");
            Ok(LockActivation {
                curtain: child,
                auth_listener,
                auth_socket_path,
                control_socket_path,
                auth_results,
                auth_sender,
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
