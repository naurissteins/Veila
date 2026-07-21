use std::{
    io::{BufRead, BufReader, Write},
    os::unix::fs::MetadataExt,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    thread,
    time::{Duration, Instant},
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
    /// The attempt produced no verdict; the daemon was unreachable, silent, or too slow.
    Failed {
        attempt_id: u64,
    },
}

const IPC_MAX_LINE_BYTES: usize = 64 * 1024;
const AUTH_WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const AUTH_RESPONSE_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) fn submit_password(
    socket_path: PathBuf,
    attempt_id: u64,
    secret: String,
    sender: Sender<AuthEvent>,
) {
    thread::spawn(move || {
        if let Err(error) = run_attempt(socket_path, attempt_id, secret, &sender) {
            tracing::warn!(attempt_id, "failed to submit password attempt: {error:#}");
            let _ = sender.send(AuthEvent::Failed { attempt_id });
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
    sender: &Sender<AuthEvent>,
) -> anyhow::Result<()> {
    let started_at = Instant::now();
    let mut stream = UnixStream::connect(&socket_path)?;
    verify_socket_peer(&stream, &socket_path).context("auth socket peer rejected")?;
    stream
        .set_write_timeout(Some(AUTH_WRITE_TIMEOUT))
        .context("failed to set auth write timeout")?;
    stream
        .set_read_timeout(Some(AUTH_RESPONSE_TIMEOUT))
        .context("failed to set auth response timeout")?;
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
        bail!("daemon closed the auth socket without sending a verdict");
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

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::net::UnixListener,
        path::PathBuf,
        sync::mpsc::channel,
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use veila_common::ipc::{DaemonMessage, encode_message};

    use super::{AuthEvent, submit_password};

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

    #[test]
    fn reports_failure_when_daemon_socket_is_missing() {
        let (sender, receiver) = channel();

        submit_password(
            unique_socket_path("auth-missing"),
            3,
            String::from("secret"),
            sender,
        );

        assert_eq!(
            receiver
                .recv_timeout(RECV_TIMEOUT)
                .expect("an unreachable daemon must still release the input guard"),
            AuthEvent::Failed { attempt_id: 3 }
        );
    }

    #[test]
    fn reports_failure_when_daemon_closes_without_verdict() {
        let path = unique_socket_path("auth-silent");
        let listener = UnixListener::bind(&path).expect("bind auth socket");
        let daemon = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream);
            let mut request = String::new();
            let _ = reader.read_line(&mut request);
        });

        let (sender, receiver) = channel();
        submit_password(path.clone(), 5, String::from("secret"), sender);

        assert_eq!(
            receiver
                .recv_timeout(RECV_TIMEOUT)
                .expect("a silent daemon must still release the input guard"),
            AuthEvent::Failed { attempt_id: 5 }
        );

        daemon.join().expect("daemon stub");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn forwards_daemon_rejection() {
        let path = unique_socket_path("auth-rejected");
        let listener = UnixListener::bind(&path).expect("bind auth socket");
        let daemon = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
            let mut request = String::new();
            reader.read_line(&mut request).expect("read request");

            let response = encode_message(&DaemonMessage::AuthenticationRejected {
                attempt_id: 9,
                retry_after_ms: Some(250),
                failed_attempts: Some(2),
            })
            .expect("encode response");
            stream
                .write_all(format!("{response}\n").as_bytes())
                .expect("write response");
            stream.flush().expect("flush response");
        });

        let (sender, receiver) = channel();
        submit_password(path.clone(), 9, String::from("secret"), sender);

        assert_eq!(
            receiver
                .recv_timeout(RECV_TIMEOUT)
                .expect("rejection event"),
            AuthEvent::Rejected {
                attempt_id: 9,
                retry_after_ms: Some(250),
                failed_attempts: Some(2),
            }
        );

        daemon.join().expect("daemon stub");
        std::fs::remove_file(&path).ok();
    }
}
