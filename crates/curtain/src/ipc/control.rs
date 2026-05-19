use std::{
    io::{BufRead, BufReader},
    os::unix::fs::{MetadataExt, PermissionsExt},
    os::unix::net::UnixListener,
    path::PathBuf,
    sync::mpsc::Sender,
    thread,
};

use anyhow::{Context, Result, bail};
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use veila_common::ipc::{CurtainControlMessage, decode_message};
use veila_common::{FingerprintStatus, NowPlayingSnapshot, ipc::LockPowerStatusSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ControlEvent {
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
const IPC_MAX_LINE_BYTES: usize = 64 * 1024;

pub(crate) fn spawn_listener(socket_path: PathBuf, sender: Sender<ControlEvent>) -> Result<()> {
    if socket_path.exists() {
        std::fs::remove_file(&socket_path).with_context(|| {
            format!(
                "failed to remove stale control socket {}",
                socket_path.display()
            )
        })?;
    }

    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("failed to bind control socket {}", socket_path.display()))?;
    std::fs::set_permissions(
        &socket_path,
        std::fs::Permissions::from_mode(CONTROL_SOCKET_MODE),
    )
    .with_context(|| {
        format!(
            "failed to restrict control socket {}",
            socket_path.display()
        )
    })?;
    let owner_uid = std::fs::metadata(&socket_path)
        .with_context(|| format!("failed to inspect control socket {}", socket_path.display()))?
        .uid();

    thread::spawn(move || {
        if let Err(error) = run_listener(listener, owner_uid, sender) {
            tracing::warn!("curtain control listener exited with error: {error:#}");
        }
    });

    Ok(())
}

fn run_listener(
    listener: UnixListener,
    owner_uid: u32,
    sender: Sender<ControlEvent>,
) -> Result<()> {
    loop {
        let (stream, _) = listener
            .accept()
            .context("failed to accept control connection")?;
        let peer = getsockopt(&stream, PeerCredentials)
            .context("failed to read control peer credentials")?;
        if peer.uid() != owner_uid {
            tracing::warn!(
                peer_uid = peer.uid(),
                expected_uid = owner_uid,
                "rejected curtain control connection from unexpected uid"
            );
            continue;
        }

        let mut reader = BufReader::new(stream);
        let Some(line) = read_bounded_line(&mut reader, "control message")? else {
            continue;
        };

        match decode_message::<CurtainControlMessage>(&line)
            .context("invalid curtain control message")?
        {
            CurtainControlMessage::Unlock { attempt_id } => {
                let _ = sender.send(ControlEvent::Unlock { attempt_id });
                return Ok(());
            }
            CurtainControlMessage::ReloadConfig => {
                let _ = sender.send(ControlEvent::Reload);
            }
            CurtainControlMessage::ArmResumeInputGuard => {
                let _ = sender.send(ControlEvent::ArmResumeInputGuard);
            }
            CurtainControlMessage::MarkResumed => {
                let _ = sender.send(ControlEvent::MarkResumed);
            }
            CurtainControlMessage::UpdateNowPlaying { snapshot } => {
                let _ = sender.send(ControlEvent::UpdateNowPlaying { snapshot });
            }
            CurtainControlMessage::UpdatePowerStatus { snapshot } => {
                let _ = sender.send(ControlEvent::UpdatePowerStatus { snapshot });
            }
            CurtainControlMessage::UpdateFingerprintStatus { status } => {
                let _ = sender.send(ControlEvent::UpdateFingerprintStatus { status });
            }
        }
    }
}

fn read_bounded_line<R: BufRead>(reader: &mut R, label: &str) -> Result<Option<String>> {
    let mut line = Vec::new();

    loop {
        let buffer = reader
            .fill_buf()
            .with_context(|| format!("failed to read {label}"))?;
        if buffer.is_empty() {
            if line.is_empty() {
                return Ok(None);
            }
            bail!("{label} ended before newline");
        }

        let line_end = buffer.iter().position(|byte| *byte == b'\n');
        let consumed = line_end.unwrap_or(buffer.len());
        if line.len() + consumed > IPC_MAX_LINE_BYTES {
            bail!("{label} exceeds {IPC_MAX_LINE_BYTES} bytes");
        }

        line.extend_from_slice(&buffer[..consumed]);
        reader.consume(consumed + usize::from(line_end.is_some()));

        if line_end.is_some() {
            return String::from_utf8(line)
                .map(Some)
                .with_context(|| format!("{label} is not UTF-8"));
        }
    }
}
