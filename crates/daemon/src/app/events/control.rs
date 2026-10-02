use super::super::runtime::control_socket_path;

use anyhow::Result;
use tokio::net::UnixStream;
use veila_common::{
    BatterySnapshot, LoadedConfig, NowPlayingSnapshot, WeatherSnapshot,
    ipc::{DaemonControlMessage, DaemonControlResponse},
};

use crate::{
    DaemonOptions,
    adapters::{ipc, logind},
    domain::{auth::AuthPolicy, lock_state::LockState},
};

use super::super::{
    battery::BatteryHandle,
    fingerprint::FingerprintHandle,
    helpers::{
        activate_and_log, build_daemon_health, build_daemon_status, reload_config_response,
        select_initial_background_path,
    },
    mpris::NowPlayingHandle,
    state::BackgroundSelectionState,
    state::RuntimeSlots,
    weather::WeatherHandle,
};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_control_message(
    mut stream: UnixStream,
    message: DaemonControlMessage,
    options: &DaemonOptions,
    session_proxy: &logind::SessionProxy<'_>,
    session_path: &str,
    loaded_config: &mut LoadedConfig,
    last_reload_result: &mut Option<String>,
    last_reload_unix_ms: &mut Option<u64>,
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
    weather: &WeatherHandle,
    battery: &BatteryHandle,
    now_playing: &NowPlayingHandle,
    background_selection: &mut Option<BackgroundSelectionState>,
    suspend_state: &mut crate::app::suspend::LockedSuspendState,
    fingerprint: &mut FingerprintHandle,
    slots: RuntimeSlots<'_>,
    auth_policy: &mut AuthPolicy,
    daemon_config_load_ms: u64,
    daemon_config_load_us: u64,
) -> Result<bool> {
    let RuntimeSlots {
        state,
        active,
        auth_state,
        active_latency_report,
    } = slots;

    let (response, stop_requested) = match message {
        DaemonControlMessage::LockNow {
            wait_ready,
            force_emergency_ui,
            latency_report,
            sleep_transition,
        } => {
            *active_latency_report = latency_report;
            if sleep_transition {
                fingerprint.pause_for_sleep().await;
            }
            if !state.is_active() {
                let initial_background_path =
                    select_initial_background_path(&loaded_config.config, background_selection);
                match activate_and_log(
                    "forwarded",
                    session_proxy,
                    state,
                    options.config_path.as_deref(),
                    initial_background_path.as_deref(),
                    weather_snapshot,
                    battery_snapshot,
                    now_playing_snapshot,
                    force_emergency_ui,
                    latency_report,
                    loaded_config.config.lock.acquire_timeout_seconds,
                    daemon_config_load_ms,
                    daemon_config_load_us,
                    active,
                    *auth_policy,
                    auth_state,
                    suspend_state,
                )
                .await
                {
                    Ok(latency_report) => (
                        if wait_ready {
                            DaemonControlResponse::Locked {
                                already_active: false,
                                latency_report: latency_report.map(Box::new),
                            }
                        } else {
                            DaemonControlResponse::Accepted
                        },
                        false,
                    ),
                    Err(error) => {
                        tracing::error!("failed to activate forwarded lock request: {error:#}");
                        (
                            if wait_ready {
                                DaemonControlResponse::Error {
                                    reason: format!(
                                        "failed to activate forwarded lock request: {error:#}"
                                    ),
                                }
                            } else {
                                DaemonControlResponse::Accepted
                            },
                            false,
                        )
                    }
                }
            } else {
                tracing::debug!(
                    state = %state,
                    "ignoring forwarded lock request while already active"
                );
                (
                    if wait_ready {
                        DaemonControlResponse::Locked {
                            already_active: true,
                            latency_report: None,
                        }
                    } else {
                        DaemonControlResponse::Accepted
                    },
                    false,
                )
            }
        }
        DaemonControlMessage::Stop => {
            tracing::info!("received daemon stop request over control socket");
            stop_response(*state, active.is_some())
        }
        DaemonControlMessage::Status => (
            DaemonControlResponse::Status(build_daemon_status(
                state,
                session_path,
                active.is_some(),
                control_socket_path(active),
                loaded_config,
                last_reload_result.as_deref(),
                *last_reload_unix_ms,
            )),
            false,
        ),
        DaemonControlMessage::Health => {
            (DaemonControlResponse::Health(build_daemon_health()), false)
        }
        DaemonControlMessage::ReloadConfig => (
            reload_config_response(
                options,
                state,
                control_socket_path(active),
                loaded_config,
                last_reload_result,
                last_reload_unix_ms,
                auth_policy,
                auth_state,
                suspend_state,
                weather,
                battery,
                now_playing,
            )
            .await,
            false,
        ),
    };

    if let Err(error) = ipc::write_daemon_control_response(&mut stream, &response).await {
        tracing::warn!("failed to acknowledge daemon control request: {error:#}");
    }

    Ok(stop_requested)
}

fn stop_response(state: LockState, curtain_running: bool) -> (DaemonControlResponse, bool) {
    if state.is_active() || curtain_running {
        // The daemon must remain available to authenticate the current lock.
        return (
            DaemonControlResponse::Error {
                reason: "cannot stop Veila while the session is locked; unlock first".to_string(),
            },
            false,
        );
    }
    (DaemonControlResponse::Accepted, true)
}

#[cfg(test)]
mod tests {
    use veila_common::ipc::DaemonControlResponse;

    use super::{LockState, stop_response};

    #[test]
    fn stop_is_refused_during_every_active_lock_phase() {
        for state in [LockState::Locking, LockState::Locked, LockState::Unlocking] {
            let (response, stop_requested) = stop_response(state, true);
            assert!(matches!(response, DaemonControlResponse::Error { .. }));
            assert!(!stop_requested);
            assert!(matches!(
                stop_response(state, false),
                (DaemonControlResponse::Error { .. }, false)
            ));
        }
        assert_eq!(
            stop_response(LockState::Unlocked, false),
            (DaemonControlResponse::Accepted, true)
        );
        assert!(matches!(
            stop_response(LockState::Unlocked, true),
            (DaemonControlResponse::Error { .. }, false)
        ));
    }
}
