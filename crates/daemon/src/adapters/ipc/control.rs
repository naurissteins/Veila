use std::{path::Path, time::Duration};

use anyhow::{Context, Result, anyhow};
use tokio::{io::AsyncWriteExt, net::UnixStream, time::timeout};
use veila_common::ipc::{
    DaemonControlMessage, DaemonControlResponse, decode_message, encode_message,
};

use super::{read_bounded_line, verify_peer_uid};

const SEND_TIMEOUT: Duration = Duration::from_secs(2);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
const DEFERRED_RESPONSE_TIMEOUT: Duration = Duration::from_secs(120);

pub async fn send_daemon_control_message(
    path: &Path,
    message: &DaemonControlMessage,
) -> Result<DaemonControlResponse> {
    let mut stream = timeout(SEND_TIMEOUT, send_request(path, message))
        .await
        .with_context(|| format!(
            "timed out sending daemon control request after {} seconds; delivery may be incomplete",
            SEND_TIMEOUT.as_secs(),
        ))??;
    let response_timeout = response_timeout(message);
    // Partial response bytes never renew the deadline for the complete frame.
    timeout(response_timeout, read_response(&mut stream))
        .await
        .with_context(|| format!(
            "timed out waiting for daemon control response after {} seconds; the request may still complete",
            response_timeout.as_secs(),
        ))??
        .ok_or_else(|| anyhow!("daemon closed control socket without a response"))
}

fn response_timeout(message: &DaemonControlMessage) -> Duration {
    match message {
        // Readiness and reload can wait behind a pending activation or authorized unlock.
        DaemonControlMessage::LockNow {
            wait_ready: true, ..
        }
        | DaemonControlMessage::ReloadConfig => DEFERRED_RESPONSE_TIMEOUT,
        DaemonControlMessage::LockNow {
            wait_ready: false, ..
        }
        | DaemonControlMessage::Stop
        | DaemonControlMessage::Status
        | DaemonControlMessage::Health => RESPONSE_TIMEOUT,
    }
}

async fn send_request(path: &Path, message: &DaemonControlMessage) -> Result<UnixStream> {
    let mut stream = UnixStream::connect(path)
        .await
        .with_context(|| format!("failed to connect to daemon socket {}", path.display()))?;
    verify_peer_uid(&stream).context("daemon control socket peer rejected")?;

    let mut payload = encode_message(message).context("failed to encode daemon control message")?;
    payload.push('\n');
    stream
        .write_all(payload.as_bytes())
        .await
        .context("failed to write daemon control message")?;
    stream
        .flush()
        .await
        .context("failed to flush daemon control message")?;
    Ok(stream)
}

async fn read_response(stream: &mut UnixStream) -> Result<Option<DaemonControlResponse>> {
    let Some(line) = read_bounded_line(stream, "daemon control response").await? else {
        return Ok(None);
    };
    decode_message(&line)
        .map(Some)
        .context("invalid daemon control response")
}

#[cfg(test)]
mod tests;
