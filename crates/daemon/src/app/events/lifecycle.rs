use std::{path::Path, process::ExitStatus};

use crate::{
    adapters::logind,
    domain::{auth::AuthPolicy, lock_state::LockState},
};

use super::super::{
    helpers::activate_and_log,
    runtime::{ActiveRuntime, reset_runtime, update_locked_hint},
    state::RuntimeSlots,
};

pub(crate) async fn handle_lock_signal(
    trigger: &'static str,
    session_proxy: &logind::SessionProxy<'_>,
    config_path: Option<&Path>,
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

    if state.is_active() {
        tracing::debug!(state = %state, "ignoring duplicate lock signal");
        return;
    }

    if let Err(error) = activate_and_log(
        trigger,
        session_proxy,
        state,
        config_path,
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
    )
    .await
    {
        tracing::error!("failed to activate lock: {error:#}");
    }
}

pub(crate) async fn handle_unlock_signal(
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

    if !state.is_active() {
        tracing::debug!(state = %state, "ignoring unlock signal while not locked");
        return;
    }

    if let Err(error) = super::super::runtime::deactivate_lock(
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
        tracing::error!("failed to deactivate lock: {error:#}");
    }
}

pub(crate) async fn handle_curtain_exit(
    status: ExitStatus,
    session_proxy: &logind::SessionProxy<'_>,
    config_path: Option<&Path>,
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

    tracing::warn!(?status, state = %state, "curtain exited");
    curtain.take();
    reset_runtime(
        auth_listener,
        auth_socket_path,
        control_socket_path,
        auth_results,
        auth_sender,
        auth_policy,
        auth_state,
    );

    if state.is_active() {
        update_locked_hint(session_proxy, false).await;
        *state = LockState::Unlocked;
        tracing::error!("curtain exited while the session should be locked; attempting restart");

        if let Err(error) = activate_and_log(
            "restart",
            session_proxy,
            state,
            config_path,
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
        )
        .await
        {
            tracing::error!("failed to restart curtain after unexpected exit: {error:#}");
        }
    }
}
