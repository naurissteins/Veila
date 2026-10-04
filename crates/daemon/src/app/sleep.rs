use super::runtime::control_socket_path;

use std::path::Path;

use zbus::zvariant::OwnedFd;

use crate::adapters::{logind, process};

use super::{startup::PendingStartup, state::AppRuntime};

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
    startup: &mut PendingStartup,
    connection: &zbus::Connection,
    session_path: &zbus::zvariant::OwnedObjectPath,
    manager: &logind::ManagerProxy<'_>,
    config_path: Option<&Path>,
) {
    if start {
        if runtime.sleep_lock.enabled {
            startup.begin(
                "sleep",
                runtime,
                connection,
                session_path,
                config_path,
                false,
                veila_common::ipc::LatencyReportMode::Disabled,
            );
        }
        if startup.is_acquiring() || startup.unlock_requested {
            startup.deferred_sleep = Some(true);
            return;
        }
        prepare_for_sleep(runtime).await;
    } else {
        if (startup.is_acquiring() || startup.unlock_requested) && startup.deferred_sleep.is_some()
        {
            startup.deferred_sleep = Some(false);
            return;
        }
        runtime.sleep_lock.arm(manager).await;
        resume_after_sleep(runtime).await;
    }
}

pub(super) async fn finish_deferred_sleep(
    startup: &mut PendingStartup,
    runtime: &mut AppRuntime,
    manager: &logind::ManagerProxy<'_>,
) {
    // Preserve guard/release/resume order even if logind's delay expires during acquisition.
    if let Some(still_sleeping) = startup.deferred_sleep.take() {
        prepare_for_sleep(runtime).await;
        if !still_sleeping {
            runtime.sleep_lock.arm(manager).await;
            resume_after_sleep(runtime).await;
        }
    }
}

async fn prepare_for_sleep(runtime: &mut AppRuntime) {
    if runtime.state.is_active()
        && let Some(control_socket_path) = control_socket_path(&runtime.active)
        && let Err(error) =
            process::request_curtain_arm_resume_input_guard(control_socket_path).await
    {
        tracing::warn!("failed to arm curtain resume input guard before sleep: {error:#}");
    }
    runtime.fingerprint.pause_for_sleep().await;
    runtime
        .fingerprint
        .forward_status_updates(
            runtime
                .active
                .as_ref()
                .map(|active| &active.control_socket_path),
        )
        .await;

    // Releasing the delay inhibitor is what lets logind continue into suspend.
    runtime.sleep_lock.inhibitor = None;
}

async fn resume_after_sleep(runtime: &mut AppRuntime) {
    runtime.fingerprint.resume_after_sleep();
    if runtime.state.is_active()
        && let Some(control_socket_path) = control_socket_path(&runtime.active)
        && let Err(error) = process::request_curtain_mark_resumed(control_socket_path).await
    {
        tracing::warn!("failed to mark curtain as resumed after sleep: {error:#}");
    }
}
