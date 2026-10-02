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
    helpers::activate_and_log, runtime::reset_runtime, state::RuntimeSlots,
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
        active,
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
        active,
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
        active,
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
        active,
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
        active,
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
    let owner_path = active
        .as_ref()
        .map(|active| active.curtain.owner_path().to_path_buf());
    reset_runtime(active, auth_policy, auth_state);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::runtime::Fixture;
    use crate::domain::auth::AuthState;

    #[tokio::test]
    async fn active_curtain_exit_clears_resources_without_authorizing_unlock() {
        for initial in [LockState::Locking, LockState::Locked, LockState::Unlocking] {
            let mut fixture = Fixture::new();
            let sender = fixture.active.as_ref().expect("active").auth_sender.clone();
            let mut state = initial;
            let mut auth_state = AuthState::default();
            let mut latency = LatencyReportMode::Disabled;
            handle_curtain_exit(
                CurtainExit::Adopted,
                RuntimeSlots {
                    state: &mut state,
                    active: &mut fixture.active,
                    auth_state: &mut auth_state,
                    active_latency_report: &mut latency,
                },
                AuthPolicy::default(),
            )
            .await;
            assert_eq!(state, initial);
            assert!(fixture.active.is_none());
            assert!(sender.is_closed());
            assert!(fixture.root.join("owner.json").exists());
        }
    }

    #[tokio::test]
    async fn released_curtain_exit_removes_ownership_and_resources() {
        let mut fixture = Fixture::new();
        let mut state = LockState::Unlocked;
        let mut auth_state = AuthState::default();
        let mut latency = LatencyReportMode::Disabled;
        handle_curtain_exit(
            CurtainExit::Adopted,
            RuntimeSlots {
                state: &mut state,
                active: &mut fixture.active,
                auth_state: &mut auth_state,
                active_latency_report: &mut latency,
            },
            AuthPolicy::default(),
        )
        .await;
        assert_eq!(state, LockState::Unlocked);
        assert!(fixture.active.is_none());
        assert!(!fixture.root.join("owner.json").exists());
        assert!(!fixture.root.join("auth.sock").exists());
    }
}
