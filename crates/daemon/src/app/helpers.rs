use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use nix::unistd::{Uid, User};
use veila_common::ipc::{
    DaemonControlResponse, DaemonHealth, DaemonReloadStatus, DaemonStatus, LiveReloadStatus,
};
use veila_common::{AppConfig, LoadedConfig};

use crate::{
    DaemonOptions,
    adapters::{logind, process},
    domain::{
        auth::{AuthPolicy, AuthState},
        lock_state::LockState,
    },
};
use tokio::process::Child;

use super::{
    prewarm,
    runtime::{ActiveRuntime, activate_lock, activate_lock_via_standby},
};

pub(super) async fn activate_and_install(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    config_path: Option<&std::path::Path>,
    runtime: ActiveRuntime<'_>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
) -> Result<()> {
    let activation = activate_lock(session_proxy, state, config_path).await?;
    runtime.install_activation(activation);
    *auth_state = AuthState::new(auth_policy);
    Ok(())
}

/// Tries to activate the lock using a pre-warmed standby curtain
pub(super) async fn try_activate_and_install(
    session_proxy: &logind::SessionProxy<'_>,
    state: &mut LockState,
    standby: Option<(Child, PathBuf)>,
    config_path: Option<&std::path::Path>,
    runtime: ActiveRuntime<'_>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
) -> Result<()> {
    if let Some((standby_child, standby_socket)) = standby {
        match activate_lock_via_standby(session_proxy, state, standby_child, &standby_socket).await
        {
            Ok(activation) => {
                runtime.install_activation(activation);
                *auth_state = AuthState::new(auth_policy);
                return Ok(());
            }
            Err(error) => {
                tracing::warn!(
                    "standby curtain activation failed ({error:#}); trying fresh curtain"
                );
            }
        }
    }

    let activation = activate_lock(session_proxy, state, config_path).await?;
    runtime.install_activation(activation);
    *auth_state = AuthState::new(auth_policy);
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
    config_path: Option<&Path>,
) -> DaemonStatus {
    DaemonStatus {
        state: state.to_string(),
        session: session.to_string(),
        active_lock: state.is_active(),
        curtain_running,
        live_reload_available: state.is_active()
            && curtain_running
            && control_socket_path.is_some(),
        config_path: config_path.map(|path| path.display().to_string()),
    }
}

pub(super) fn build_daemon_health() -> DaemonHealth {
    crate::local_build_info()
}

pub(super) async fn reload_config_response(
    options: &DaemonOptions,
    state: &LockState,
    control_socket_path: Option<&Path>,
    loaded_config: &mut LoadedConfig,
    auth_policy: &mut AuthPolicy,
    auth_state: &mut AuthState,
) -> DaemonControlResponse {
    match AppConfig::load(options.config_path.as_deref()) {
        Ok(new_loaded_config) => {
            *loaded_config = new_loaded_config;
            *auth_policy = AuthPolicy::new(
                Duration::from_millis(loaded_config.config.lock.auth_backoff_base_ms),
                Duration::from_secs(loaded_config.config.lock.auth_backoff_max_seconds),
            );
            if !state.is_active() {
                *auth_state = AuthState::new(*auth_policy);
            }
            prewarm::spawn_background_prewarm(&loaded_config.config);

            let live_reload = if !state.is_active() {
                Ok(LiveReloadStatus::NotActive)
            } else if let Some(control_socket_path) = control_socket_path {
                process::request_curtain_reload(control_socket_path)
                    .await
                    .map_err(|error| {
                        format!("failed to forward live config reload to curtain: {error:#}")
                    })
                    .map(|_| LiveReloadStatus::Forwarded)
            } else {
                Err(
                    "failed to forward live config reload to curtain: active lock has no control socket"
                        .to_string(),
                )
            };

            match live_reload {
                Ok(live_reload) => {
                    tracing::info!(
                        active_lock = state.is_active(),
                        live_reload = match live_reload {
                            LiveReloadStatus::NotActive => "not-active",
                            LiveReloadStatus::Forwarded => "forwarded",
                        },
                        config = loaded_config
                            .path
                            .as_deref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| "defaults".to_string()),
                        "reloaded daemon config"
                    );
                    DaemonControlResponse::Reloaded(DaemonReloadStatus {
                        config_path: loaded_config
                            .path
                            .as_deref()
                            .map(|path| path.display().to_string()),
                        active_lock: state.is_active(),
                        live_reload,
                    })
                }
                Err(reason) => {
                    tracing::warn!("{reason}");
                    DaemonControlResponse::Error { reason }
                }
            }
        }
        Err(error) => DaemonControlResponse::Error {
            reason: format!("failed to reload daemon config: {error:#}"),
        },
    }
}
