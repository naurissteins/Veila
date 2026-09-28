use std::path::Path;

use veila_common::{BatterySnapshot, NowPlayingSnapshot, WeatherSnapshot, ipc::LatencyReportMode};

use crate::{
    adapters::{
        logind, ownership,
        process::{self, CurtainExit},
    },
    domain::{auth::AuthPolicy, lock_state::LockState},
};

use super::super::{
    helpers::activate_and_log,
    runtime::{ActiveRuntime, reset_runtime},
    state::RuntimeSlots,
    suspend::LockedSuspendState,
};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_lock_signal(
    trigger: &'static str,
    session_proxy: &logind::SessionProxy<'_>,
    config_path: Option<&Path>,
    initial_background_path: Option<&Path>,
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
    acquire_timeout_seconds: u64,
    daemon_config_load_ms: u64,
    daemon_config_load_us: u64,
    slots: RuntimeSlots<'_>,
    auth_policy: AuthPolicy,
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
        active_latency_report,
    } = slots;

    if state.is_active() {
        tracing::debug!(state = %state, "ignoring duplicate lock signal");
        return;
    }

    *active_latency_report = LatencyReportMode::Disabled;
    if let Err(error) = activate_and_log(
        trigger,
        session_proxy,
        state,
        config_path,
        initial_background_path,
        weather_snapshot,
        battery_snapshot,
        now_playing_snapshot,
        false,
        LatencyReportMode::Disabled,
        acquire_timeout_seconds,
        daemon_config_load_ms,
        daemon_config_load_us,
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
        suspend_state,
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
    } else {
        suspend_state.clear();
    }
}

pub(crate) async fn handle_curtain_exit(
    status: CurtainExit,
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
        active_latency_report: _,
    } = slots;

    match status {
        CurtainExit::Spawned(exit_status) => {
            tracing::warn!(?exit_status, state = %state, "curtain exited");
        }
        CurtainExit::Adopted => {
            tracing::warn!(state = %state, "adopted curtain exited");
        }
    }
    let owner_path = curtain
        .as_ref()
        .map(|curtain| curtain.owner_path().to_path_buf());
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
        tracing::error!("curtain exited while locked; preserving unresolved lock ownership");
    } else if let Some(path) = owner_path
        && let Err(error) = ownership::remove(&path)
    {
        tracing::warn!("failed to clear released curtain ownership: {error:#}");
    }
}

pub(crate) async fn handle_now_playing_update(
    state: &LockState,
    control_socket_path: Option<&Path>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
) {
    if !state.is_active() {
        return;
    }

    let Some(control_socket_path) = control_socket_path else {
        tracing::debug!("ignoring now playing update without active curtain control socket");
        return;
    };

    if let Err(error) =
        process::request_curtain_now_playing_update(control_socket_path, now_playing_snapshot).await
    {
        tracing::warn!("failed to forward live now playing update to curtain: {error:#}");
    }
}
