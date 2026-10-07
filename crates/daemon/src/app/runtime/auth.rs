use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use tokio::{net::UnixStream, sync::mpsc::UnboundedSender};
use veila_common::{
    PowerAction, Secret,
    config::VisualConfig,
    duration_ms, duration_us, elapsed_ms, elapsed_us,
    ipc::{ClientMessage, DaemonMessage, LatencyReportMode},
};

use crate::{
    adapters::{ipc, logind, pam},
    app::suspend::LockedSuspendState,
    domain::auth::{AuthAdmission, AuthState},
};

const AUTH_RESPONSE_WRITE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

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
    pub(crate) visuals: &'a VisualConfig,
    pub(crate) auth_state: &'a mut AuthState,
    pub(crate) auth_sender: Option<&'a UnboundedSender<AuthResult>>,
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
        visuals,
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
            validate_power_action(visuals, action)?;
            suspend_state.note_activity(Instant::now());
            request_power_action(manager_proxy, action).await?;
        }
        ClientMessage::SubmitPassword { attempt_id, secret } => {
            suspend_state.note_activity(Instant::now());
            let started_at = Instant::now();
            tracing::info!(attempt_id, "received password submission");
            match auth_state.admit(Instant::now()) {
                AuthAdmission::Allowed => {
                    let Some(sender) = auth_sender.cloned() else {
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
                    let retry_after_ms = duration_ms(delay);
                    ipc::write_daemon_message(
                        &mut stream,
                        &DaemonMessage::AuthenticationRejected {
                            attempt_id,
                            retry_after_ms: Some(retry_after_ms),
                            failed_attempts: Some(auth_state.failed_attempts()),
                            message: None,
                        },
                    )
                    .await?;
                }
            }
        }
        ClientMessage::AuthenticationResponse { .. }
        | ClientMessage::CancelAuthentication { .. } => {
            return Err(anyhow!("unexpected authentication follow-up"));
        }
    }

    Ok(())
}

fn validate_power_action(visuals: &VisualConfig, action: PowerAction) -> Result<()> {
    // daemon applies the current config independently of curtain UI visibility
    if !visuals.power_button_enabled(action) {
        return Err(anyhow!("power action {action:?} is disabled"));
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
    secret: Secret,
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
    let worker_start_delay_ms = duration_ms(auth_started_at.saturating_duration_since(started_at));
    let worker_start_delay_us = duration_us(auth_started_at.saturating_duration_since(started_at));
    let result = pam::authenticate(&username, secret, attempt_id, &mut stream).await;
    let elapsed_ms = elapsed_ms(auth_started_at);
    let elapsed_us = elapsed_us(auth_started_at);

    match result {
        Ok(pam::PamReply { accepted: true, .. }) => {
            log_auth_latency_report(
                latency_report,
                attempt_id,
                worker_start_delay_ms,
                worker_start_delay_us,
                elapsed_ms,
                elapsed_us,
            );
            tracing::info!(attempt_id, elapsed_ms, "authentication accepted");
            if let Err(write_error) = report_auth_result(
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
        Ok(pam::PamReply {
            accepted: false,
            message,
        }) => {
            log_auth_latency_report(
                latency_report,
                attempt_id,
                worker_start_delay_ms,
                worker_start_delay_us,
                elapsed_ms,
                elapsed_us,
            );
            tracing::info!(attempt_id, elapsed_ms, "authentication rejected");
            if let Err(write_error) = report_auth_result(
                &mut stream,
                &DaemonMessage::AuthenticationRejected {
                    attempt_id,
                    retry_after_ms: None,
                    failed_attempts: Some(failed_attempts),
                    message,
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
            if let Err(write_error) = report_auth_result(
                &mut stream,
                &DaemonMessage::AuthenticationRejected {
                    attempt_id,
                    retry_after_ms: None,
                    failed_attempts: Some(failed_attempts),
                    message: None,
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

async fn report_auth_result(stream: &mut UnixStream, message: &DaemonMessage) -> Result<()> {
    tokio::time::timeout(
        AUTH_RESPONSE_WRITE_TIMEOUT,
        ipc::write_daemon_message(stream, message),
    )
    .await
    .context("timed out reporting authentication result")?
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

#[cfg(test)]
mod tests {
    use veila_common::{AppConfig, PowerAction};

    use super::validate_power_action;

    #[test]
    fn rejects_disabled_power_actions() {
        let config = AppConfig::default();
        for action in [
            PowerAction::Suspend,
            PowerAction::Reboot,
            PowerAction::Poweroff,
        ] {
            assert!(validate_power_action(&config.visuals, action).is_err());
        }
    }

    #[test]
    fn accepts_only_enabled_power_action() {
        let config = AppConfig::from_toml_str(
            "[visuals.power.suspend]\nenabled = true\n[visuals.power.reboot]\nenabled = false\n",
        )
        .expect("power config should parse");

        assert!(validate_power_action(&config.visuals, PowerAction::Suspend).is_ok());
        assert!(validate_power_action(&config.visuals, PowerAction::Reboot).is_err());
        assert!(validate_power_action(&config.visuals, PowerAction::Poweroff).is_err());
    }
}
