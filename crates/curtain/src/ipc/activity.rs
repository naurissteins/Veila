use std::{
    io::Write,
    os::{fd::AsRawFd, unix::net::UnixStream},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use nix::sys::socket::{AddressFamily, SockFlag, SockType, UnixAddr, connect, socket};
use veila_common::ipc::{ClientMessage, encode_message};

use super::auth::verify_socket_peer;

// stay below the shortest supported suspend delay, even during continuous input
const NOTIFICATION_INTERVAL: Duration = Duration::from_millis(250);

pub(crate) struct ActivityNotifier {
    socket_path: Option<PathBuf>,
    sender: Option<SyncSender<()>>,
}

impl ActivityNotifier {
    pub(crate) fn new(socket_path: Option<PathBuf>) -> Self {
        Self {
            socket_path,
            sender: None,
        }
    }

    pub(crate) fn notify(&mut self) {
        if self.sender.is_none() {
            let Some(path) = self.socket_path.clone() else {
                return;
            };
            let (sender, receiver) = mpsc::sync_channel(1);
            match thread::Builder::new()
                .name("veila-activity".into())
                .spawn(move || {
                    run_worker(receiver, NOTIFICATION_INTERVAL, || {
                        if let Err(error) = send_activity(&path) {
                            tracing::debug!(
                                "failed to notify daemon about lock activity: {error:#}"
                            );
                        }
                    });
                }) {
                Ok(_) => self.sender = Some(sender),
                Err(error) => {
                    tracing::warn!("failed to start curtain activity worker: {error}");
                    return;
                }
            }
        }
        if let Some(sender) = &self.sender {
            // full queue already guarantees a trailing update, input must never wait for IPC
            let _ = sender.try_send(());
        }
    }
}

fn run_worker(receiver: Receiver<()>, interval: Duration, mut notify: impl FnMut()) {
    while receiver.recv().is_ok() {
        notify();
        let mut deadline = Instant::now() + interval;
        let mut pending = false;
        loop {
            if pending && Instant::now() >= deadline {
                notify();
                pending = false;
                deadline = Instant::now() + interval;
            }
            match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(()) => pending = true,
                Err(RecvTimeoutError::Timeout) if !pending => break,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }
}

fn send_activity(path: &Path) -> Result<()> {
    // nonblocking connect also bounds failures when the daemon's accept backlog is full
    let fd = socket(
        AddressFamily::Unix,
        SockType::Stream,
        SockFlag::SOCK_NONBLOCK | SockFlag::SOCK_CLOEXEC,
        None,
    )?;
    connect(fd.as_raw_fd(), &UnixAddr::new(path)?)?;
    let mut stream = UnixStream::from(fd);
    verify_socket_peer(&stream).context("activity socket peer rejected")?;
    let mut payload = encode_message(&ClientMessage::Activity)?;
    payload.push('\n');
    stream.write_all(payload.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests;
