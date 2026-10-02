use std::path::{Path, PathBuf};

use tokio::{
    net::UnixListener,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

use super::auth::AuthResult;
use crate::adapters::process::CurtainHandle;

pub(crate) struct ActiveLock {
    pub(crate) curtain: CurtainHandle,
    pub(crate) auth_listener: UnixListener,
    pub(crate) auth_socket_path: PathBuf,
    pub(crate) control_socket_path: PathBuf,
    pub(crate) auth_results: UnboundedReceiver<AuthResult>,
    pub(crate) auth_sender: UnboundedSender<AuthResult>,
}

impl ActiveLock {
    pub(super) fn clear_sockets(self) {
        // Unresolved exits must retain the separately managed ownership record.
        drop(self.auth_listener);
        drop(self.auth_results);
        drop(self.auth_sender);
        drop(self.curtain);
        let _ = std::fs::remove_file(self.auth_socket_path);
        let _ = std::fs::remove_file(self.control_socket_path);
    }
}

pub(crate) fn control_socket_path(active: &Option<ActiveLock>) -> Option<&Path> {
    active
        .as_ref()
        .map(|active| active.control_socket_path.as_path())
}

#[cfg(test)]
pub(crate) mod tests;
