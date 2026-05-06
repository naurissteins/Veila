use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use nix::unistd::{Uid, User};
use veila_common::ipc::{
    DaemonControlResponse, DaemonHealth, DaemonReloadStatus, DaemonStatus, LiveReloadStatus,
};
use veila_common::{AppConfig, BatterySnapshot, LoadedConfig, NowPlayingSnapshot, WeatherSnapshot};

use super::{
    battery::BatteryHandle,
    memory,
    mpris::NowPlayingHandle,
    prewarm,
    runtime::{ActiveRuntime, activate_lock},
    state::BackgroundSelectionState,
    suspend::{LockedSuspendState, suspend_delay_seconds},
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
    initial_background_path: Option<&Path>,
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
    runtime: ActiveRuntime<'_>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
    suspend_state: &mut LockedSuspendState,
) -> Result<()> {
    let activation = activate_lock(
        trigger,
        session_proxy,
        state,
        config_path,
        initial_background_path,
        weather_snapshot,
        battery_snapshot,
        now_playing_snapshot,
    )
    .await?;
    runtime.install_activation(activation);
    *auth_state = AuthState::new(auth_policy);
    suspend_state.arm(Instant::now());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn activate_and_log(
    trigger: &'static str,
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    config_path: Option<&std::path::Path>,
    initial_background_path: Option<&Path>,
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
    runtime: ActiveRuntime<'_>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
    suspend_state: &mut LockedSuspendState,
) -> Result<()> {
    let started_at = Instant::now();
    activate_and_install(
        trigger,
        session_proxy,
        state,
        config_path,
        initial_background_path,
        weather_snapshot,
        battery_snapshot,
        now_playing_snapshot,
        runtime,
        auth_policy,
        auth_state,
        suspend_state,
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

pub(super) fn select_initial_background_path(
    config: &AppConfig,
    background_selection: &mut Option<BackgroundSelectionState>,
) -> Option<PathBuf> {
    let background = &config.background;
    if !background.slideshow_enabled() {
        *background_selection = None;
        return None;
    }

    let slideshow = background.slideshow.as_ref()?;
    let paths = background.resolved_slideshow_paths().ok()?;
    if paths.is_empty() {
        *background_selection = None;
        return None;
    }

    match (slideshow.mode, slideshow.order) {
        (
            veila_common::config::BackgroundSlideshowMode::Timed,
            veila_common::config::BackgroundSlideshowOrder::Sequence,
        ) => {
            *background_selection = None;
            paths.into_iter().next()
        }
        (
            veila_common::config::BackgroundSlideshowMode::Timed,
            veila_common::config::BackgroundSlideshowOrder::Random,
        )
        | (veila_common::config::BackgroundSlideshowMode::LockOnly, _) => {
            let selection = background_selection.get_or_insert_with(Default::default);
            selection.next_path(&paths, slideshow.order)
        }
    }
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
    suspend_state: &mut LockedSuspendState,
    weather: &WeatherHandle,
    battery: &BatteryHandle,
    now_playing: &NowPlayingHandle,
) -> Result<DaemonReloadStatus, String> {
    let prewarm_needed =
        prewarm::prewarm_inputs_changed(&loaded_config.config, &new_loaded_config.config);
    let rss_kib_before_reload = memory::current_rss_kib();
    *loaded_config = new_loaded_config;
    *auth_policy = AuthPolicy::new(
        Duration::from_millis(loaded_config.config.lock.auth_backoff_base_ms),
        Duration::from_secs(loaded_config.config.lock.auth_backoff_max_seconds),
    );
    suspend_state.set_policy(
        suspend_delay_seconds(&loaded_config.config).map(Duration::from_secs),
        loaded_config.config.lock.suspend_only_on_battery,
        Instant::now(),
        state.is_active(),
    );
    if !state.is_active() {
        *auth_state = AuthState::new(*auth_policy);
    }
    if prewarm_needed {
        prewarm::spawn_background_prewarm(loaded_config.path.as_deref());
    } else {
        tracing::debug!(
            reload_source,
            "skipping background prewarm after config reload because prewarm inputs are unchanged"
        );
    }
    weather.update_config(&loaded_config.config.weather);
    battery.update_config(
        &loaded_config.config.battery,
        loaded_config.config.lock.suspend_only_on_battery,
    );
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
            background_prewarm_triggered = prewarm_needed,
            rss_kib_before_reload,
            rss_kib_after_reload = memory::current_rss_kib(),
            "reloaded daemon config"
        );
    } else {
        tracing::info!(
            active_lock = state.is_active(),
            live_reload = live_reload_status,
            config,
            reload_source,
            background_prewarm_triggered = prewarm_needed,
            rss_kib_before_reload,
            rss_kib_after_reload = memory::current_rss_kib(),
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
    suspend_state: &mut LockedSuspendState,
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
            suspend_state,
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
