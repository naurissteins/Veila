use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use nix::unistd::{Uid, User};
use veila_common::ipc::{
    DaemonControlResponse, DaemonHealth, DaemonReloadStatus, DaemonStatus, LiveReloadStatus,
};
use veila_common::{AppConfig, BatterySnapshot, LoadedConfig, NowPlayingSnapshot, WeatherSnapshot};

use super::{
    battery::BatteryHandle,
    mpris::NowPlayingHandle,
    prewarm,
    runtime::{ActiveRuntime, activate_lock},
    watch::effective_auto_reload_debounce_ms,
    weather::WeatherHandle,
};
use crate::{
    DaemonOptions,
    adapters::{logind, process},
    domain::{
        auth::{AuthPolicy, AuthState},
        lock_state::LockState,
    },
};

#[allow(clippy::too_many_arguments)]
pub(super) async fn activate_and_install(
    trigger: &'static str,
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    config_path: Option<&std::path::Path>,
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
    runtime: ActiveRuntime<'_>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
) -> Result<()> {
    let activation = activate_lock(
        trigger,
        session_proxy,
        state,
        config_path,
        weather_snapshot,
        battery_snapshot,
        now_playing_snapshot,
    )
    .await?;
    runtime.install_activation(activation);
    *auth_state = AuthState::new(auth_policy);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn activate_and_log(
    trigger: &'static str,
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    config_path: Option<&std::path::Path>,
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
    runtime: ActiveRuntime<'_>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
) -> Result<()> {
    let started_at = Instant::now();
    activate_and_install(
        trigger,
        session_proxy,
        state,
        config_path,
        weather_snapshot,
        battery_snapshot,
        now_playing_snapshot,
        runtime,
        auth_policy,
        auth_state,
    )
    .await?;
    tracing::info!(
        trigger,
        activation_elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        "lock timing summary"
    );
    Ok(())
}

pub(super) fn current_username() -> Result<String> {
    let uid = Uid::current();
    let Some(user) = User::from_uid(uid).context("failed to resolve current username")? else {
        return Err(anyhow!("current uid {uid} does not resolve to a user"));
    };

    Ok(user.name)
}

pub(super) fn build_daemon_status(
    state: &LockState,
    session: &str,
    curtain_running: bool,
    control_socket_path: Option<&Path>,
    loaded_config: &LoadedConfig,
    last_reload_result: Option<&str>,
    last_reload_unix_ms: Option<u64>,
) -> DaemonStatus {
    DaemonStatus {
        state: state.to_string(),
        session: session.to_string(),
        active_lock: state.is_active(),
        curtain_running,
        live_reload_available: state.is_active()
            && curtain_running
            && control_socket_path.is_some(),
        auto_reload_enabled: loaded_config.config.lock.auto_reload_config,
        auto_reload_debounce_ms: effective_auto_reload_debounce_ms(loaded_config),
        last_reload_result: last_reload_result.map(str::to_string),
        last_reload_unix_ms,
        config_path: loaded_config
            .path
            .as_deref()
            .map(|path| path.display().to_string()),
    }
}

pub(super) fn build_daemon_health() -> DaemonHealth {
    crate::local_build_info()
}

fn current_unix_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn apply_loaded_config(
    state: &LockState,
    control_socket_path: Option<&Path>,
    loaded_config: &mut LoadedConfig,
    new_loaded_config: LoadedConfig,
    last_reload_result: &mut Option<String>,
    last_reload_unix_ms: &mut Option<u64>,
    reload_source: &str,
    reload_debounce_ms: Option<u64>,
    auth_policy: &mut AuthPolicy,
    auth_state: &mut AuthState,
    weather: &WeatherHandle,
    battery: &BatteryHandle,
    now_playing: &NowPlayingHandle,
) -> Result<DaemonReloadStatus, String> {
    *loaded_config = new_loaded_config;
    *auth_policy = AuthPolicy::new(
        Duration::from_millis(loaded_config.config.lock.auth_backoff_base_ms),
        Duration::from_secs(loaded_config.config.lock.auth_backoff_max_seconds),
    );
    if !state.is_active() {
        *auth_state = AuthState::new(*auth_policy);
    }
    prewarm::spawn_background_prewarm(&loaded_config.config);
    weather.update_config(&loaded_config.config.weather);
    battery.update_config(&loaded_config.config.battery);
    now_playing.update_config(&loaded_config.config.now_playing);

    let live_reload = if !state.is_active() {
        Ok(LiveReloadStatus::NotActive)
    } else if let Some(control_socket_path) = control_socket_path {
        process::request_curtain_reload(control_socket_path)
            .await
            .map_err(|error| format!("failed to forward live config reload to curtain: {error:#}"))
            .map(|_| LiveReloadStatus::Forwarded)
    } else {
        Err(
            "failed to forward live config reload to curtain: active lock has no control socket"
                .to_string(),
        )
    }?;

    let config = loaded_config
        .path
        .as_deref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "defaults".to_string());
    let live_reload_status = match live_reload {
        LiveReloadStatus::NotActive => "not-active",
        LiveReloadStatus::Forwarded => "forwarded",
    };
    if let Some(reload_debounce_ms) = reload_debounce_ms {
        tracing::info!(
            active_lock = state.is_active(),
            live_reload = live_reload_status,
            config,
            reload_source,
            reload_debounce_ms,
            "reloaded daemon config"
        );
    } else {
        tracing::info!(
            active_lock = state.is_active(),
            live_reload = live_reload_status,
            config,
            reload_source,
            "reloaded daemon config"
        );
    }

    *last_reload_result = Some(format!("ok:{reload_source}"));
    *last_reload_unix_ms = current_unix_ms();

    Ok(DaemonReloadStatus {
        config_path: loaded_config
            .path
            .as_deref()
            .map(|path| path.display().to_string()),
        active_lock: state.is_active(),
        reload_source: reload_source.to_string(),
        live_reload,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn reload_config_response(
    options: &DaemonOptions,
    state: &LockState,
    control_socket_path: Option<&Path>,
    loaded_config: &mut LoadedConfig,
    last_reload_result: &mut Option<String>,
    last_reload_unix_ms: &mut Option<u64>,
    auth_policy: &mut AuthPolicy,
    auth_state: &mut AuthState,
    weather: &WeatherHandle,
    battery: &BatteryHandle,
    now_playing: &NowPlayingHandle,
) -> DaemonControlResponse {
    match AppConfig::load(options.config_path.as_deref()) {
        Ok(new_loaded_config) => match apply_loaded_config(
            state,
            control_socket_path,
            loaded_config,
            new_loaded_config,
            last_reload_result,
            last_reload_unix_ms,
            "manual",
            None,
            auth_policy,
            auth_state,
            weather,
            battery,
            now_playing,
        )
        .await
        {
            Ok(status) => DaemonControlResponse::Reloaded(status),
            Err(reason) => {
                *last_reload_result = Some(format!("error:manual:{reason}"));
                *last_reload_unix_ms = current_unix_ms();
                tracing::warn!("{reason}");
                DaemonControlResponse::Error { reason }
            }
        },
        Err(error) => {
            let reason = format!("failed to reload daemon config: {error:#}");
            *last_reload_result = Some(format!("error:manual:{reason}"));
            *last_reload_unix_ms = current_unix_ms();
            DaemonControlResponse::Error { reason }
        }
    }
}
