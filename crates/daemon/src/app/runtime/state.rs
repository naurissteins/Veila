use std::{future::pending, path::PathBuf, process::ExitStatus};

use anyhow::{Context, Result};
use tokio::{
    net::{UnixListener, UnixStream},
    process::Child,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::{
    adapters::logind,
    domain::auth::{AuthPolicy, AuthState},
};

use super::auth::AuthResult;

pub(crate) struct LockActivation {
    pub(super) curtain: Child,
    pub(super) auth_listener: UnixListener,
    pub(super) auth_socket_path: PathBuf,
    pub(super) control_socket_path: PathBuf,
    pub(super) auth_results: UnboundedReceiver<AuthResult>,
    pub(super) auth_sender: UnboundedSender<AuthResult>,
}

pub(crate) struct ActiveRuntime<'a> {
    pub(super) curtain: &'a mut Option<Child>,
    pub(super) auth_listener: &'a mut Option<UnixListener>,
    pub(super) auth_socket_path: &'a mut Option<PathBuf>,
    pub(super) control_socket_path: &'a mut Option<PathBuf>,
    pub(super) auth_results: &'a mut Option<UnboundedReceiver<AuthResult>>,
    pub(super) auth_sender: &'a mut Option<UnboundedSender<AuthResult>>,
}

impl<'a> ActiveRuntime<'a> {
    pub(crate) fn new(
        curtain: &'a mut Option<Child>,
        auth_listener: &'a mut Option<UnixListener>,
        auth_socket_path: &'a mut Option<PathBuf>,
        control_socket_path: &'a mut Option<PathBuf>,
        auth_results: &'a mut Option<UnboundedReceiver<AuthResult>>,
        auth_sender: &'a mut Option<UnboundedSender<AuthResult>>,
    ) -> Self {
        Self {
            curtain,
            auth_listener,
            auth_socket_path,
            control_socket_path,
            auth_results,
            auth_sender,
        }
    }

    pub(crate) fn install_activation(self, activation: LockActivation) {
        *self.curtain = Some(activation.curtain);
        *self.auth_listener = Some(activation.auth_listener);
        *self.auth_socket_path = Some(activation.auth_socket_path);
        *self.control_socket_path = Some(activation.control_socket_path);
        *self.auth_results = Some(activation.auth_results);
        *self.auth_sender = Some(activation.auth_sender);
    }
}

pub(crate) fn reset_runtime(
    auth_listener: &mut Option<UnixListener>,
    auth_socket_path: &mut Option<PathBuf>,
    control_socket_path: &mut Option<PathBuf>,
    auth_results: &mut Option<UnboundedReceiver<AuthResult>>,
    auth_sender: &mut Option<UnboundedSender<AuthResult>>,
    auth_policy: AuthPolicy,
    auth_state: &mut AuthState,
) {
    auth_listener.take();
    auth_results.take();
    auth_sender.take();
    if let Some(path) = auth_socket_path.take() {
        let _ = std::fs::remove_file(path);
    }
    if let Some(path) = control_socket_path.take() {
        let _ = std::fs::remove_file(path);
    }
    *auth_state = AuthState::new(auth_policy);
}

pub(crate) async fn wait_for_curtain_exit(curtain: &mut Option<Child>) -> Result<ExitStatus> {
    match curtain.as_mut() {
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
    auth_listener: &mut Option<UnixListener>,
) -> Result<UnixStream> {
    match auth_listener.as_mut() {
        Some(listener) => listener
            .accept()
            .await
            .map(|(stream, _)| stream)
            .context("failed to accept auth connection"),
        None => pending().await,
    }
}

pub(crate) async fn accept_control_connection(
    control_listener: &mut UnixListener,
) -> Result<UnixStream> {
    control_listener
        .accept()
        .await
        .map(|(stream, _)| stream)
        .context("failed to accept daemon control connection")
}

pub(crate) async fn receive_auth_result(
    auth_results: &mut Option<UnboundedReceiver<AuthResult>>,
) -> Option<AuthResult> {
    match auth_results.as_mut() {
        Some(receiver) => receiver.recv().await,
        None => pending().await,
    }
}
