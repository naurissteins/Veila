use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use kwylock_common::ipc::{
    ClientMessage, DaemonControlMessage, DaemonControlResponse, DaemonMessage, decode_message,
    encode_message,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};

pub async fn bind_listener(path: &Path) -> Result<UnixListener> {
    if path.exists() {
        std::fs::remove_file(path)
            .with_context(|| format!("failed to remove stale socket {}", path.display()))?;
    }

    UnixListener::bind(path).with_context(|| format!("failed to bind {}", path.display()))
}

pub async fn bind_single_instance_listener(path: &Path) -> Result<UnixListener> {
    if path.exists() {
        match UnixStream::connect(path).await {
            Ok(_) => {
                return Err(anyhow!(
                    "kwylockd is already running and listening on {}",
                    path.display()
                ));
            }
            Err(_) => {
                std::fs::remove_file(path).with_context(|| {
                    format!("failed to remove stale daemon socket {}", path.display())
                })?;
            }
        }
    }

    UnixListener::bind(path).with_context(|| format!("failed to bind {}", path.display()))
}

pub fn auth_socket_path() -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or_default();
    let runtime_dir = runtime_dir();

    runtime_dir.join(format!("kwylock-auth-{stamp}.sock"))
}

pub fn daemon_socket_path() -> PathBuf {
    runtime_dir().join("kwylockd.sock")
}

pub async fn send_daemon_control_message(
    path: &Path,
    message: &DaemonControlMessage,
) -> Result<()> {
    let mut stream = UnixStream::connect(path)
        .await
        .with_context(|| format!("failed to connect to daemon socket {}", path.display()))?;
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

    let response = read_daemon_control_response(&mut stream).await?;
    if response != Some(DaemonControlResponse::Accepted) {
        return Err(anyhow!("daemon did not acknowledge control message"));
    }

    Ok(())
}

pub async fn read_client_message(stream: &mut UnixStream) -> Result<Option<ClientMessage>> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .await
        .context("failed to read auth request from socket")?;

    if read == 0 {
        return Ok(None);
    }

    decode_message(line.trim_end())
        .map(Some)
        .context("invalid auth request")
}

pub async fn write_daemon_message(stream: &mut UnixStream, message: &DaemonMessage) -> Result<()> {
    let mut payload = encode_message(message).context("failed to encode auth response")?;
    payload.push('\n');
    stream
        .write_all(payload.as_bytes())
        .await
        .context("failed to write auth response")?;
    stream
        .flush()
        .await
        .context("failed to flush auth response")
}

pub async fn read_daemon_control_message(
    stream: &mut UnixStream,
) -> Result<Option<DaemonControlMessage>> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .await
        .context("failed to read daemon control message")?;

    if read == 0 {
        return Ok(None);
    }

    decode_message(line.trim_end())
        .map(Some)
        .context("invalid daemon control message")
}

pub async fn write_daemon_control_response(
    stream: &mut UnixStream,
    response: &DaemonControlResponse,
) -> Result<()> {
    let mut payload =
        encode_message(response).context("failed to encode daemon control response")?;
    payload.push('\n');
    stream
        .write_all(payload.as_bytes())
        .await
        .context("failed to write daemon control response")?;
    stream
        .flush()
        .await
        .context("failed to flush daemon control response")
}

async fn read_daemon_control_response(
    stream: &mut UnixStream,
) -> Result<Option<DaemonControlResponse>> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .await
        .context("failed to read daemon control response")?;

    if read == 0 {
        return Ok(None);
    }

    decode_message(line.trim_end())
        .map(Some)
        .context("invalid daemon control response")
}

fn runtime_dir() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir())
}
