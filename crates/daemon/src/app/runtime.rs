use std::{future::pending, path::PathBuf, process::ExitStatus, time::Instant};

use anyhow::{Context, Result, anyhow};
use kwylock_common::ipc::{ClientMessage, DaemonMessage};
use tokio::{
    net::{UnixListener, UnixStream},
    process::Child,
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    time::{Duration, timeout},
};

use crate::{
    adapters::{ipc, logind, pam, process},
    domain::{
        auth::{AuthAdmission, AuthPolicy, AuthState},
        lock_state::LockState,
    },
};

use super::update_locked_hint;

pub(super) struct LockActivation {
    curtain: Child,
    auth_listener: UnixListener,
    auth_socket_path: PathBuf,
    auth_results: UnboundedReceiver<AuthResult>,
    auth_sender: UnboundedSender<AuthResult>,
}

pub(super) struct ActiveRuntime<'a> {
    curtain: &'a mut Option<Child>,
    auth_listener: &'a mut Option<UnixListener>,
    auth_socket_path: &'a mut Option<PathBuf>,
    auth_results: &'a mut Option<UnboundedReceiver<AuthResult>>,
    auth_sender: &'a mut Option<UnboundedSender<AuthResult>>,
}

impl<'a> ActiveRuntime<'a> {
    pub(super) fn new(
        curtain: &'a mut Option<Child>,
        auth_listener: &'a mut Option<UnixListener>,
        auth_socket_path: &'a mut Option<PathBuf>,
        auth_results: &'a mut Option<UnboundedReceiver<AuthResult>>,
        auth_sender: &'a mut Option<UnboundedSender<AuthResult>>,
    ) -> Self {
        Self {
            curtain,
            auth_listener,
            auth_socket_path,
            auth_results,
            auth_sender,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AuthResult {
    Succeeded,
    Rejected,
}

pub(super) async fn activate_lock(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    config_path: Option<&std::path::Path>,
) -> Result<LockActivation> {
    *state = LockState::Locking;

    let notify_path = process::notify_socket_path();
    let auth_socket_path = ipc::auth_socket_path();
    let notify_listener = ipc::bind_listener(&notify_path).await?;
    let auth_listener = ipc::bind_listener(&auth_socket_path).await?;
    let child = process::spawn_curtain(&notify_path, &auth_socket_path, config_path).await?;
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

pub(super) async fn deactivate_lock(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    runtime: ActiveRuntime<'_>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
) -> Result<()> {
    if runtime.curtain.is_none() {
        *state = LockState::Unlocked;
        reset_auth_runtime(
            runtime.auth_listener,
            runtime.auth_socket_path,
            runtime.auth_results,
            runtime.auth_sender,
            auth_policy,
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
        auth_policy,
        auth_state,
    );
    *state = LockState::Unlocked;
    update_locked_hint(session_proxy, false).await;

    tracing::info!("curtain stopped; session considered unlocked");
    Ok(())
}

pub(super) async fn handle_client_message(
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

pub(super) fn install_activation(
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

pub(super) fn reset_auth_runtime(
    auth_listener: &mut Option<UnixListener>,
    auth_socket_path: &mut Option<PathBuf>,
    auth_results: &mut Option<UnboundedReceiver<AuthResult>>,
    auth_sender: &mut Option<UnboundedSender<AuthResult>>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
) {
    auth_listener.take();
    auth_results.take();
    auth_sender.take();
    if let Some(path) = auth_socket_path.take() {
        let _ = std::fs::remove_file(path);
    }
    *auth_state = AuthState::new(auth_policy);
}

pub(super) async fn wait_for_curtain_exit(curtain: &mut Option<Child>) -> Result<ExitStatus> {
    match curtain.as_mut() {
        Some(child) => child
            .wait()
            .await
            .context("failed while waiting for curtain process"),
        None => pending().await,
    }
}

pub(super) async fn accept_auth_connection(
    auth_listener: &mut Option<UnixListener>,
) -> Result<UnixStream> {
    match auth_listener.as_mut() {
        Some(listener) => listener
            .accept()
            .await
            .map(|(stream, _)| stream)
            .context("failed to accept auth connection"),
        None => pending().await,
    }
}

pub(super) async fn receive_auth_result(
    auth_results: &mut Option<UnboundedReceiver<AuthResult>>,
) -> Option<AuthResult> {
    match auth_results.as_mut() {
        Some(receiver) => receiver.recv().await,
        None => pending().await,
    }
}
