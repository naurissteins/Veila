use std::time::Instant;

use anyhow::Result;
use tokio::net::UnixStream;
use veila_common::ipc::LatencyReportMode;

use crate::{
    adapters::{ipc, logind},
    domain::auth::{AuthPolicy, AuthState},
};

use super::super::{
    runtime::{
        ActiveRuntime, AuthResult, ClientMessageContext, deactivate_lock, handle_client_message,
    },
    state::RuntimeSlots,
    suspend::LockedSuspendState,
};

pub(crate) async fn handle_auth_connection(
    username: &str,
    auth_sender: &Option<tokio::sync::mpsc::UnboundedSender<AuthResult>>,
    auth_state: &mut AuthState,
    suspend_state: &mut LockedSuspendState,
    manager_proxy: &logind::ManagerProxy<'_>,
    latency_report: LatencyReportMode,
    mut stream: UnixStream,
) -> Result<()> {
    if let Some(message) = ipc::read_client_message(&mut stream).await?
        && let Err(error) = handle_client_message(
            ClientMessageContext {
                username,
                auth_state,
                auth_sender,
                suspend_state,
                manager_proxy,
                latency_report,
            },
            stream,
            message,
        )
        .await
    {
        tracing::warn!("failed to handle auth request: {error:#}");
    }

    Ok(())
}

pub(crate) async fn handle_auth_result(
    session_proxy: &logind::SessionProxy<'_>,
    slots: RuntimeSlots<'_>,
    auth_policy: AuthPolicy,
    result: AuthResult,
    suspend_state: &mut LockedSuspendState,
) {
    let RuntimeSlots {
        state,
        curtain,
        auth_listener,
        auth_socket_path,
        control_socket_path,
        auth_results,
        auth_sender,
        auth_state,
        active_latency_report: _,
    } = slots;

    match result {
        AuthResult::Succeeded {
            attempt_id,
            started_at,
            elapsed_ms,
        } => {
            tracing::info!(
                attempt_id,
                elapsed_ms,
                "starting unlock after successful authentication"
            );
            auth_state.finish_success();
            let unlock_started_at = Instant::now();

            if let Err(error) = deactivate_lock(
                session_proxy,
                state,
                ActiveRuntime::new(
                    curtain,
                    auth_listener,
                    auth_socket_path,
                    control_socket_path,
                    auth_results,
                    auth_sender,
                ),
                auth_policy,
                auth_state,
                Some(attempt_id),
            )
            .await
            {
                tracing::error!("failed to unlock after successful authentication: {error:#}");
            } else {
                suspend_state.clear();
                tracing::info!(
                    attempt_id,
                    auth_elapsed_ms = elapsed_ms,
                    unlock_elapsed_ms = unlock_started_at
                        .elapsed()
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64,
                    daemon_total_ms =
                        started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                    "unlock timing summary"
                );
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
            auth_state.finish_failure(Instant::now());
        }
    }
}
