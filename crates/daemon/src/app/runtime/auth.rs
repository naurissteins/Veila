use std::time::Instant;

use anyhow::{Result, anyhow};
use tokio::{net::UnixStream, sync::mpsc::UnboundedSender};
use veila_common::{
    PowerAction,
    ipc::{ClientMessage, DaemonMessage, LatencyReportMode},
};

use crate::{
    adapters::{ipc, logind, pam},
    app::suspend::LockedSuspendState,
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

pub(crate) struct ClientMessageContext<'a, 'p> {
    pub(crate) username: &'a str,
    pub(crate) auth_state: &'a mut AuthState,
    pub(crate) auth_sender: &'a Option<UnboundedSender<AuthResult>>,
    pub(crate) suspend_state: &'a mut LockedSuspendState,
    pub(crate) manager_proxy: &'a logind::ManagerProxy<'p>,
    pub(crate) latency_report: LatencyReportMode,
}

pub(crate) async fn handle_client_message(
    context: ClientMessageContext<'_, '_>,
    mut stream: UnixStream,
    message: ClientMessage,
) -> Result<()> {
    let ClientMessageContext {
        username,
        auth_state,
        auth_sender,
        suspend_state,
        manager_proxy,
        latency_report,
    } = context;

    match message {
        ClientMessage::Activity => {
            suspend_state.note_activity(Instant::now());
        }
        ClientMessage::RequestPowerAction { action } => {
            suspend_state.note_activity(Instant::now());
            request_power_action(manager_proxy, action).await?;
        }
        ClientMessage::SubmitPassword { attempt_id, secret } => {
            suspend_state.note_activity(Instant::now());
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
                    let failed_attempts = auth_state.next_failed_attempts();

                    auth_state.start_attempt();
                    tokio::spawn(run_auth_attempt(AuthAttempt {
                        attempt_id,
                        started_at,
                        failed_attempts,
                        username: username.to_string(),
                        secret,
                        stream,
                        sender,
                        latency_report,
                    }));
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
                            failed_attempts: Some(auth_state.failed_attempts()),
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

async fn request_power_action(
    manager_proxy: &logind::ManagerProxy<'_>,
    action: PowerAction,
) -> Result<()> {
    tracing::info!(?action, "received daemon-mediated power action request");
    match action {
        PowerAction::Suspend => manager_proxy.suspend(false).await?,
        PowerAction::Reboot => manager_proxy.reboot(false).await?,
        PowerAction::Poweroff => manager_proxy.power_off(false).await?,
    }
    Ok(())
}

struct AuthAttempt {
    attempt_id: u64,
    started_at: Instant,
    failed_attempts: u8,
    username: String,
    secret: String,
    stream: UnixStream,
    sender: UnboundedSender<AuthResult>,
    latency_report: LatencyReportMode,
}

async fn run_auth_attempt(attempt: AuthAttempt) {
    let AuthAttempt {
        attempt_id,
        started_at,
        failed_attempts,
        username,
        secret,
        mut stream,
        sender,
        latency_report,
    } = attempt;
    let auth_started_at = Instant::now();
    let worker_start_delay_ms = auth_started_at
        .saturating_duration_since(started_at)
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let worker_start_delay_us = auth_started_at
        .saturating_duration_since(started_at)
        .as_micros()
        .min(u128::from(u64::MAX)) as u64;
    let result = tokio::task::spawn_blocking(move || pam::authenticate(&username, &secret)).await;
    let elapsed_ms = auth_started_at
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let elapsed_us = auth_started_at
        .elapsed()
        .as_micros()
        .min(u128::from(u64::MAX)) as u64;

    match result {
        Ok(Ok(())) => {
            log_auth_latency_report(
                latency_report,
                attempt_id,
                worker_start_delay_ms,
                worker_start_delay_us,
                elapsed_ms,
                elapsed_us,
            );
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
            log_auth_latency_report(
                latency_report,
                attempt_id,
                worker_start_delay_ms,
                worker_start_delay_us,
                elapsed_ms,
                elapsed_us,
            );
            tracing::info!(attempt_id, elapsed_ms, "authentication rejected: {error}");
            if let Err(write_error) = ipc::write_daemon_message(
                &mut stream,
                &DaemonMessage::AuthenticationRejected {
                    attempt_id,
                    retry_after_ms: None,
                    failed_attempts: Some(failed_attempts),
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
            log_auth_latency_report(
                latency_report,
                attempt_id,
                worker_start_delay_ms,
                worker_start_delay_us,
                elapsed_ms,
                elapsed_us,
            );
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
                    failed_attempts: Some(failed_attempts),
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

fn log_auth_latency_report(
    enabled: LatencyReportMode,
    attempt_id: u64,
    worker_start_delay_ms: u64,
    worker_start_delay_us: u64,
    pam_elapsed_ms: u64,
    pam_elapsed_us: u64,
) {
    if !enabled.is_enabled() {
        return;
    }

    tracing::info!(
        attempt_id,
        worker_start_delay_ms,
        worker_start_delay_us = enabled.is_verbose().then_some(worker_start_delay_us),
        pam_elapsed_ms,
        pam_elapsed_us = enabled.is_verbose().then_some(pam_elapsed_us),
        "auth latency report"
    );
}
