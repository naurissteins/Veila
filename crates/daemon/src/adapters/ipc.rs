use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use kwylock_common::ipc::{ClientMessage, DaemonMessage, decode_message, encode_message};
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

pub fn auth_socket_path() -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or_default();
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());

    runtime_dir.join(format!("kwylock-auth-{stamp}.sock"))
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
