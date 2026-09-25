use std::path::Path;

use zbus::zvariant::OwnedFd;

use crate::adapters::{logind, process};

use super::{idle::activate_triggered_lock, state::AppRuntime};

/// Holds a logind sleep delay inhibitor while lock-before-sleep is enabled.
#[derive(Default)]
pub(super) struct SleepLockInhibitor {
    enabled: bool,
    inhibitor: Option<OwnedFd>,
}

impl SleepLockInhibitor {
    pub(super) async fn sync(&mut self, manager: &logind::ManagerProxy<'_>, enabled: bool) {
        if enabled == self.enabled {
            return;
        }

        self.enabled = enabled;
        if enabled {
            self.arm(manager).await;
        } else {
            self.inhibitor = None;
            tracing::info!("lock before sleep disabled");
        }
    }

    async fn arm(&mut self, manager: &logind::ManagerProxy<'_>) {
        if !self.enabled || self.inhibitor.is_some() {
            return;
        }

        match manager
            .inhibit("sleep", "Veila", "Lock the session before sleep", "delay")
            .await
        {
            Ok(inhibitor) => {
                self.inhibitor = Some(inhibitor);
                tracing::debug!("lock before sleep armed with a logind delay inhibitor");
            }
            Err(error) => tracing::warn!(
                "failed to take a logind sleep delay inhibitor; lock before sleep may race suspend: {error}"
            ),
        }
    }
}

pub(super) async fn handle_prepare_for_sleep(
    start: bool,
    runtime: &mut AppRuntime,
    session_proxy: &logind::SessionProxy<'_>,
    manager: &logind::ManagerProxy<'_>,
    config_path: Option<&Path>,
) {
    if start {
        prepare_for_sleep(runtime, session_proxy, config_path).await;
    } else {
        runtime.sleep_lock.arm(manager).await;
        resume_after_sleep(runtime).await;
    }
}

async fn prepare_for_sleep(
    runtime: &mut AppRuntime,
    session_proxy: &logind::SessionProxy<'_>,
    config_path: Option<&Path>,
) {
    if runtime.sleep_lock.enabled && !runtime.state.is_active() {
        tracing::info!("locking before sleep");
        activate_triggered_lock("sleep", runtime, session_proxy, config_path).await;
    }

    if runtime.state.is_active()
        && let Some(control_socket_path) = runtime.control_socket_path.as_deref()
        && let Err(error) =
            process::request_curtain_arm_resume_input_guard(control_socket_path).await
    {
        tracing::warn!("failed to arm curtain resume input guard before sleep: {error:#}");
    }
    runtime.fingerprint.pause_for_sleep().await;
    runtime
        .fingerprint
        .forward_status_updates(runtime.control_socket_path.as_ref())
        .await;

    // Releasing the delay inhibitor is what lets logind continue into suspend.
    runtime.sleep_lock.inhibitor = None;
}

async fn resume_after_sleep(runtime: &mut AppRuntime) {
    runtime.fingerprint.resume_after_sleep();
    if runtime.state.is_active()
        && let Some(control_socket_path) = runtime.control_socket_path.as_deref()
        && let Err(error) = process::request_curtain_mark_resumed(control_socket_path).await
    {
        tracing::warn!("failed to mark curtain as resumed after sleep: {error:#}");
    }
}
