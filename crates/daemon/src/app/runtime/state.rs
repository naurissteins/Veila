use std::future::pending;

use anyhow::{Context, Result};
use tokio::{
    net::{UnixListener, UnixStream},
    sync::mpsc::UnboundedReceiver,
};

use crate::{
    adapters::{
        ipc, logind,
        process::{CurtainExit, CurtainHandle},
    },
    domain::auth::{AuthPolicy, AuthState},
};

use super::{active::ActiveLock, auth::AuthResult};

pub(crate) struct LockActivation {
    pub(crate) active: ActiveLock,
    pub(crate) readiness: super::activation::RichReadiness,
}

pub(crate) fn reset_runtime(
    active: &mut Option<ActiveLock>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
) {
    if let Some(active) = active.take() {
        active.clear_sockets();
    }
    *auth_state = AuthState::new(auth_policy);
}

pub(crate) async fn wait_for_curtain_exit(
    curtain: Option<&mut CurtainHandle>,
) -> Result<CurtainExit> {
    match curtain {
        Some(child) => child
            .wait()
            .await
            .context("failed while waiting for curtain process"),
        None => pending().await,
    }
}

pub(crate) async fn update_locked_hint(session_proxy: &logind::SessionProxy<'_>, locked: bool) {
    if let Err(error) = session_proxy.set_locked_hint(locked).await {
        if is_locked_hint_not_supported(&error) {
            tracing::debug!(locked, "logind LockedHint is not supported: {error}");
        } else {
            tracing::warn!(locked, "failed to update logind LockedHint: {error}");
        }
    }
}

fn is_locked_hint_not_supported(error: &zbus::Error) -> bool {
    matches!(
        error,
        zbus::Error::MethodError(name, _, _)
            if name.as_str() == "org.freedesktop.DBus.Error.NotSupported"
    )
}

pub(crate) async fn accept_auth_connection(
    auth_listener: Option<&mut UnixListener>,
) -> Result<UnixStream> {
    match auth_listener {
        Some(listener) => ipc::accept_verified(listener, "auth").await,
        None => pending().await,
    }
}

pub(crate) async fn accept_control_connection(
    control_listener: &mut UnixListener,
) -> Result<UnixStream> {
    ipc::accept_verified(control_listener, "daemon control").await
}

pub(crate) async fn receive_auth_result(
    auth_results: Option<&mut UnboundedReceiver<AuthResult>>,
) -> Option<AuthResult> {
    match auth_results {
        Some(receiver) => receiver.recv().await,
        None => pending().await,
    }
}
