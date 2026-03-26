use std::time::Instant;

use anyhow::{Result, anyhow};
use tokio::{net::UnixStream, sync::mpsc::UnboundedSender};
use veila_common::ipc::{ClientMessage, DaemonMessage};

use crate::{
    adapters::{ipc, pam},
    domain::auth::{AuthAdmission, AuthState},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthResult {
    Succeeded {
        attempt_id: u64,
        started_at: Instant,
        elapsed_ms: u64,
    },
    Rejected {
        attempt_id: u64,
        started_at: Instant,
        elapsed_ms: u64,
    },
}

pub(crate) async fn handle_client_message(
    username: &str,
    auth_state: &mut AuthState,
    auth_sender: &Option<UnboundedSender<AuthResult>>,
    mut stream: UnixStream,
    message: ClientMessage,
) -> Result<()> {
    match message {
        ClientMessage::SubmitPassword { attempt_id, secret } => {
            let started_at = Instant::now();
            tracing::info!(
                attempt_id,
                secret_len = secret.chars().count(),
                "received password submission"
            );
            match auth_state.admit(Instant::now()) {
                AuthAdmission::Allowed => {
                    let Some(sender) = auth_sender.clone() else {
                        return Err(anyhow!("authentication channel is unavailable"));
                    };

                    auth_state.start_attempt();
                    tokio::spawn(run_auth_attempt(
                        attempt_id,
                        started_at,
                        username.to_string(),
                        secret,
                        stream,
                        sender,
                    ));
                }
                AuthAdmission::Busy => {
                    ipc::write_daemon_message(
                        &mut stream,
                        &DaemonMessage::AuthenticationBusy { attempt_id },
                    )
                    .await?;
                }
                AuthAdmission::RateLimited(delay) => {
                    let retry_after_ms = delay.as_millis().min(u128::from(u64::MAX)) as u64;
                    ipc::write_daemon_message(
                        &mut stream,
                        &DaemonMessage::AuthenticationRejected {
                            attempt_id,
                            retry_after_ms: Some(retry_after_ms),
                        },
                    )
                    .await?;
                }
            }
        }
        ClientMessage::CancelAuthentication => {}
    }

    Ok(())
}

async fn run_auth_attempt(
    attempt_id: u64,
    started_at: Instant,
    username: String,
    secret: String,
    mut stream: UnixStream,
    sender: UnboundedSender<AuthResult>,
) {
    let auth_started_at = Instant::now();
    let result = tokio::task::spawn_blocking(move || pam::authenticate(&username, &secret)).await;
    let elapsed_ms = auth_started_at
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;

    match result {
        Ok(Ok(())) => {
            tracing::info!(attempt_id, elapsed_ms, "authentication accepted");
            if let Err(write_error) = ipc::write_daemon_message(
                &mut stream,
                &DaemonMessage::AuthenticationAccepted { attempt_id },
            )
            .await
            {
                tracing::warn!("failed to report auth success: {write_error:#}");
            }
            let _ = sender.send(AuthResult::Succeeded {
                attempt_id,
                started_at,
                elapsed_ms,
            });
        }
        Ok(Err(error)) => {
            tracing::info!(attempt_id, elapsed_ms, "authentication rejected: {error}");
            if let Err(write_error) = ipc::write_daemon_message(
                &mut stream,
                &DaemonMessage::AuthenticationRejected {
                    attempt_id,
                    retry_after_ms: None,
                },
            )
            .await
            {
                tracing::warn!("failed to report auth rejection: {write_error:#}");
            }
            let _ = sender.send(AuthResult::Rejected {
                attempt_id,
                started_at,
                elapsed_ms,
            });
        }
        Err(error) => {
            tracing::error!(
                attempt_id,
                elapsed_ms,
                "authentication worker failed: {error}"
            );
            if let Err(write_error) = ipc::write_daemon_message(
                &mut stream,
                &DaemonMessage::AuthenticationRejected {
                    attempt_id,
                    retry_after_ms: None,
                },
            )
            .await
            {
                tracing::warn!("failed to report worker failure to client: {write_error:#}");
            }
            let _ = sender.send(AuthResult::Rejected {
                attempt_id,
                started_at,
                elapsed_ms,
            });
        }
    }
}
