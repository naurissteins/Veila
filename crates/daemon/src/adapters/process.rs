use std::{
    io::Write,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use kwylock_common::ipc::{CurtainControlMessage, encode_message};
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};
use tokio::{
    process::{Child, Command},
    time::timeout,
};

pub async fn spawn_curtain(
    notify_socket: &Path,
    daemon_socket: &Path,
    control_socket: &Path,
    config_path: Option<&Path>,
) -> Result<Child> {
    let binary = curtain_binary_path()?;
    let mut command = Command::new(&binary);
    command.arg(format!("--notify-socket={}", notify_socket.display()));
    command.arg(format!("--daemon-socket={}", daemon_socket.display()));
    command.arg(format!("--control-socket={}", control_socket.display()));
    if let Some(config_path) = config_path {
        command.arg(format!("--config={}", config_path.display()));
    }

    tracing::info!(binary = %binary.display(), "spawning curtain");

    command
        .spawn()
        .with_context(|| format!("failed to spawn '{}'", binary.display()))
}

pub async fn request_curtain_unlock(control_socket: &Path, attempt_id: Option<u64>) -> Result<()> {
    send_curtain_control_message(
        control_socket,
        &CurtainControlMessage::Unlock { attempt_id },
        "unlock request",
    )
}

pub async fn request_curtain_reload(control_socket: &Path) -> Result<()> {
    send_curtain_control_message(
        control_socket,
        &CurtainControlMessage::ReloadConfig,
        "reload request",
    )
}

fn send_curtain_control_message(
    control_socket: &Path,
    message: &CurtainControlMessage,
    label: &str,
) -> Result<()> {
    let mut stream = UnixStream::connect(control_socket).with_context(|| {
        format!(
            "failed to connect to curtain control socket {}",
            control_socket.display()
        )
    })?;
    let mut payload =
        encode_message(message).with_context(|| format!("failed to encode {label}"))?;
    payload.push('\n');
    stream
        .write_all(payload.as_bytes())
        .with_context(|| format!("failed to write {label}"))?;
    stream
        .flush()
        .with_context(|| format!("failed to flush {label}"))
}

pub async fn force_stop_curtain(mut child: Child) -> Result<()> {
    if let Some(raw_pid) = child.id() {
        kill(Pid::from_raw(raw_pid as i32), Signal::SIGTERM)
            .with_context(|| format!("failed to send SIGTERM to curtain process {raw_pid}"))?;
    }

    match timeout(Duration::from_secs(2), child.wait()).await {
        Ok(Ok(status)) => {
            tracing::info!(?status, "curtain exited");
            Ok(())
        }
        Ok(Err(error)) => Err(error).context("failed while waiting for curtain to exit"),
        Err(_) => {
            tracing::warn!("curtain did not exit after SIGTERM; sending SIGKILL");
            child.kill().await.context("failed to SIGKILL curtain")
        }
    }
}

pub async fn wait_for_graceful_curtain_exit(
    mut child: Child,
    window: Duration,
) -> Result<Option<Child>> {
    match timeout(window, child.wait()).await {
        Ok(Ok(status)) => {
            tracing::info!(?status, "curtain exited");
            Ok(None)
        }
        Ok(Err(error)) => Err(error).context("failed while waiting for curtain to exit"),
        Err(_) => Ok(Some(child)),
    }
}

pub fn notify_socket_path() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("kwylock-curtain-{stamp}.sock"))
}

pub fn control_socket_path() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or_default();
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());

    runtime_dir.join(format!("kwylock-control-{stamp}.sock"))
}

fn curtain_binary_path() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("KWYLOCK_CURTAIN_BIN") {
        return Ok(PathBuf::from(path));
    }

    if let Ok(mut current_exe) = std::env::current_exe() {
        current_exe.set_file_name("kwylock-curtain");
        if current_exe.exists() {
            return Ok(current_exe);
        }
    }

    Ok(PathBuf::from("kwylock-curtain"))
}
