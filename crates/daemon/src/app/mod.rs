mod battery;
mod cache;
mod events;
mod helpers;
mod memory;
mod mpris;
mod output_probe;
mod prewarm;
mod runtime;
mod state;
mod suspend;
mod watch;
mod weather;

use std::path::{Path, PathBuf};

use crate::{DaemonOptions, adapters::logind};
use anyhow::{Context, Result};
use futures_util::StreamExt;
use tokio::{
    net::UnixListener,
    signal::unix::{SignalKind, signal},
    time::{self, MissedTickBehavior},
};
use veila_common::AppConfig;

use self::events::{
    handle_auth_connection, handle_auth_result, handle_control_connection, handle_curtain_exit,
    handle_lock_signal, handle_now_playing_update, handle_unlock_signal, shutdown_runtime,
};
use self::helpers::{activate_and_log, current_username};
use self::runtime::{
    ActiveRuntime, accept_auth_connection, accept_control_connection, receive_auth_result,
    wait_for_curtain_exit,
};
use self::state::AppRuntime;
use self::watch::{AutoReloadTrigger, AutoReloadWatcher, effective_auto_reload_debounce_ms};

pub async fn run_background_prewarm_once(config_path: Option<&Path>) -> Result<()> {
    let loaded_config =
        AppConfig::load(config_path).context("failed to load config for prewarm")?;
    prewarm::run_background_prewarm_once(loaded_config.config).await;
    Ok(())
}

pub async fn run(
    options: DaemonOptions,
    mut control_listener: UnixListener,
    daemon_control_socket_path: PathBuf,
) -> Result<()> {
    let mut runtime = AppRuntime::new(
        AppConfig::load(options.config_path.as_deref()).context("failed to load daemon config")?,
    );
    prewarm::spawn_background_prewarm(runtime.loaded_config.path.as_deref());
    cache::spawn_background_cache_pruner();
    let connection = logind::connect_system().await?;
    let manager_proxy = logind::ManagerProxy::new(&connection)
        .await
        .context("failed to create logind manager proxy")?;
    let session_path = logind::get_session_path(&connection, options.session_id.as_deref()).await?;
    let session_proxy = logind::session_proxy(&connection, &session_path).await?;
    let username = current_username()?;
    let mut lock_stream = session_proxy
        .receive_lock()
        .await
        .context("failed to subscribe to logind Lock signal")?;
    let mut unlock_stream = session_proxy
        .receive_unlock()
        .await
        .context("failed to subscribe to logind Unlock signal")?;
    let mut prepare_for_sleep_stream = manager_proxy
        .receive_prepare_for_sleep()
        .await
        .context("failed to subscribe to logind PrepareForSleep signal")?;
    let mut sigint =
        signal(SignalKind::interrupt()).context("failed to register SIGINT handler")?;
    let mut sigterm =
        signal(SignalKind::terminate()).context("failed to register SIGTERM handler")?;
    let mut now_playing_updates = runtime.now_playing.subscribe();
    let mut auto_reload_watcher =
        AutoReloadWatcher::new(options.config_path.as_deref(), &runtime.loaded_config);
    let mut auto_reload_tick = time::interval(std::time::Duration::from_millis(250));
    auto_reload_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    tracing::info!(
        session = %session_path,
        session_id_override = options.session_id.as_deref().unwrap_or("none"),
        manual_lock = options.lock_now,
        config = runtime.loaded_config.path.as_deref().map(|path| path.display().to_string()).unwrap_or_else(|| "defaults".to_string()),
        "veilad ready"
    );

    if options.lock_now {
        tracing::info!("manual lock requested via --lock-now");
        let now_playing_snapshot = runtime.now_playing.current_snapshot();
        let initial_background_path = runtime.select_initial_background_path();
        activate_and_log(
            "manual",
            &session_proxy,
            &mut runtime.state,
            options.config_path.as_deref(),
            initial_background_path.as_deref(),
            runtime.weather.current_snapshot().as_ref(),
            runtime.battery.current_snapshot().as_ref(),
            now_playing_snapshot.as_ref(),
            ActiveRuntime::new(
                &mut runtime.curtain,
                &mut runtime.auth_listener,
                &mut runtime.auth_socket_path,
                &mut runtime.control_socket_path,
                &mut runtime.auth_results,
                &mut runtime.auth_sender,
            ),
            runtime.auth_policy,
            &mut runtime.auth_state,
            &mut runtime.suspend_state,
        )
        .await
        .context("failed to activate manual lock")?;
    }

    loop {
        tokio::select! {
            Some(_) = lock_stream.next() => {
                let was_active = runtime.state.is_active();
                let weather_snapshot = runtime.weather.current_snapshot();
                let battery_snapshot = runtime.battery.current_snapshot();
                let now_playing_snapshot = runtime.now_playing.current_snapshot();
                let initial_background_path = runtime.select_initial_background_path();
                let (auth_policy, suspend_state, slots) = runtime.slots_with_policy_and_suspend();
                handle_lock_signal(
                    "logind",
                    &session_proxy,
                    options.config_path.as_deref(),
                    initial_background_path.as_deref(),
                    weather_snapshot.as_ref(),
                    battery_snapshot.as_ref(),
                    now_playing_snapshot.as_ref(),
                    slots,
                    auth_policy,
                    suspend_state,
                ).await;
                if !was_active && runtime.state.is_active() {
                    runtime.last_power_status_snapshot = None;
                    runtime.power_status_sent = false;
                }
            }
            Some(_) = unlock_stream.next() => {
                let (auth_policy, suspend_state, slots) = runtime.slots_with_policy_and_suspend();
                handle_unlock_signal(
                    &session_proxy,
                    slots,
                    auth_policy,
                    suspend_state,
                ).await;
                if !runtime.state.is_active() {
                    runtime.last_power_status_snapshot = None;
                    runtime.power_status_sent = false;
                }
            }
            Some(signal) = prepare_for_sleep_stream.next() => {
                match signal.args() {
                    Ok(args) if *args.start() => {
                        if runtime.state.is_active()
                            && let Some(control_socket_path) = runtime.control_socket_path.as_deref()
                        {
                            match crate::adapters::process::request_curtain_arm_resume_input_guard(control_socket_path).await {
                                Ok(()) => {}
                                Err(error) => {
                                    tracing::warn!("failed to arm curtain resume input guard before sleep: {error:#}");
                                }
                            }
                        }
                    }
                    Ok(_) => {
                        if runtime.state.is_active()
                            && let Some(control_socket_path) = runtime.control_socket_path.as_deref()
                        {
                            match crate::adapters::process::request_curtain_mark_resumed(control_socket_path).await {
                                Ok(()) => {}
                                Err(error) => {
                                    tracing::warn!("failed to mark curtain as resumed after sleep: {error:#}");
                                }
                            }
                        }
                    }
                    Err(error) => {
                        tracing::warn!("failed to decode logind PrepareForSleep signal: {error}");
                    }
                }
            }
            result = wait_for_curtain_exit(&mut runtime.curtain), if runtime.curtain.is_some() => {
                let weather_snapshot = runtime.weather.current_snapshot();
                let battery_snapshot = runtime.battery.current_snapshot();
                let now_playing_snapshot = runtime.now_playing.current_snapshot();
                let initial_background_path = runtime.select_initial_background_path();
                let (auth_policy, suspend_state, slots) = runtime.slots_with_policy_and_suspend();
                handle_curtain_exit(
                    result?,
                    &session_proxy,
                    options.config_path.as_deref(),
                    initial_background_path.as_deref(),
                    weather_snapshot.as_ref(),
                    battery_snapshot.as_ref(),
                    now_playing_snapshot.as_ref(),
                    slots,
                    auth_policy,
                    suspend_state,
                ).await;
                if !runtime.state.is_active() {
                    runtime.last_power_status_snapshot = None;
                    runtime.power_status_sent = false;
                }
            }
            result = accept_auth_connection(&mut runtime.auth_listener), if runtime.state.is_active() && runtime.auth_listener.is_some() => {
                handle_auth_connection(
                    &username,
                    &runtime.auth_sender,
                    &mut runtime.auth_state,
                    &mut runtime.suspend_state,
                    result?,
                ).await?;
            }
            result = receive_auth_result(&mut runtime.auth_results), if runtime.auth_results.is_some() => {
                let Some(result) = result else {
                    continue;
                };

                let (auth_policy, suspend_state, slots) = runtime.slots_with_policy_and_suspend();
                handle_auth_result(
                    &session_proxy,
                    slots,
                    auth_policy,
                    result,
                    suspend_state,
                ).await;
            }
            result = accept_control_connection(&mut control_listener) => {
                let weather = runtime.weather.clone();
                let battery = runtime.battery.clone();
                let now_playing = runtime.now_playing.clone();
                let weather_snapshot = weather.current_snapshot();
                let battery_snapshot = battery.current_snapshot();
                let now_playing_snapshot = runtime.now_playing.current_snapshot();
                let (
                    loaded_config,
                    last_reload_result,
                    last_reload_unix_ms,
                    auth_policy,
                    background_selection,
                    suspend_state,
                    slots,
                ) = runtime.control_inputs();
                if handle_control_connection(
                    result?,
                    &options,
                    &session_proxy,
                    &session_path,
                    loaded_config,
                    last_reload_result,
                    last_reload_unix_ms,
                    weather_snapshot.as_ref(),
                    battery_snapshot.as_ref(),
                    now_playing_snapshot.as_ref(),
                    &weather,
                    &battery,
                    &now_playing,
                    background_selection,
                    suspend_state,
                    slots,
                    auth_policy,
                ).await? {
                    break;
                }
            }
            result = now_playing_updates.changed() => {
                if result.is_err() {
                    continue;
                }

                let snapshot = now_playing_updates.borrow().clone();
                handle_now_playing_update(
                    &runtime.state,
                    runtime.control_socket_path.as_deref(),
                    snapshot.as_ref(),
                ).await;
            }
            _ = auto_reload_tick.tick() => {
                let now = std::time::Instant::now();
                let suspend_decision = runtime.suspend_state.evaluate(
                    now,
                    runtime.state.is_active(),
                    runtime.auth_state.in_flight(),
                    runtime.battery.current_snapshot().as_ref(),
                    runtime.now_playing.currently_playing(),
                );
                match suspend_decision {
                    suspend::SuspendDecision::Ready => {
                        runtime.suspend_state.clear_reported_skip_reason();
                        if let Some(control_socket_path) = runtime.control_socket_path.as_deref() {
                            match crate::adapters::process::request_curtain_arm_resume_input_guard(control_socket_path).await {
                                Ok(()) => {}
                                Err(error) => {
                                    tracing::warn!("failed to arm curtain resume input guard before suspend: {error:#}");
                                }
                            }
                        }
                        runtime.suspend_state.mark_requested();
                        match suspend::request_system_suspend(&connection).await {
                            Ok(()) => {
                                tracing::info!(
                                    suspend_seconds = runtime.loaded_config.config.lock.suspend_seconds,
                                    suspend_only_on_battery = runtime
                                        .loaded_config
                                        .config
                                        .lock
                                        .suspend_only_on_battery,
                                    skip_suspend_while_media_playing = runtime
                                        .loaded_config
                                        .config
                                        .lock
                                        .skip_suspend_while_media_playing,
                                    "requesting system suspend after locked inactivity"
                                );
                            }
                            Err(error) => {
                                tracing::warn!(
                                    "failed to request system suspend after locked inactivity: {error:#}"
                                );
                            }
                        }
                    }
                    suspend::SuspendDecision::Skipped(reason) => {
                        if let Some(reason) = runtime.suspend_state.note_skip_reason(reason) {
                            tracing::info!(
                                suspend_seconds = runtime.loaded_config.config.lock.suspend_seconds,
                                suspend_only_on_battery = runtime
                                    .loaded_config
                                    .config
                                    .lock
                                    .suspend_only_on_battery,
                                skip_suspend_while_media_playing = runtime
                                    .loaded_config
                                    .config
                                    .lock
                                    .skip_suspend_while_media_playing,
                                reason = reason.as_str(),
                                "skipping locked idle suspend"
                            );
                        }
                    }
                    suspend::SuspendDecision::Pending => {
                        runtime.suspend_state.clear_reported_skip_reason();
                    }
                }

                if runtime.state.is_active() {
                    let power_status_snapshot = runtime
                        .suspend_state
                        .power_status_snapshot(now, runtime.state.is_active());
                    let power_status_changed = !runtime.power_status_sent
                        || runtime.last_power_status_snapshot != power_status_snapshot;
                    if power_status_changed
                        && let Some(control_socket_path) = runtime.control_socket_path.as_deref()
                    {
                        match crate::adapters::process::request_curtain_power_status_update(
                            control_socket_path,
                            power_status_snapshot.as_ref(),
                        )
                        .await
                        {
                            Ok(()) => {
                                runtime.last_power_status_snapshot = power_status_snapshot;
                                runtime.power_status_sent = true;
                            }
                            Err(error) => {
                                tracing::warn!(
                                    "failed to forward power status update to curtain: {error:#}"
                                );
                            }
                        }
                    }
                } else {
                    runtime.last_power_status_snapshot = None;
                    runtime.power_status_sent = false;
                }

                match auto_reload_watcher.poll(options.config_path.as_deref(), &runtime.loaded_config) {
                    Some(AutoReloadTrigger::Config) => {
                        let current_auto_reload = runtime.loaded_config.config.lock.auto_reload_config;
                        match AppConfig::load(options.config_path.as_deref()) {
                            Ok(new_loaded_config) => {
                                let should_apply = current_auto_reload || new_loaded_config.config.lock.auto_reload_config;
                                if should_apply {
                                    let debounce_ms = effective_auto_reload_debounce_ms(&new_loaded_config);
                                    let weather = runtime.weather.clone();
                                    let battery = runtime.battery.clone();
                                    let now_playing = runtime.now_playing.clone();
                                    let (
                                        loaded_config,
                                        last_reload_result,
                                        last_reload_unix_ms,
                                        auth_policy,
                                        _background_selection,
                                        suspend_state,
                                        slots,
                                    ) = runtime.control_inputs();
                                    match helpers::apply_loaded_config(
                                        slots.state,
                                        slots.control_socket_path.as_deref(),
                                        loaded_config,
                                        new_loaded_config,
                                        last_reload_result,
                                        last_reload_unix_ms,
                                        "config-change",
                                        Some(debounce_ms),
                                        auth_policy,
                                        slots.auth_state,
                                        suspend_state,
                                        &weather,
                                        &battery,
                                        &now_playing,
                                    ).await {
                                        Ok(_) => {}
                                        Err(reason) => {
                                            *last_reload_result =
                                                Some(format!("error:config-change:{reason}"));
                                            *last_reload_unix_ms = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .ok()
                                                .and_then(|duration| u64::try_from(duration.as_millis()).ok());
                                            tracing::warn!("{reason}");
                                        }
                                    }
                                } else {
                                    tracing::debug!("ignoring config file change because auto_reload_config is disabled");
                                }
                            }
                            Err(error) => {
                                if current_auto_reload {
                                    runtime.last_reload_result = Some(format!(
                                        "error:config-change:failed to auto reload daemon config after config file change: {error:#}"
                                    ));
                                    runtime.last_reload_unix_ms = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .ok()
                                        .and_then(|duration| u64::try_from(duration.as_millis()).ok());
                                    tracing::warn!("failed to auto reload daemon config after config file change: {error:#}");
                                }
                            }
                        }
                    }
                    Some(trigger @ (AutoReloadTrigger::Theme | AutoReloadTrigger::Include)) => {
                        let (source, file_kind) = match trigger {
                            AutoReloadTrigger::Theme => ("theme-change", "theme"),
                            AutoReloadTrigger::Include => ("include-change", "include"),
                            _ => unreachable!(),
                        };
                        match AppConfig::load(options.config_path.as_deref()) {
                            Ok(new_loaded_config) => {
                                let debounce_ms = effective_auto_reload_debounce_ms(&new_loaded_config);
                                let weather = runtime.weather.clone();
                                let battery = runtime.battery.clone();
                                let now_playing = runtime.now_playing.clone();
                                let (
                                    loaded_config,
                                    last_reload_result,
                                    last_reload_unix_ms,
                                    auth_policy,
                                    _background_selection,
                                    suspend_state,
                                    slots,
                                ) = runtime.control_inputs();
                                match helpers::apply_loaded_config(
                                    slots.state,
                                    slots.control_socket_path.as_deref(),
                                    loaded_config,
                                    new_loaded_config,
                                    last_reload_result,
                                    last_reload_unix_ms,
                                    source,
                                    Some(debounce_ms),
                                    auth_policy,
                                    slots.auth_state,
                                    suspend_state,
                                    &weather,
                                    &battery,
                                    &now_playing,
                                ).await {
                                    Ok(_) => {}
                                    Err(reason) => {
                                        *last_reload_result =
                                            Some(format!("error:{source}:{reason}"));
                                        *last_reload_unix_ms = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .ok()
                                            .and_then(|duration| u64::try_from(duration.as_millis()).ok());
                                        tracing::warn!("{reason}");
                                    }
                                }
                            }
                            Err(error) => {
                                runtime.last_reload_result = Some(format!(
                                    "error:{source}:failed to auto reload daemon config after {file_kind} file change: {error:#}"
                                ));
                                runtime.last_reload_unix_ms = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .ok()
                                    .and_then(|duration| u64::try_from(duration.as_millis()).ok());
                                tracing::warn!(
                                    "failed to auto reload daemon config after {file_kind} file change: {error:#}"
                                );
                            }
                        }
                    }
                    Some(AutoReloadTrigger::Wallpaper) => {
                        match AppConfig::load(options.config_path.as_deref()) {
                            Ok(new_loaded_config) => {
                                let debounce_ms = effective_auto_reload_debounce_ms(&new_loaded_config);
                                let weather = runtime.weather.clone();
                                let battery = runtime.battery.clone();
                                let now_playing = runtime.now_playing.clone();
                                let (
                                    loaded_config,
                                    last_reload_result,
                                    last_reload_unix_ms,
                                    auth_policy,
                                    _background_selection,
                                    suspend_state,
                                    slots,
                                ) = runtime.control_inputs();
                                match helpers::apply_loaded_config(
                                    slots.state,
                                    slots.control_socket_path.as_deref(),
                                    loaded_config,
                                    new_loaded_config,
                                    last_reload_result,
                                    last_reload_unix_ms,
                                    "wallpaper-change",
                                    Some(debounce_ms),
                                    auth_policy,
                                    slots.auth_state,
                                    suspend_state,
                                    &weather,
                                    &battery,
                                    &now_playing,
                                    ).await {
                                        Ok(_) => {}
                                    Err(reason) => {
                                        *last_reload_result =
                                            Some(format!("error:wallpaper-change:{reason}"));
                                        *last_reload_unix_ms = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .ok()
                                            .and_then(|duration| u64::try_from(duration.as_millis()).ok());
                                        tracing::warn!("{reason}");
                                    }
                                }
                            }
                            Err(error) => {
                                runtime.last_reload_result = Some(format!(
                                    "error:wallpaper-change:failed to auto reload daemon config after wallpaper change: {error:#}"
                                ));
                                runtime.last_reload_unix_ms = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .ok()
                                    .and_then(|duration| u64::try_from(duration.as_millis()).ok());
                                tracing::warn!("failed to auto reload daemon config after wallpaper change: {error:#}");
                            }
                        }
                    }
                    None => {}
                }
            }
            _ = sigint.recv() => {
                tracing::info!("received SIGINT");
                break;
            }
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM");
                break;
            }
        }
    }

    let (auth_policy, slots) = runtime.slots_with_policy();
    shutdown_runtime(&session_proxy, slots, auth_policy).await;

    let _ = std::fs::remove_file(&daemon_control_socket_path);
    tracing::info!("veilad exiting");
    Ok(())
}
