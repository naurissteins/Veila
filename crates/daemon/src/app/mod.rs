mod battery;
mod events;
mod helpers;
mod mpris;
mod output_probe;
mod prewarm;
mod runtime;
mod state;
mod weather;

use std::path::PathBuf;

use crate::{DaemonOptions, adapters::logind};
use anyhow::{Context, Result};
use futures_util::StreamExt;
use tokio::{
    net::UnixListener,
    signal::unix::{SignalKind, signal},
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

pub async fn run(
    options: DaemonOptions,
    mut control_listener: UnixListener,
    daemon_control_socket_path: PathBuf,
) -> Result<()> {
    let mut runtime = AppRuntime::new(
        AppConfig::load(options.config_path.as_deref()).context("failed to load daemon config")?,
    );
    prewarm::spawn_background_prewarm(&runtime.loaded_config.config);
    let connection = logind::connect_system().await?;
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
    let mut sigint =
        signal(SignalKind::interrupt()).context("failed to register SIGINT handler")?;
    let mut sigterm =
        signal(SignalKind::terminate()).context("failed to register SIGTERM handler")?;
    let mut now_playing_updates = runtime.now_playing.subscribe();

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
        activate_and_log(
            "manual",
            &session_proxy,
            &mut runtime.state,
            options.config_path.as_deref(),
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
        )
        .await
        .context("failed to activate manual lock")?;
    }

    loop {
        tokio::select! {
            Some(_) = lock_stream.next() => {
                let weather_snapshot = runtime.weather.current_snapshot();
                let battery_snapshot = runtime.battery.current_snapshot();
                let now_playing_snapshot = runtime.now_playing.current_snapshot();
                let (auth_policy, slots) = runtime.slots_with_policy();
                handle_lock_signal(
                    "logind",
                    &session_proxy,
                    options.config_path.as_deref(),
                    weather_snapshot.as_ref(),
                    battery_snapshot.as_ref(),
                    now_playing_snapshot.as_ref(),
                    slots,
                    auth_policy,
                ).await;
            }
            Some(_) = unlock_stream.next() => {
                let (auth_policy, slots) = runtime.slots_with_policy();
                handle_unlock_signal(
                    &session_proxy,
                    slots,
                    auth_policy,
                ).await;
            }
            result = wait_for_curtain_exit(&mut runtime.curtain), if runtime.curtain.is_some() => {
                let weather_snapshot = runtime.weather.current_snapshot();
                let battery_snapshot = runtime.battery.current_snapshot();
                let now_playing_snapshot = runtime.now_playing.current_snapshot();
                let (auth_policy, slots) = runtime.slots_with_policy();
                handle_curtain_exit(
                    result?,
                    &session_proxy,
                    options.config_path.as_deref(),
                    weather_snapshot.as_ref(),
                    battery_snapshot.as_ref(),
                    now_playing_snapshot.as_ref(),
                    slots,
                    auth_policy,
                ).await;
            }
            result = accept_auth_connection(&mut runtime.auth_listener), if runtime.state.is_active() && runtime.auth_listener.is_some() => {
                handle_auth_connection(&username, &runtime.auth_sender, &mut runtime.auth_state, result?).await?;
            }
            result = receive_auth_result(&mut runtime.auth_results), if runtime.auth_results.is_some() => {
                let Some(result) = result else {
                    continue;
                };

                let (auth_policy, slots) = runtime.slots_with_policy();
                handle_auth_result(
                    &session_proxy,
                    slots,
                    auth_policy,
                    result,
                ).await;
            }
            result = accept_control_connection(&mut control_listener) => {
                let weather = runtime.weather.clone();
                let battery = runtime.battery.clone();
                let weather_snapshot = weather.current_snapshot();
                let battery_snapshot = battery.current_snapshot();
                let now_playing_snapshot = runtime.now_playing.current_snapshot();
                let (loaded_config, auth_policy, slots) = runtime.control_inputs();
                if handle_control_connection(
                    result?,
                    &options,
                    &session_proxy,
                    &session_path,
                    loaded_config,
                    weather_snapshot.as_ref(),
                    battery_snapshot.as_ref(),
                    now_playing_snapshot.as_ref(),
                    &weather,
                    &battery,
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
