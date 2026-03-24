mod helpers;
mod output_probe;
mod prewarm;
mod runtime;

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{
    DaemonOptions,
    adapters::{ipc, logind, process},
    domain::{
        auth::{AuthPolicy, AuthState},
        lock_state::LockState,
    },
};
use anyhow::{Context, Result};
use futures_util::StreamExt;
use tokio::{
    net::UnixListener,
    signal::unix::{SignalKind, signal},
};
use veila_common::AppConfig;
use veila_common::ipc::{DaemonControlMessage, DaemonControlResponse};

use self::helpers::{
    activate_and_install, build_daemon_health, build_daemon_status, current_username,
    reload_config_response, try_activate_and_install,
};
use self::runtime::{
    ActiveRuntime, AuthResult, accept_auth_connection, accept_control_connection, deactivate_lock,
    handle_client_message, receive_auth_result, reset_runtime, update_locked_hint,
    wait_for_curtain_exit,
};

pub async fn run(
    options: DaemonOptions,
    mut control_listener: UnixListener,
    daemon_control_socket_path: PathBuf,
) -> Result<()> {
    let mut loaded_config =
        AppConfig::load(options.config_path.as_deref()).context("failed to load daemon config")?;
    prewarm::spawn_background_prewarm(&loaded_config.config);
    let mut auth_policy = AuthPolicy::new(
        Duration::from_millis(loaded_config.config.lock.auth_backoff_base_ms),
        Duration::from_secs(loaded_config.config.lock.auth_backoff_max_seconds),
    );
    let connection = logind::connect_system().await?;
    let session_path = logind::get_session_path(&connection, options.session_id.as_deref()).await?;
    let session_proxy = logind::session_proxy(&connection, &session_path).await?;
    let username = current_username()?;
    let mut lock_stream = session_proxy
        .receive_lock()
        .await
        .context("failed to subscribe to logind Lock signal")?;
    let mut unlock_stream = session_proxy
        .receive_unlock()
        .await
        .context("failed to subscribe to logind Unlock signal")?;
    let mut sigint =
        signal(SignalKind::interrupt()).context("failed to register SIGINT handler")?;
    let mut sigterm =
        signal(SignalKind::terminate()).context("failed to register SIGTERM handler")?;

    let mut state = LockState::Unlocked;
    let mut curtain = None;
    let mut auth_listener = None;
    let mut auth_socket_path = None;
    let mut control_socket_path = None;
    let mut auth_results = None;
    let mut auth_sender = None;
    let mut auth_state = AuthState::new(auth_policy);
    let mut standby_curtain: Option<tokio::process::Child> = None;
    let mut standby_control_socket: Option<PathBuf> = None;

    tracing::info!(
        session = %session_path,
        session_id_override = options.session_id.as_deref().unwrap_or("none"),
        manual_lock = options.lock_now,
        config = loaded_config.path.as_deref().map(|path| path.display().to_string()).unwrap_or_else(|| "defaults".to_string()),
        "veilad ready"
    );

    if !options.lock_now {
        spawn_standby(
            options.config_path.as_deref(),
            &mut standby_curtain,
            &mut standby_control_socket,
        )
        .await;
    }

    if options.lock_now {
        tracing::info!("manual lock requested via --lock-now");
        try_activate_and_install(
            &session_proxy,
            &mut state,
            take_standby(&mut standby_curtain, &mut standby_control_socket),
            options.config_path.as_deref(),
            ActiveRuntime::new(
                &mut curtain,
                &mut auth_listener,
                &mut auth_socket_path,
                &mut control_socket_path,
                &mut auth_results,
                &mut auth_sender,
            ),
            auth_policy,
            &mut auth_state,
        )
        .await
        .context("failed to activate manual lock")?;
        spawn_standby(
            options.config_path.as_deref(),
            &mut standby_curtain,
            &mut standby_control_socket,
        )
        .await;
    }

    loop {
        tokio::select! {
            Some(_) = lock_stream.next() => {
                if state.is_active() {
                    tracing::debug!(state = %state, "ignoring duplicate lock signal");
                    continue;
                }

                if let Err(error) = try_activate_and_install(
                    &session_proxy,
                    &mut state,
                    take_standby(&mut standby_curtain, &mut standby_control_socket),
                    options.config_path.as_deref(),
                    ActiveRuntime::new(
                        &mut curtain,
                        &mut auth_listener,
                        &mut auth_socket_path,
                        &mut control_socket_path,
                        &mut auth_results,
                        &mut auth_sender,
                    ),
                    auth_policy,
                    &mut auth_state,
                ).await {
                    tracing::error!("failed to activate lock: {error:#}");
                } else {
                    spawn_standby(
                        options.config_path.as_deref(),
                        &mut standby_curtain,
                        &mut standby_control_socket,
                    ).await;
                }
            }
            Some(_) = unlock_stream.next() => {
                if !state.is_active() {
                    tracing::debug!(state = %state, "ignoring unlock signal while not locked");
                    continue;
                }

                if let Err(error) = deactivate_lock(
                    &session_proxy,
                    &mut state,
                    ActiveRuntime::new(
                        &mut curtain,
                        &mut auth_listener,
                        &mut auth_socket_path,
                        &mut control_socket_path,
                        &mut auth_results,
                        &mut auth_sender,
                    ),
                    auth_policy,
                    &mut auth_state,
                    None,
                ).await {
                    tracing::error!("failed to deactivate lock: {error:#}");
                } else {
                    spawn_standby(
                        options.config_path.as_deref(),
                        &mut standby_curtain,
                        &mut standby_control_socket,
                    ).await;
                }
            }
            result = wait_for_curtain_exit(&mut curtain), if curtain.is_some() => {
                let status = result?;
                tracing::warn!(?status, state = %state, "curtain exited");
                curtain.take();
                reset_runtime(
                    &mut auth_listener,
                    &mut auth_socket_path,
                    &mut control_socket_path,
                    &mut auth_results,
                    &mut auth_sender,
                    auth_policy,
                    &mut auth_state,
                );

                if state.is_active() {
                    update_locked_hint(&session_proxy, false).await;
                    state = LockState::Unlocked;
                    tracing::error!("curtain exited while the session should be locked; attempting restart");

                    if let Err(error) = activate_and_install(
                        &session_proxy,
                        &mut state,
                        options.config_path.as_deref(),
                        ActiveRuntime::new(
                            &mut curtain,
                            &mut auth_listener,
                            &mut auth_socket_path,
                            &mut control_socket_path,
                            &mut auth_results,
                            &mut auth_sender,
                        ),
                        auth_policy,
                        &mut auth_state,
                    ).await {
                        tracing::error!("failed to restart curtain after unexpected exit: {error:#}");
                    } else {
                        spawn_standby(
                            options.config_path.as_deref(),
                            &mut standby_curtain,
                            &mut standby_control_socket,
                        ).await;
                    }
                } else {
                    spawn_standby(
                        options.config_path.as_deref(),
                        &mut standby_curtain,
                        &mut standby_control_socket,
                    ).await;
                }
            }
            result = wait_for_curtain_exit(&mut standby_curtain), if standby_curtain.is_some() => {
                match result {
                    Ok(status) => tracing::info!(?status, "standby curtain exited"),
                    Err(error) => tracing::warn!("error waiting for standby curtain: {error:#}"),
                }
                standby_curtain.take();
                if let Some(path) = standby_control_socket.take() {
                    let _ = std::fs::remove_file(path);
                }
            }
            result = accept_auth_connection(&mut auth_listener), if matches!(state, LockState::Locked) && auth_listener.is_some() => {
                let mut stream = result?;
                if let Some(message) = crate::adapters::ipc::read_client_message(&mut stream).await?
                    && let Err(error) = handle_client_message(
                        &username,
                        &mut auth_state,
                        &auth_sender,
                        stream,
                    message,
                    ).await
                {
                    tracing::warn!("failed to handle auth request: {error:#}");
                }
            }
            result = receive_auth_result(&mut auth_results), if auth_results.is_some() => {
                let Some(result) = result else {
                    continue;
                };

                match result {
                    AuthResult::Succeeded {
                        attempt_id,
                        started_at,
                        elapsed_ms,
                    } => {
                        tracing::info!(attempt_id, elapsed_ms, "starting unlock after successful authentication");
                        auth_state.finish_success();
                        let unlock_started_at = Instant::now();

                        if let Err(error) = deactivate_lock(
                            &session_proxy,
                            &mut state,
                            ActiveRuntime::new(
                                &mut curtain,
                                &mut auth_listener,
                                &mut auth_socket_path,
                                &mut control_socket_path,
                                &mut auth_results,
                                &mut auth_sender,
                            ),
                            auth_policy,
                            &mut auth_state,
                            Some(attempt_id),
                        ).await {
                            tracing::error!("failed to unlock after successful authentication: {error:#}");
                        } else {
                            tracing::info!(
                                attempt_id,
                                auth_elapsed_ms = elapsed_ms,
                                unlock_elapsed_ms = unlock_started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                                daemon_total_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                                "unlock timing summary"
                            );
                            spawn_standby(
                                options.config_path.as_deref(),
                                &mut standby_curtain,
                                &mut standby_control_socket,
                            ).await;
                        }
                    }
                    AuthResult::Rejected {
                        attempt_id,
                        started_at,
                        elapsed_ms,
                    } => {
                        tracing::info!(
                            attempt_id,
                            auth_elapsed_ms = elapsed_ms,
                            daemon_total_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                            "recording failed authentication attempt"
                        );
                        auth_state.finish_failure(Instant::now())
                    }
                }
            }
            result = accept_control_connection(&mut control_listener) => {
                let mut stream = result?;
                if let Some(message) = ipc::read_daemon_control_message(&mut stream).await? {
                    let (response, stop_requested) = match message {
                        DaemonControlMessage::LockNow => {
                            if !state.is_active() {
                                if let Err(error) = try_activate_and_install(
                                    &session_proxy,
                                    &mut state,
                                    take_standby(&mut standby_curtain, &mut standby_control_socket),
                                    options.config_path.as_deref(),
                                    ActiveRuntime::new(
                                        &mut curtain,
                                        &mut auth_listener,
                                        &mut auth_socket_path,
                                        &mut control_socket_path,
                                        &mut auth_results,
                                        &mut auth_sender,
                                    ),
                                    auth_policy,
                                    &mut auth_state,
                                ).await {
                                    tracing::error!("failed to activate forwarded lock request: {error:#}");
                                } else {
                                    spawn_standby(
                                        options.config_path.as_deref(),
                                        &mut standby_curtain,
                                        &mut standby_control_socket,
                                    ).await;
                                }
                            } else {
                                tracing::debug!(state = %state, "ignoring forwarded lock request while already active");
                            }

                            (DaemonControlResponse::Accepted, false)
                        }
                        DaemonControlMessage::Stop => {
                            tracing::info!("received daemon stop request over control socket");
                            (DaemonControlResponse::Accepted, true)
                        }
                        DaemonControlMessage::Status => {
                            (DaemonControlResponse::Status(build_daemon_status(
                                &state,
                                &session_path,
                                curtain.is_some(),
                                control_socket_path.as_deref(),
                                loaded_config.path.as_deref(),
                            )), false)
                        }
                        DaemonControlMessage::Health => {
                            (DaemonControlResponse::Health(build_daemon_health()), false)
                        }
                        DaemonControlMessage::ReloadConfig => (
                            reload_config_response(
                                &options,
                                &state,
                                control_socket_path.as_deref(),
                                &mut loaded_config,
                                &mut auth_policy,
                                &mut auth_state,
                            ).await,
                            false,
                        ),
                    };

                    if let Err(error) = ipc::write_daemon_control_response(&mut stream, &response)
                        .await
                    {
                        tracing::warn!("failed to acknowledge daemon control request: {error:#}");
                    }

                    if stop_requested {
                        break;
                    }
                }
            }
            _ = sigint.recv() => {
                tracing::info!("received SIGINT");
                break;
            }
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM");
                break;
            }
        }
    }

    if let Err(error) = deactivate_lock(
        &session_proxy,
        &mut state,
        ActiveRuntime::new(
            &mut curtain,
            &mut auth_listener,
            &mut auth_socket_path,
            &mut control_socket_path,
            &mut auth_results,
            &mut auth_sender,
        ),
        auth_policy,
        &mut auth_state,
        None,
    )
    .await
    {
        tracing::warn!("failed to stop curtain during shutdown: {error:#}");
    }

    if let Some(child) = standby_curtain.take() {
        if let Err(error) = process::force_stop_curtain(child).await {
            tracing::warn!("failed to stop standby curtain during shutdown: {error:#}");
        }
        if let Some(path) = standby_control_socket.take() {
            let _ = std::fs::remove_file(path);
        }
    }

    let _ = std::fs::remove_file(&daemon_control_socket_path);
    tracing::info!("veilad exiting");
    Ok(())
}

fn take_standby(
    curtain: &mut Option<tokio::process::Child>,
    socket: &mut Option<PathBuf>,
) -> Option<(tokio::process::Child, PathBuf)> {
    match (curtain.take(), socket.take()) {
        (Some(child), Some(path)) => Some((child, path)),
        (child, path) => {
            // Restore if only one side was present (shouldn't happen in practice).
            *curtain = child;
            *socket = path;
            None
        }
    }
}

async fn spawn_standby(
    config_path: Option<&std::path::Path>,
    standby_curtain: &mut Option<tokio::process::Child>,
    standby_control_socket: &mut Option<PathBuf>,
) {
    if standby_curtain.is_some() {
        return;
    }

    let socket_path = process::control_socket_path();
    match process::spawn_standby_curtain(&socket_path, config_path).await {
        Ok(child) => {
            *standby_curtain = Some(child);
            *standby_control_socket = Some(socket_path);
            tracing::info!("standby curtain spawned");
        }
        Err(error) => {
            tracing::warn!("failed to spawn standby curtain: {error:#}");
        }
    }
}
