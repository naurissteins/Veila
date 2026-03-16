use std::{future::pending, path::PathBuf, process::ExitStatus, time::Instant};

use anyhow::{Context, Result, anyhow};
use futures_util::StreamExt;
use kwylock_common::ipc::{ClientMessage, DaemonMessage};
use nix::unistd::{Uid, User};
use tokio::{
    net::{UnixListener, UnixStream},
    process::Child,
    signal::unix::{SignalKind, signal},
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    time::{Duration, timeout},
};

use crate::{
    adapters::{ipc, logind, pam, process},
    domain::{
        auth::{AuthAdmission, AuthState},
        lock_state::LockState,
    },
};

struct LockActivation {
    curtain: Child,
    auth_listener: UnixListener,
    auth_socket_path: PathBuf,
    auth_results: UnboundedReceiver<AuthResult>,
    auth_sender: UnboundedSender<AuthResult>,
}

struct ActiveRuntime<'a> {
    curtain: &'a mut Option<Child>,
    auth_listener: &'a mut Option<UnixListener>,
    auth_socket_path: &'a mut Option<PathBuf>,
    auth_results: &'a mut Option<UnboundedReceiver<AuthResult>>,
    auth_sender: &'a mut Option<UnboundedSender<AuthResult>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthResult {
    Succeeded,
    Rejected,
}

pub async fn run() -> Result<()> {
    let connection = logind::connect_system().await?;
    let session_path = logind::get_session_path(&connection).await?;
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
    let mut curtain: Option<Child> = None;
    let mut auth_listener: Option<UnixListener> = None;
    let mut auth_socket_path: Option<PathBuf> = None;
    let mut auth_results: Option<UnboundedReceiver<AuthResult>> = None;
    let mut auth_sender: Option<UnboundedSender<AuthResult>> = None;
    let mut auth_state = AuthState::default();

    tracing::info!(session = %session_path, "kwylockd ready");

    loop {
        tokio::select! {
            Some(_) = lock_stream.next() => {
                if state.is_active() {
                    tracing::debug!(state = %state, "ignoring duplicate lock signal");
                    continue;
                }

                match activate_lock(&session_proxy, &mut state).await {
                    Ok(activation) => {
                        install_activation(
                            activation,
                            &mut curtain,
                            &mut auth_listener,
                            &mut auth_socket_path,
                            &mut auth_results,
                            &mut auth_sender,
                        );
                        auth_state = AuthState::default();
                    }
                    Err(error) => tracing::error!("failed to activate lock: {error:#}"),
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
                    ActiveRuntime {
                        curtain: &mut curtain,
                        auth_listener: &mut auth_listener,
                        auth_socket_path: &mut auth_socket_path,
                        auth_results: &mut auth_results,
                        auth_sender: &mut auth_sender,
                    },
                    &mut auth_state,
                ).await {
                    tracing::error!("failed to deactivate lock: {error:#}");
                }
            }
            result = wait_for_curtain_exit(&mut curtain), if curtain.is_some() => {
                let status = result?;
                tracing::warn!(?status, state = %state, "curtain exited");
                curtain.take();
                reset_auth_runtime(
                    &mut auth_listener,
                    &mut auth_socket_path,
                    &mut auth_results,
                    &mut auth_sender,
                    &mut auth_state,
                );

                if state.is_active() {
                    update_locked_hint(&session_proxy, false).await;
                    state = LockState::Unlocked;
                    tracing::error!("curtain exited while the session should be locked; attempting restart");

                    match activate_lock(&session_proxy, &mut state).await {
                        Ok(activation) => install_activation(
                            activation,
                            &mut curtain,
                            &mut auth_listener,
                            &mut auth_socket_path,
                            &mut auth_results,
                            &mut auth_sender,
                        ),
                        Err(error) => tracing::error!("failed to restart curtain after unexpected exit: {error:#}"),
                    }
                }
            }
            result = accept_auth_connection(&mut auth_listener), if matches!(state, LockState::Locked) && auth_listener.is_some() => {
                let mut stream = result?;
                if let Some(message) = ipc::read_client_message(&mut stream).await?
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
                            ActiveRuntime {
                                curtain: &mut curtain,
                                auth_listener: &mut auth_listener,
                                auth_socket_path: &mut auth_socket_path,
                                auth_results: &mut auth_results,
                                auth_sender: &mut auth_sender,
                            },
                            &mut auth_state,
                        ).await {
                            tracing::error!("failed to unlock after successful authentication: {error:#}");
                        }
                    }
                    AuthResult::Rejected => auth_state.finish_failure(Instant::now()),
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
        ActiveRuntime {
            curtain: &mut curtain,
            auth_listener: &mut auth_listener,
            auth_socket_path: &mut auth_socket_path,
            auth_results: &mut auth_results,
            auth_sender: &mut auth_sender,
        },
        &mut auth_state,
    )
    .await
    {
        tracing::warn!("failed to stop curtain during shutdown: {error:#}");
    }

    tracing::info!("kwylockd exiting");
    Ok(())
}

async fn activate_lock(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
) -> Result<LockActivation> {
    *state = LockState::Locking;

    let notify_path = process::notify_socket_path();
    let auth_socket_path = ipc::auth_socket_path();
    let notify_listener = ipc::bind_listener(&notify_path).await?;
    let auth_listener = ipc::bind_listener(&auth_socket_path).await?;
    let child = process::spawn_curtain(&notify_path, &auth_socket_path).await?;
    let (auth_sender, auth_results) = unbounded_channel();

    let ready_result = timeout(Duration::from_secs(5), notify_listener.accept()).await;
    let _ = std::fs::remove_file(&notify_path);

    match ready_result {
        Ok(Ok((_stream, _addr))) => {
            *state = LockState::Locked;
            update_locked_hint(session_proxy, true).await;
            tracing::info!("curtain ready; session considered locked");
            Ok(LockActivation {
                curtain: child,
                auth_listener,
                auth_socket_path,
                auth_results,
                auth_sender,
            })
        }
        Ok(Err(error)) => {
            *state = LockState::Unlocked;
            let _ = std::fs::remove_file(&auth_socket_path);
            process::stop_curtain(child).await?;
            update_locked_hint(session_proxy, false).await;
            Err(error).context("failed while waiting for curtain readiness")
        }
        Err(_) => {
            *state = LockState::Unlocked;
            let _ = std::fs::remove_file(&auth_socket_path);
            process::stop_curtain(child).await?;
            update_locked_hint(session_proxy, false).await;
            Err(anyhow!("timed out waiting for curtain readiness"))
        }
    }
}

async fn deactivate_lock(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    runtime: ActiveRuntime<'_>,
    auth_state: &mut AuthState,
) -> Result<()> {
    if runtime.curtain.is_none() {
        *state = LockState::Unlocked;
        reset_auth_runtime(
            runtime.auth_listener,
            runtime.auth_socket_path,
            runtime.auth_results,
            runtime.auth_sender,
            auth_state,
        );
        update_locked_hint(session_proxy, false).await;
        return Ok(());
    }

    *state = LockState::Unlocking;

    if let Some(child) = runtime.curtain.take() {
        process::stop_curtain(child).await?;
    }

    reset_auth_runtime(
        runtime.auth_listener,
        runtime.auth_socket_path,
        runtime.auth_results,
        runtime.auth_sender,
        auth_state,
    );
    *state = LockState::Unlocked;
    update_locked_hint(session_proxy, false).await;

    tracing::info!("curtain stopped; session considered unlocked");
    Ok(())
}

async fn handle_client_message(
    username: &str,
    auth_state: &mut AuthState,
    auth_sender: &Option<UnboundedSender<AuthResult>>,
    mut stream: UnixStream,
    message: ClientMessage,
) -> Result<()> {
    match message {
        ClientMessage::SubmitPassword { secret } => match auth_state.admit(Instant::now()) {
            AuthAdmission::Allowed => {
                let Some(sender) = auth_sender.clone() else {
                    return Err(anyhow!("authentication channel is unavailable"));
                };

                auth_state.start_attempt();
                tokio::spawn(run_auth_attempt(
                    username.to_string(),
                    secret,
                    stream,
                    sender,
                ));
            }
            AuthAdmission::Busy => {
                ipc::write_daemon_message(&mut stream, &DaemonMessage::AuthenticationBusy).await?;
            }
            AuthAdmission::RateLimited(delay) => {
                let retry_after_ms = delay.as_millis().min(u128::from(u64::MAX)) as u64;
                ipc::write_daemon_message(
                    &mut stream,
                    &DaemonMessage::AuthenticationRejected {
                        retry_after_ms: Some(retry_after_ms),
                    },
                )
                .await?;
            }
        },
        ClientMessage::CancelAuthentication => {}
    }

    Ok(())
}

async fn run_auth_attempt(
    username: String,
    secret: String,
    mut stream: UnixStream,
    sender: UnboundedSender<AuthResult>,
) {
    let result = tokio::task::spawn_blocking(move || pam::authenticate(&username, &secret)).await;

    match result {
        Ok(Ok(())) => {
            let _ = sender.send(AuthResult::Succeeded);
        }
        Ok(Err(error)) => {
            tracing::info!("authentication rejected: {error}");
            if let Err(write_error) = ipc::write_daemon_message(
                &mut stream,
                &DaemonMessage::AuthenticationRejected {
                    retry_after_ms: None,
                },
            )
            .await
            {
                tracing::warn!("failed to report auth rejection: {write_error:#}");
            }
            let _ = sender.send(AuthResult::Rejected);
        }
        Err(error) => {
            tracing::error!("authentication worker failed: {error}");
            if let Err(write_error) = ipc::write_daemon_message(
                &mut stream,
                &DaemonMessage::AuthenticationRejected {
                    retry_after_ms: None,
                },
            )
            .await
            {
                tracing::warn!("failed to report worker failure to client: {write_error:#}");
            }
            let _ = sender.send(AuthResult::Rejected);
        }
    }
}

fn install_activation(
    activation: LockActivation,
    curtain: &mut Option<Child>,
    auth_listener: &mut Option<UnixListener>,
    auth_socket_path: &mut Option<PathBuf>,
    auth_results: &mut Option<UnboundedReceiver<AuthResult>>,
    auth_sender: &mut Option<UnboundedSender<AuthResult>>,
) {
    *curtain = Some(activation.curtain);
    *auth_listener = Some(activation.auth_listener);
    *auth_socket_path = Some(activation.auth_socket_path);
    *auth_results = Some(activation.auth_results);
    *auth_sender = Some(activation.auth_sender);
}

fn reset_auth_runtime(
    auth_listener: &mut Option<UnixListener>,
    auth_socket_path: &mut Option<PathBuf>,
    auth_results: &mut Option<UnboundedReceiver<AuthResult>>,
    auth_sender: &mut Option<UnboundedSender<AuthResult>>,
    auth_state: &mut AuthState,
) {
    auth_listener.take();
    auth_results.take();
    auth_sender.take();
    if let Some(path) = auth_socket_path.take() {
        let _ = std::fs::remove_file(path);
    }
    *auth_state = AuthState::default();
}

async fn wait_for_curtain_exit(curtain: &mut Option<Child>) -> Result<ExitStatus> {
    match curtain.as_mut() {
        Some(child) => child
            .wait()
            .await
            .context("failed while waiting for curtain process"),
        None => pending().await,
    }
}

async fn accept_auth_connection(auth_listener: &mut Option<UnixListener>) -> Result<UnixStream> {
    match auth_listener.as_mut() {
        Some(listener) => listener
            .accept()
            .await
            .map(|(stream, _)| stream)
            .context("failed to accept auth connection"),
        None => pending().await,
    }
}

async fn receive_auth_result(
    auth_results: &mut Option<UnboundedReceiver<AuthResult>>,
) -> Option<AuthResult> {
    match auth_results.as_mut() {
        Some(receiver) => receiver.recv().await,
        None => pending().await,
    }
}

fn current_username() -> Result<String> {
    let uid = Uid::current();
    let Some(user) = User::from_uid(uid).context("failed to resolve current username")? else {
        return Err(anyhow!("current uid {uid} does not resolve to a user"));
    };

    Ok(user.name)
}

async fn update_locked_hint(session_proxy: &logind::SessionProxy<'_>, locked: bool) {
    if let Err(error) = session_proxy.set_locked_hint(locked).await {
        tracing::warn!(locked, "failed to update logind LockedHint: {error}");
    }
}
