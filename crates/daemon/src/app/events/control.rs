use anyhow::Result;
use tokio::net::UnixStream;
use veila_common::{
    BatterySnapshot, LoadedConfig, NowPlayingSnapshot, WeatherSnapshot,
    ipc::{DaemonControlMessage, DaemonControlResponse},
};

use crate::{
    DaemonOptions,
    adapters::{ipc, logind},
    domain::auth::AuthPolicy,
};

use super::super::{
    battery::BatteryHandle,
    helpers::{activate_and_log, build_daemon_health, build_daemon_status, reload_config_response},
    mpris::NowPlayingHandle,
    runtime::ActiveRuntime,
    state::RuntimeSlots,
    weather::WeatherHandle,
};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_control_connection(
    mut stream: UnixStream,
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
    slots: RuntimeSlots<'_>,
    auth_policy: &mut AuthPolicy,
) -> Result<bool> {
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

    let Some(message) = ipc::read_daemon_control_message(&mut stream).await? else {
        return Ok(false);
    };

    let (response, stop_requested) = match message {
        DaemonControlMessage::LockNow => {
            if !state.is_active() {
                if let Err(error) = activate_and_log(
                    "forwarded",
                    session_proxy,
                    state,
                    options.config_path.as_deref(),
                    weather_snapshot,
                    battery_snapshot,
                    now_playing_snapshot,
                    ActiveRuntime::new(
                        curtain,
                        auth_listener,
                        auth_socket_path,
                        control_socket_path,
                        auth_results,
                        auth_sender,
                    ),
                    *auth_policy,
                    auth_state,
                )
                .await
                {
                    tracing::error!("failed to activate forwarded lock request: {error:#}");
                }
            } else {
                tracing::debug!(
                    state = %state,
                    "ignoring forwarded lock request while already active"
                );
            }

            (DaemonControlResponse::Accepted, false)
        }
        DaemonControlMessage::Stop => {
            tracing::info!("received daemon stop request over control socket");
            (DaemonControlResponse::Accepted, true)
        }
        DaemonControlMessage::Status => (
            DaemonControlResponse::Status(build_daemon_status(
                state,
                session_path,
                curtain.is_some(),
                control_socket_path.as_deref(),
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
                control_socket_path.as_deref(),
                loaded_config,
                last_reload_result,
                last_reload_unix_ms,
                auth_policy,
                auth_state,
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
