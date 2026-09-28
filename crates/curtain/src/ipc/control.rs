use std::{
    io::{BufReader, Write},
    os::unix::fs::PermissionsExt,
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
        mpsc::Sender,
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use calloop::ping::Ping;

use super::read_bounded_line;
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use nix::unistd::Uid;
use veila_common::ipc::{
    CurtainControlMessage, CurtainControlResponse, CurtainLockState, decode_message, encode_message,
};
use veila_common::{FingerprintStatus, NowPlayingSnapshot, ipc::LockPowerStatusSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ControlEvent {
    Probe,
    Unlock {
        attempt_id: Option<u64>,
    },
    Reload,
    ArmResumeInputGuard,
    MarkResumed,
    UpdateNowPlaying {
        snapshot: Option<NowPlayingSnapshot>,
    },
    UpdatePowerStatus {
        snapshot: Option<LockPowerStatusSnapshot>,
    },
    UpdateFingerprintStatus {
        status: Option<FingerprintStatus>,
    },
}

const CONTROL_SOCKET_MODE: u32 = 0o600;
const ACCEPT_BACKOFF_MIN: Duration = Duration::from_millis(50);
const ACCEPT_BACKOFF_MAX: Duration = Duration::from_secs(1);
const CONTROL_READ_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) struct ControlSender {
    sender: Sender<ControlEvent>,
    ping: Ping,
}

impl ControlSender {
    pub(crate) fn new(sender: Sender<ControlEvent>, ping: Ping) -> Self {
        Self { sender, ping }
    }

    fn send(
        &self,
        event: ControlEvent,
    ) -> std::result::Result<(), std::sync::mpsc::SendError<ControlEvent>> {
        self.sender.send(event)?;
        self.ping.ping();
        Ok(())
    }
}

pub(crate) fn spawn_listener(
    socket_path: PathBuf,
    sender: ControlSender,
    state: Arc<AtomicU8>,
) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).with_context(|| {
            format!(
                "failed to remove stale control socket {}",
                socket_path.display()
            )
        })?;
    }

    let listener = bind_secured(&socket_path)?;
    let owner_uid = Uid::effective().as_raw();

    thread::spawn(move || run_listener(listener, owner_uid, sender, state));

    Ok(())
}

fn bind_secured(socket_path: &std::path::Path) -> Result<UnixListener> {
    let mut name = std::ffi::OsString::from(".");
    name.push(
        socket_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("control.sock")),
    );
    name.push(format!(".{}.staging", std::process::id()));
    let staging = socket_path.with_file_name(name);
    let _ = std::fs::remove_file(&staging);

    let listener = UnixListener::bind(&staging)
        .with_context(|| format!("failed to bind control socket {}", staging.display()))?;
    if let Err(error) = std::fs::set_permissions(
        &staging,
        std::fs::Permissions::from_mode(CONTROL_SOCKET_MODE),
    ) {
        let _ = std::fs::remove_file(&staging);
        return Err(error)
            .with_context(|| format!("failed to restrict control socket {}", staging.display()));
    }
    if let Err(error) = std::fs::rename(&staging, socket_path) {
        let _ = std::fs::remove_file(&staging);
        return Err(error).with_context(|| {
            format!(
                "failed to publish control socket at {}",
                socket_path.display()
            )
        });
    }

    Ok(listener)
}

/// Runs until an unlock is delivered or the curtain drops the receiver
fn run_listener(
    listener: UnixListener,
    owner_uid: u32,
    sender: ControlSender,
    state: Arc<AtomicU8>,
) {
    let mut accept_backoff = ACCEPT_BACKOFF_MIN;

    loop {
        let mut stream = match listener.accept() {
            Ok((stream, _)) => {
                accept_backoff = ACCEPT_BACKOFF_MIN;
                stream
            }
            Err(error) => {
                tracing::warn!(
                    retry_in_ms = accept_backoff.as_millis().min(u128::from(u64::MAX)) as u64,
                    "failed to accept curtain control connection; retrying: {error}"
                );
                thread::sleep(accept_backoff);
                accept_backoff = accept_backoff.saturating_mul(2).min(ACCEPT_BACKOFF_MAX);
                continue;
            }
        };

        let message = match read_control_message(&mut stream, owner_uid) {
            Ok(Some(message)) => message,
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!("ignoring invalid curtain control connection: {error:#}");
                continue;
            }
        };

        if matches!(message, CurtainControlMessage::Probe) {
            let lock_state = match state.load(Ordering::Acquire) {
                1 => CurtainLockState::Locked,
                2 => CurtainLockState::Finished,
                _ => CurtainLockState::Starting,
            };
            let response = CurtainControlResponse::Status { state: lock_state };
            if let Ok(mut payload) = encode_message(&response) {
                payload.push('\n');
                let _ = stream.write_all(payload.as_bytes());
            }
            continue;
        }

        let unlock_requested = matches!(message, CurtainControlMessage::Unlock { .. });
        if sender.send(control_event(message)).is_err() {
            tracing::debug!("curtain control receiver is gone; stopping control listener");
            return;
        }

        if unlock_requested {
            return;
        }
    }
}

fn read_control_message(
    stream: &mut UnixStream,
    owner_uid: u32,
) -> Result<Option<CurtainControlMessage>> {
    let peer =
        getsockopt(&stream, PeerCredentials).context("failed to read control peer credentials")?;
    if peer.uid() != owner_uid {
        bail!(
            "rejected curtain control connection from uid {}, expected curtain uid {owner_uid}",
            peer.uid()
        );
    }

    stream
        .set_read_timeout(Some(CONTROL_READ_TIMEOUT))
        .context("failed to set control read timeout")?;

    let mut reader = BufReader::new(stream);
    let Some(line) = read_bounded_line(&mut reader, "control message")? else {
        return Ok(None);
    };

    decode_message(&line)
        .map(Some)
        .context("invalid curtain control message")
}

fn control_event(message: CurtainControlMessage) -> ControlEvent {
    match message {
        CurtainControlMessage::Probe => ControlEvent::Probe,
        CurtainControlMessage::Unlock { attempt_id } => ControlEvent::Unlock { attempt_id },
        CurtainControlMessage::ReloadConfig => ControlEvent::Reload,
        CurtainControlMessage::ArmResumeInputGuard => ControlEvent::ArmResumeInputGuard,
        CurtainControlMessage::MarkResumed => ControlEvent::MarkResumed,
        CurtainControlMessage::UpdateNowPlaying { snapshot } => {
            ControlEvent::UpdateNowPlaying { snapshot }
        }
        CurtainControlMessage::UpdatePowerStatus { snapshot } => {
            ControlEvent::UpdatePowerStatus { snapshot }
        }
        CurtainControlMessage::UpdateFingerprintStatus { status } => {
            ControlEvent::UpdateFingerprintStatus { status }
        }
    }
}

#[cfg(test)]
mod tests;
