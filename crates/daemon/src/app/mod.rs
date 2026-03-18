mod runtime;

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use futures_util::StreamExt;
use kwylock_common::AppConfig;
use kwylock_common::ipc::{DaemonControlMessage, DaemonControlResponse};
use nix::unistd::{Uid, User};
use tokio::{
    net::UnixListener,
    signal::unix::{SignalKind, signal},
};

use crate::{
    DaemonOptions,
    adapters::{ipc, logind},
    domain::{
        auth::{AuthPolicy, AuthState},
        lock_state::LockState,
    },
};

use self::runtime::{
    ActiveRuntime, AuthResult, accept_auth_connection, accept_control_connection, activate_lock,
    deactivate_lock, handle_client_message, receive_auth_result, reset_runtime,
    wait_for_curtain_exit,
};

pub async fn run(
    options: DaemonOptions,
    mut control_listener: UnixListener,
    daemon_control_socket_path: PathBuf,
) -> Result<()> {
    let loaded_config =
        AppConfig::load(options.config_path.as_deref()).context("failed to load daemon config")?;
    let auth_policy = AuthPolicy::new(
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

    tracing::info!(
        session = %session_path,
        session_id_override = options.session_id.as_deref().unwrap_or("none"),
        manual_lock = options.lock_now,
        config = loaded_config.path.as_deref().map(|path| path.display().to_string()).unwrap_or_else(|| "defaults".to_string()),
        "kwylockd ready"
    );

    if options.lock_now {
        tracing::info!("manual lock requested via --lock-now");
        activate_and_install(
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
        )
        .await
        .context("failed to activate manual lock")?;
    }

    loop {
        tokio::select! {
            Some(_) = lock_stream.next() => {
                if state.is_active() {
                    tracing::debug!(state = %state, "ignoring duplicate lock signal");
                    continue;
                }

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
                    tracing::error!("failed to activate lock: {error:#}");
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
                ).await {
                    tracing::error!("failed to deactivate lock: {error:#}");
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
                    }
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
                    AuthResult::Succeeded => {
                        auth_state.finish_success();

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
                        ).await {
                            tracing::error!("failed to unlock after successful authentication: {error:#}");
                        }
                    }
                    AuthResult::Rejected => auth_state.finish_failure(Instant::now()),
                }
            }
            result = accept_control_connection(&mut control_listener) => {
                let mut stream = result?;
                if let Some(message) = ipc::read_daemon_control_message(&mut stream).await? {
                    match message {
                        DaemonControlMessage::LockNow => {
                            if !state.is_active() {
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
                                    tracing::error!("failed to activate forwarded lock request: {error:#}");
                                }
                            } else {
                                tracing::debug!(state = %state, "ignoring forwarded lock request while already active");
                            }
                        }
                    }

                    if let Err(error) = ipc::write_daemon_control_response(
                        &mut stream,
                        &DaemonControlResponse::Accepted,
                    )
                    .await {
                        tracing::warn!("failed to acknowledge daemon control request: {error:#}");
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
    )
    .await
    {
        tracing::warn!("failed to stop curtain during shutdown: {error:#}");
    }

    let _ = std::fs::remove_file(&daemon_control_socket_path);
    tracing::info!("kwylockd exiting");
    Ok(())
}

async fn activate_and_install(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    config_path: Option<&std::path::Path>,
    runtime: ActiveRuntime<'_>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
) -> Result<()> {
    let activation = activate_lock(session_proxy, state, config_path).await?;
    runtime.install_activation(activation);
    *auth_state = AuthState::new(auth_policy);
    Ok(())
}

fn current_username() -> Result<String> {
    let uid = Uid::current();
    let Some(user) = User::from_uid(uid).context("failed to resolve current username")? else {
        return Err(anyhow!("current uid {uid} does not resolve to a user"));
    };

    Ok(user.name)
}

pub(super) async fn update_locked_hint(session_proxy: &logind::SessionProxy<'_>, locked: bool) {
    if let Err(error) = session_proxy.set_locked_hint(locked).await {
        tracing::warn!(locked, "failed to update logind LockedHint: {error}");
    }
}
