use crate::{adapters::logind, domain::auth::AuthPolicy};

use super::super::{
    runtime::{ActiveRuntime, deactivate_lock},
    state::RuntimeSlots,
};

pub(crate) async fn shutdown_runtime(
    session_proxy: &logind::SessionProxy<'_>,
    slots: RuntimeSlots<'_>,
    auth_policy: AuthPolicy,
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
    } = slots;

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
        None,
    )
    .await
    {
        tracing::warn!("failed to stop curtain during shutdown: {error:#}");
    }
}
