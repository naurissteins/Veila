use std::{
    io::{BufRead, BufReader},
    os::unix::fs::{MetadataExt, PermissionsExt},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    sync::mpsc::Sender,
    thread,
    time::Duration,
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
const ACCEPT_BACKOFF_MIN: Duration = Duration::from_millis(50);
const ACCEPT_BACKOFF_MAX: Duration = Duration::from_secs(1);
const CONTROL_READ_TIMEOUT: Duration = Duration::from_secs(5);

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

    thread::spawn(move || run_listener(listener, owner_uid, sender));

    Ok(())
}

/// Runs until an unlock is delivered or the curtain drops the receiver
fn run_listener(listener: UnixListener, owner_uid: u32, sender: Sender<ControlEvent>) {
    let mut accept_backoff = ACCEPT_BACKOFF_MIN;

    loop {
        let stream = match listener.accept() {
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

        let message = match read_control_message(stream, owner_uid) {
            Ok(Some(message)) => message,
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!("ignoring invalid curtain control connection: {error:#}");
                continue;
            }
        };

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
    stream: UnixStream,
    owner_uid: u32,
) -> Result<Option<CurtainControlMessage>> {
    let peer =
        getsockopt(&stream, PeerCredentials).context("failed to read control peer credentials")?;
    if peer.uid() != owner_uid {
        bail!(
            "rejected curtain control connection from uid {}, expected socket owner uid {owner_uid}",
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

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        os::unix::net::{UnixListener, UnixStream},
        path::{Path, PathBuf},
        sync::mpsc::channel,
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use nix::unistd::Uid;
    use veila_common::ipc::{CurtainControlMessage, encode_message};

    use super::{ControlEvent, run_listener};

    const RECV_TIMEOUT: Duration = Duration::from_secs(5);

    fn unique_socket_path(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "veila-test-{label}-{}-{stamp}.sock",
            std::process::id()
        ))
    }

    fn send_payload(path: &Path, payload: &str) {
        let mut stream = UnixStream::connect(path).expect("connect to control socket");
        stream.write_all(payload.as_bytes()).expect("write payload");
        stream.flush().expect("flush payload");
    }

    fn encoded_unlock(attempt_id: u64) -> String {
        let encoded = encode_message(&CurtainControlMessage::Unlock {
            attempt_id: Some(attempt_id),
        })
        .expect("encode unlock");
        format!("{encoded}\n")
    }

    #[test]
    fn delivers_unlock_after_malformed_connections() {
        let path = unique_socket_path("control-resilience");
        let listener = UnixListener::bind(&path).expect("bind control socket");
        let (sender, receiver) = channel();
        let handle = thread::spawn({
            let owner_uid = Uid::effective().as_raw();
            move || run_listener(listener, owner_uid, sender)
        });

        // Each of these previously killed the listener thread for the rest of the lock session
        send_payload(&path, "not json at all\n");
        send_payload(&path, "{\"Unlock\": \n");
        send_payload(&path, "truncated without newline");
        UnixStream::connect(&path).expect("connect and close without sending");

        send_payload(&path, &encoded_unlock(7));

        let event = receiver
            .recv_timeout(RECV_TIMEOUT)
            .expect("unlock must still be delivered after malformed connections");
        assert_eq!(
            event,
            ControlEvent::Unlock {
                attempt_id: Some(7)
            }
        );

        handle.join().expect("listener thread should exit cleanly");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn keeps_serving_updates_before_unlock() {
        let path = unique_socket_path("control-continuity");
        let listener = UnixListener::bind(&path).expect("bind control socket");
        let (sender, receiver) = channel();
        let handle = thread::spawn({
            let owner_uid = Uid::effective().as_raw();
            move || run_listener(listener, owner_uid, sender)
        });

        let reload = encode_message(&CurtainControlMessage::ReloadConfig).expect("encode reload");
        send_payload(&path, &format!("{reload}\n"));
        assert_eq!(
            receiver.recv_timeout(RECV_TIMEOUT).expect("reload event"),
            ControlEvent::Reload
        );

        send_payload(&path, "}{ still not json\n");
        send_payload(&path, &encoded_unlock(1));

        assert_eq!(
            receiver.recv_timeout(RECV_TIMEOUT).expect("unlock event"),
            ControlEvent::Unlock {
                attempt_id: Some(1)
            }
        );

        handle.join().expect("listener thread should exit cleanly");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn stops_when_curtain_receiver_is_gone() {
        let path = unique_socket_path("control-shutdown");
        let listener = UnixListener::bind(&path).expect("bind control socket");
        let (sender, receiver) = channel::<ControlEvent>();
        let handle = thread::spawn({
            let owner_uid = Uid::effective().as_raw();
            move || run_listener(listener, owner_uid, sender)
        });

        drop(receiver);
        let reload = encode_message(&CurtainControlMessage::ReloadConfig).expect("encode reload");
        send_payload(&path, &format!("{reload}\n"));

        handle
            .join()
            .expect("listener thread should stop once the receiver is dropped");
        std::fs::remove_file(&path).ok();
    }
}
