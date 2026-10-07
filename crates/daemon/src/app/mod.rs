use self::runtime::control_socket_path;

mod auto_reload;
mod battery;
mod cache;
mod connections;
mod events;
mod fingerprint;
mod helpers;
mod idle;
mod maintenance;
mod memory;
mod mpris;
pub(crate) mod output_probe;
mod prewarm;
mod recovery;
mod runtime;
mod sleep;
mod startup;
mod state;
mod suspend;
mod unlock;
mod watch;
mod weather;

use std::path::{Path, PathBuf};

use crate::{DaemonOptions, adapters::logind};
use anyhow::{Context, Result};
use futures_util::StreamExt;
use tokio::{
    net::UnixListener,
    signal::unix::{SignalKind, signal},
    time,
};
use veila_common::{AppConfig, LoadedConfig, elapsed_ms, elapsed_us};

use self::events::{
    ShutdownGate, handle_auth_message, handle_auth_result, handle_control_message,
    handle_curtain_exit, handle_now_playing_update, shutdown_runtime,
};
use self::helpers::current_username;
use self::runtime::{
    ClientMessageContext, accept_auth_connection, accept_control_connection, receive_auth_result,
    wait_for_curtain_exit,
};
use self::state::AppRuntime;
use self::watch::AutoReloadWatcher;

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
    let config_load_started_at = std::time::Instant::now();
    let loaded_config = match AppConfig::load(options.config_path.as_deref()) {
        Ok(loaded_config) => loaded_config,
        Err(error) => {
            tracing::warn!(
                "failed to load daemon config: {error:#}; using defaults so the emergency fallback can still lock"
            );
            LoadedConfig {
                path: options.config_path.clone(),
                config: AppConfig::default(),
            }
        }
    };
    let daemon_config_load_ms = elapsed_ms(config_load_started_at);
    let daemon_config_load_us = elapsed_us(config_load_started_at);
    if let Err(error) = cache::harden_existing_cache_root() {
        tracing::warn!("failed to secure existing cache directory: {error}");
    }
    let connection = logind::connect_system().await?;
    let manager_proxy = logind::ManagerProxy::new(&connection)
        .await
        .context("failed to create logind manager proxy")?;
    let session_path = logind::get_session_path(&connection, options.session_id.as_deref()).await?;
    let session_proxy = logind::session_proxy(&connection, &session_path).await?;
    let mut runtime = AppRuntime::new(loaded_config, daemon_config_load_ms, daemon_config_load_us);
    recovery::adopt_surviving_curtain(session_path.as_str(), &mut runtime).await?;
    if runtime.state.is_active() {
        runtime::update_locked_hint(&session_proxy, true).await;
    }
    prewarm::spawn_background_prewarm(runtime.loaded_config.path.as_deref());
    cache::spawn_background_cache_pruner();
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
    let mut maintenance_tick = maintenance::MaintenanceClock::new();
    let (auth_connection_sender, mut auth_connections) = tokio::sync::mpsc::unbounded_channel();
    let (control_connection_sender, mut control_connections) =
        tokio::sync::mpsc::unbounded_channel();
    let mut curtain_wait_retry_at = None;
    let mut shutdown_gate = ShutdownGate::default();
    let mut startup = startup::PendingStartup::default();
    let mut unlock = unlock::PendingUnlock::default();
    let mut next_session_close_check = std::time::Instant::now();

    tracing::info!(
        session = %session_path,
        session_id_override = options.session_id.as_deref().unwrap_or("none"),
        daemon_config_load_ms,
        daemon_config_load_us,
        config = runtime.loaded_config.path.as_deref().map(|path| path.display().to_string()).unwrap_or_else(|| "defaults".to_string()),
        "daemon ready"
    );
    idle::warn_about_legacy_idle_service();

    loop {
        startup.reconcile(&runtime);
        if startup.unlock_requested
            && !startup.is_acquiring()
            && !unlock.is_pending()
            && startup.relock.is_none()
        {
            sleep::finish_deferred_sleep(&mut startup, &mut runtime, &manager_proxy).await;
        }
        if startup.unlock_requested
            && !startup.is_acquiring()
            && !unlock.is_pending()
            && !unlock.begin(&mut runtime, None)
        {
            startup.unlock_requested = false;
        }
        startup.resume_relock(
            &mut runtime,
            &connection,
            &session_path,
            options.config_path.as_deref(),
        );
        if !startup.is_acquiring() && !unlock.is_pending() && startup.relock.is_none() {
            sleep::finish_deferred_sleep(&mut startup, &mut runtime, &manager_proxy).await;
        }
        if !startup.is_pending()
            && !unlock.is_pending()
            && let Some(request) = startup.reloads.pop_front()
        {
            handle_control_message(
                request.stream,
                request.message,
                &options,
                session_path.as_str(),
                &mut runtime,
            )
            .await?;
            continue;
        }
        if shutdown_gate.ready(runtime.state, runtime.active.is_some()) {
            tracing::info!("pending daemon shutdown can finish after unlock");
            break;
        }
        maintenance_tick.sync(
            runtime.state.is_active()
                || shutdown_gate.is_requested()
                || auto_reload_watcher.is_pending(),
        );
        runtime.idle.sync(&runtime.loaded_config.config.idle);
        runtime
            .sleep_lock
            .sync(
                &manager_proxy,
                runtime.loaded_config.config.idle.lock_before_sleep,
            )
            .await;

        // These waits borrow separate fields until the selected branch starts.
        let active_present = runtime.active.is_some();
        let (curtain, auth_listener, auth_results) = match runtime.active.as_mut() {
            Some(active) => (
                Some(&mut active.curtain),
                Some(&mut active.auth_listener),
                Some(&mut active.auth_results),
            ),
            None => (None, None, None),
        };
        tokio::select! {
            Some(_) = lock_stream.next() => {
                startup.begin("logind", &mut runtime, &connection, &session_path, options.config_path.as_deref(), false, veila_common::ipc::LatencyReportMode::Disabled);
            }
            () = runtime.idle.idled() => {
                startup.begin("idle", &mut runtime, &connection, &session_path, options.config_path.as_deref(), false, veila_common::ipc::LatencyReportMode::Disabled);
            }
            milestone = startup.next(), if !unlock.is_pending() => {
                startup.complete(milestone, &mut runtime).await;
            }
            progress = unlock.next() => {
                if unlock.advance(progress, &mut runtime).await {
                    startup.unlock_requested = false;
                    if !runtime.state.is_active() { runtime.fingerprint.stop().await; }
                }
            }
            Some(_) = unlock_stream.next() => {
                if runtime.state.is_active() { startup.request_unlock(); }
            }
            Some(signal) = prepare_for_sleep_stream.next() => {
                match signal.args() {
                    Ok(args) => {
                        sleep::handle_prepare_for_sleep(*args.start(), &mut runtime, &mut startup, &connection, &session_path, &manager_proxy, options.config_path.as_deref()).await;
                    }
                    Err(error) => {
                        tracing::warn!("failed to decode logind PrepareForSleep signal: {error}");
                    }
                }
            }
            result = wait_for_curtain_exit(curtain), if active_present && unlock.can_wait_for_exit() && curtain_wait_retry_at.is_none_or(|retry_at| std::time::Instant::now() >= retry_at) => {
                match result {
                    Ok(status) => {
                        curtain_wait_retry_at = None;
                        if unlock.is_pending() {
                            unlock.released(&mut runtime, &connection, &session_path);
                        } else {
                            let (auth_policy, slots) = runtime.slots_with_policy();
                            handle_curtain_exit(status, slots, auth_policy).await;
                        }
                        if !runtime.state.is_active() {
                            runtime.last_power_status_snapshot = None;
                            runtime.power_status_sent = false;
                            runtime.fingerprint.stop().await;
                        }
                    }
                    Err(error) => {
                        tracing::warn!("failed while waiting for curtain process: {error:#}");
                        if unlock.is_pending() {
                            unlock.fail(&mut runtime);
                            startup.unlock_requested = false;
                        }
                        curtain_wait_retry_at = Some(
                            std::time::Instant::now() + std::time::Duration::from_millis(250),
                        );
                    }
                }
            }
            result = accept_auth_connection(auth_listener), if runtime.state.is_active() && active_present && !unlock.is_pending() => {
                match result {
                    Ok(stream) => {
                        if let Some(generation) = runtime.active.as_ref().map(|active| active.auth_socket_path.clone()) {
                            connections::spawn_auth_reader(
                                stream,
                                generation,
                                auth_connection_sender.clone(),
                            );
                        } else {
                            tracing::warn!("discarding auth connection without an active socket generation");
                        }
                    }
                    Err(error) => {
                        tracing::warn!("failed to accept auth connection: {error:#}");
                    }
                }
            }
            Some(connection) = auth_connections.recv(), if !unlock.is_pending() => {
                if runtime.active.as_ref().map(|active| &active.auth_socket_path) != Some(&connection.generation) {
                    tracing::debug!("discarding auth request from an inactive lock generation");
                    continue;
                }
                handle_auth_message(
                    ClientMessageContext {
                        username: &username,
                        visuals: &runtime.loaded_config.config.visuals,
                        auth_sender: runtime.active.as_ref().map(|active| &active.auth_sender),
                        auth_state: &mut runtime.auth_state,
                        suspend_state: &mut runtime.suspend_state,
                        manager_proxy: &manager_proxy,
                        latency_report: runtime.active_latency_report,
                    },
                    connection,
                ).await;
            }
            result = receive_auth_result(auth_results), if active_present && !unlock.is_pending() => {
                let Some(result) = result else {
                    continue;
                };
                if runtime.fingerprint.should_discard_auth_result(&result) {
                    tracing::debug!("discarding fingerprint success received during sleep preparation");
                    continue;
                }

                if let Some(result) = handle_auth_result(&mut runtime.auth_state, result) {
                    startup.request_unlock();
                    if !unlock.begin(&mut runtime, Some(result)) { startup.unlock_requested = false; }
                }
            }
            result = accept_control_connection(&mut control_listener) => {
                match result {
                    Ok(stream) => connections::spawn_control_reader(
                        stream,
                        control_connection_sender.clone(),
                    ),
                    Err(error) => {
                        tracing::warn!("failed to accept daemon control connection: {error:#}");
                    }
                }
            }
            Some(request) = control_connections.recv() => {
                if let Some(request) = startup.handle_request(request, &mut runtime, &connection, &session_path, options.config_path.as_deref()).await {
                    match handle_control_message(request.stream, request.message, &options, session_path.as_str(), &mut runtime).await {
                        Ok(true) => break,
                        Ok(false) => {},
                        Err(error) => tracing::warn!("failed to handle daemon control request: {error:#}"),
                    }
                }
            }
            result = now_playing_updates.changed(), if !unlock.is_pending() => {
                if result.is_err() {
                    continue;
                }

                let snapshot = now_playing_updates.borrow().clone();
                handle_now_playing_update(
                    &runtime.state,
                    control_socket_path(&runtime.active),
                    snapshot.as_ref(),
                ).await;
            }
            _ = maintenance_tick.tick() => {
                let now = std::time::Instant::now();
                if shutdown_gate.is_requested() && !startup.is_acquiring() && !unlock.is_pending() && now >= next_session_close_check {
                    next_session_close_check = now + std::time::Duration::from_secs(5);
                    if matches!(
                        time::timeout(std::time::Duration::from_millis(200), session_proxy.state()).await,
                        Ok(Ok(state)) if state == "closing"
                    ) {
                        // A closing logind session no longer needs an interactive lock daemon.
                        tracing::info!("logind session is closing; finishing daemon shutdown");
                        break;
                    }
                }
                let suspend_decision = runtime.suspend_state.evaluate(
                    now,
                    runtime.state.is_active() && !unlock.is_pending(),
                    runtime.auth_state.in_flight(),
                    runtime.battery.current_snapshot().as_ref(),
                    runtime.now_playing.currently_playing(),
                );
                match suspend_decision {
                    suspend::SuspendDecision::Ready => {
                        runtime.suspend_state.clear_reported_skip_reason();
                        if let Some(control_socket_path) = control_socket_path(&runtime.active) {
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

                if runtime.active.is_some() && !unlock.is_pending() {
                    runtime.fingerprint.update(
                        true,
                        runtime.loaded_config.config.fingerprint.enabled,
                        runtime.loaded_config.config.fingerprint.max_failed_attempts,
                        &username,
                        runtime.active.as_ref().map(|active| active.auth_sender.clone()),
                    ).await;
                    runtime
                        .fingerprint
                        .forward_status_updates(runtime.active.as_ref().map(|active| &active.control_socket_path))
                        .await;

                    let power_status_snapshot = runtime
                        .suspend_state
                        .power_status_snapshot(now, runtime.state.is_active());
                    let power_status_changed = !runtime.power_status_sent
                        || runtime.last_power_status_snapshot != power_status_snapshot;
                    if power_status_changed
                        && let Some(control_socket_path) = control_socket_path(&runtime.active)
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
                    runtime
                        .fingerprint
                        .update(
                            false,
                            false,
                            runtime.loaded_config.config.fingerprint.max_failed_attempts,
                            &username,
                            None,
                        )
                        .await;
                    runtime.last_power_status_snapshot = None;
                    runtime.power_status_sent = false;
                }

                if !startup.is_pending() && !unlock.is_pending() && let Some(trigger) = auto_reload_watcher.poll(options.config_path.as_deref(), &runtime.loaded_config) {
                    auto_reload::handle(trigger, options.config_path.as_deref(), &mut runtime).await;
                }
            }
            _ = sigint.recv() => {
                tracing::info!("received SIGINT");
                if shutdown_gate.request(runtime.state, runtime.active.is_some()) {
                    break;
                }
                next_session_close_check = std::time::Instant::now();
                tracing::info!("deferring daemon shutdown until the active lock is released");
            }
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM");
                if shutdown_gate.request(runtime.state, runtime.active.is_some()) {
                    break;
                }
                next_session_close_check = std::time::Instant::now();
                tracing::info!("deferring daemon shutdown until the active lock is released");
            }
        }
    }

    runtime.fingerprint.stop().await;
    let (auth_policy, slots) = runtime.slots_with_policy();
    shutdown_runtime(&session_proxy, slots, auth_policy).await;

    let _ = std::fs::remove_file(&daemon_control_socket_path);
    tracing::info!("daemon exiting");
    Ok(())
}
