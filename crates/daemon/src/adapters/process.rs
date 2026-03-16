use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};
use tokio::{
    process::{Child, Command},
    time::timeout,
};

pub async fn spawn_curtain(notify_socket: &Path) -> Result<Child> {
    let binary = curtain_binary_path()?;
    let mut command = Command::new(&binary);
    command.arg(format!("--notify-socket={}", notify_socket.display()));

    tracing::info!(binary = %binary.display(), "spawning curtain");

    command
        .spawn()
        .with_context(|| format!("failed to spawn '{}'", binary.display()))
}

pub async fn stop_curtain(mut child: Child) -> Result<()> {
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

pub fn notify_socket_path() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("kwylock-curtain-{stamp}.sock"))
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
