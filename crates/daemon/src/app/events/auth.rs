use std::time::Instant;

use super::super::{
    connections::AuthConnection,
    runtime::{AuthResult, ClientMessageContext, handle_client_message},
};

pub(crate) async fn handle_auth_message(
    context: ClientMessageContext<'_, '_>,
    connection: AuthConnection,
) {
    let AuthConnection {
        generation: _,
        stream,
        message,
    } = connection;
    if let Err(error) = handle_client_message(context, stream, message).await {
        tracing::warn!("failed to handle auth request: {error:#}");
    }
}

pub(crate) fn handle_auth_result(
    auth_state: &mut crate::domain::auth::AuthState,
    result: AuthResult,
) -> Option<AuthResult> {
    match result {
        AuthResult::Succeeded {
            attempt_id,
            elapsed_ms,
            ..
        } => {
            tracing::info!(
                attempt_id,
                elapsed_ms,
                "starting unlock after successful authentication"
            );
            auth_state.finish_success();
            Some(result)
        }
        AuthResult::Rejected {
            attempt_id,
            started_at,
            elapsed_ms,
        } => {
            tracing::info!(
                attempt_id,
                auth_elapsed_ms = elapsed_ms,
                daemon_total_ms = veila_common::time::elapsed_ms(started_at),
                "recording failed authentication attempt"
            );
            auth_state.finish_failure(Instant::now());
            None
        }
    }
}
