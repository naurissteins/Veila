use std::{
    io::{BufRead, BufReader, Write},
    os::unix::fs::MetadataExt,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    thread,
    time::Instant,
};

use anyhow::{Context, Result, bail};
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use veila_common::{
    PowerAction,
    ipc::{ClientMessage, DaemonMessage, decode_message, encode_message},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AuthEvent {
    Accepted {
        attempt_id: u64,
    },
    Rejected {
        attempt_id: u64,
        retry_after_ms: Option<u64>,
        failed_attempts: Option<u8>,
    },
    Busy {
        attempt_id: u64,
    },
}

const IPC_MAX_LINE_BYTES: usize = 64 * 1024;

pub(crate) fn submit_password(
    socket_path: PathBuf,
    attempt_id: u64,
    secret: String,
    sender: Sender<AuthEvent>,
) {
    thread::spawn(move || {
        if let Err(error) = run_attempt(socket_path, attempt_id, secret, sender) {
            tracing::warn!("failed to submit password attempt: {error:#}");
        }
    });
}

pub(crate) fn notify_activity(socket_path: PathBuf) {
    thread::spawn(move || {
        if let Err(error) = run_activity_notification(socket_path) {
            tracing::debug!("failed to notify daemon about lock activity: {error:#}");
        }
    });
}

pub(crate) fn request_power_action(socket_path: PathBuf, action: PowerAction) {
    thread::spawn(move || {
        if let Err(error) = run_power_action_request(socket_path, action) {
            tracing::warn!(?action, "failed to request power action: {error:#}");
        }
    });
}

fn run_attempt(
    socket_path: PathBuf,
    attempt_id: u64,
    secret: String,
    sender: Sender<AuthEvent>,
) -> anyhow::Result<()> {
    let started_at = Instant::now();
    let mut stream = UnixStream::connect(&socket_path)?;
    verify_socket_peer(&stream, &socket_path).context("auth socket peer rejected")?;
    let mut payload = encode_message(&ClientMessage::SubmitPassword { attempt_id, secret })?;
    payload.push('\n');
    stream.write_all(payload.as_bytes())?;
    stream.flush()?;
    tracing::debug!(
        attempt_id,
        elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        "submitted authentication request to daemon"
    );

    let mut reader = BufReader::new(stream);
    let Some(line) = read_bounded_line(&mut reader, "auth response")? else {
        tracing::info!(
            attempt_id,
            elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            "authentication socket closed without response; assuming success path"
        );
        return Ok(());
    };

    match decode_message::<DaemonMessage>(&line)? {
        DaemonMessage::AuthenticationAccepted { attempt_id } => {
            tracing::info!(
                elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                attempt_id,
                "daemon accepted authentication request"
            );
            let _ = sender.send(AuthEvent::Accepted { attempt_id });
        }
        DaemonMessage::AuthenticationRejected {
            attempt_id,
            retry_after_ms,
            failed_attempts,
        } => {
            tracing::info!(
                elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                attempt_id,
                "daemon rejected authentication request"
            );
            let _ = sender.send(AuthEvent::Rejected {
                attempt_id,
                retry_after_ms,
                failed_attempts,
            });
        }
        DaemonMessage::AuthenticationBusy { attempt_id } => {
            tracing::debug!(
                elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                attempt_id,
                "daemon reported authentication request is busy"
            );
            let _ = sender.send(AuthEvent::Busy { attempt_id });
        }
    }

    Ok(())
}

fn run_activity_notification(socket_path: PathBuf) -> anyhow::Result<()> {
    let mut stream = UnixStream::connect(&socket_path)?;
    verify_socket_peer(&stream, &socket_path).context("auth socket peer rejected")?;
    let mut payload = encode_message(&ClientMessage::Activity)?;
    payload.push('\n');
    stream.write_all(payload.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn run_power_action_request(socket_path: PathBuf, action: PowerAction) -> anyhow::Result<()> {
    let mut stream = UnixStream::connect(&socket_path)?;
    verify_socket_peer(&stream, &socket_path).context("auth socket peer rejected")?;
    let mut payload = encode_message(&ClientMessage::RequestPowerAction { action })?;
    payload.push('\n');
    stream.write_all(payload.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn verify_socket_peer(stream: &UnixStream, socket_path: &Path) -> Result<()> {
    let expected_uid = std::fs::metadata(socket_path)
        .with_context(|| format!("failed to inspect socket {}", socket_path.display()))?
        .uid();
    let peer = getsockopt(stream, PeerCredentials).context("failed to read peer credentials")?;
    if peer.uid() != expected_uid {
        bail!(
            "peer uid {} does not match socket owner uid {}",
            peer.uid(),
            expected_uid
        );
    }
    Ok(())
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
