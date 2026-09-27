use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};
use tokio::{
    io::AsyncWriteExt,
    net::UnixStream,
    process::{Child, Command},
    time::timeout,
};
use veila_common::{
    BatterySnapshot, FingerprintStatus, NowPlayingSnapshot, WeatherSnapshot,
    ipc::{CurtainControlMessage, LatencyReportMode, LockPowerStatusSnapshot, encode_message},
};

use super::ipc;

const CURTAIN_CONTROL_TIMEOUT: Duration = Duration::from_secs(2);
const SELF_EXE: &str = "/proc/self/exe";

pub const CURTAIN_SUBCOMMAND: &str = "__curtain";
pub const PREWARM_SUBCOMMAND: &str = "__prewarm";
pub const DAEMON_PROCESS_NAME: &str = "veila-daemon";
pub const CURTAIN_PROCESS_NAME: &str = "veila-curtain";
pub const PREWARM_PROCESS_NAME: &str = "veila-prewarm";

#[allow(clippy::too_many_arguments)]
pub async fn spawn_curtain(
    notify_socket: &Path,
    daemon_socket: &Path,
    control_socket: &Path,
    config_path: Option<&Path>,
    initial_background_path: Option<&Path>,
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
    force_emergency_ui: bool,
    latency_report: LatencyReportMode,
) -> Result<Child> {
    let mut command = self_exe_command(CURTAIN_PROCESS_NAME, CURTAIN_SUBCOMMAND);
    command.arg(format!("--notify-socket={}", notify_socket.display()));
    command.arg(format!("--daemon-socket={}", daemon_socket.display()));
    command.arg(format!("--control-socket={}", control_socket.display()));
    if let Some(config_path) = config_path {
        command.arg(format!("--config={}", config_path.display()));
    }
    if let Some(initial_background_path) = initial_background_path {
        command.arg(format!(
            "--initial-background-path={}",
            initial_background_path.display()
        ));
    }
    if let Some(weather_snapshot) = weather_snapshot {
        command.arg(format!(
            "--weather-snapshot={}",
            encode_message(weather_snapshot).context("failed to encode weather snapshot")?
        ));
    }
    if let Some(battery_snapshot) = battery_snapshot {
        command.arg(format!(
            "--battery-snapshot={}",
            encode_message(battery_snapshot).context("failed to encode battery snapshot")?
        ));
    }
    if let Some(now_playing_snapshot) = now_playing_snapshot {
        command.arg(format!(
            "--now-playing-snapshot={}",
            encode_message(now_playing_snapshot)
                .context("failed to encode now playing snapshot")?
        ));
    }
    if force_emergency_ui {
        command.arg("--force-emergency-ui");
    }
    match latency_report {
        LatencyReportMode::Disabled => {}
        LatencyReportMode::Basic => {
            command.arg("--latency-report");
        }
        LatencyReportMode::Verbose => {
            command.arg("--latency-report=verbose");
        }
    }

    tracing::info!("spawning curtain");

    command
        .spawn()
        .context("failed to spawn the curtain process")
}

pub async fn request_curtain_unlock(control_socket: &Path, attempt_id: Option<u64>) -> Result<()> {
    send_curtain_control_message(
        control_socket,
        &CurtainControlMessage::Unlock { attempt_id },
        "unlock request",
    )
    .await
}

pub async fn request_curtain_reload(control_socket: &Path) -> Result<()> {
    send_curtain_control_message(
        control_socket,
        &CurtainControlMessage::ReloadConfig,
        "reload request",
    )
    .await
}

pub async fn request_curtain_arm_resume_input_guard(control_socket: &Path) -> Result<()> {
    send_curtain_control_message(
        control_socket,
        &CurtainControlMessage::ArmResumeInputGuard,
        "resume input guard request",
    )
    .await
}

pub async fn request_curtain_mark_resumed(control_socket: &Path) -> Result<()> {
    send_curtain_control_message(
        control_socket,
        &CurtainControlMessage::MarkResumed,
        "resume completed request",
    )
    .await
}

pub async fn request_curtain_now_playing_update(
    control_socket: &Path,
    snapshot: Option<&NowPlayingSnapshot>,
) -> Result<()> {
    send_curtain_control_message(
        control_socket,
        &CurtainControlMessage::UpdateNowPlaying {
            snapshot: snapshot.cloned(),
        },
        "now playing update",
    )
    .await
}

pub async fn request_curtain_power_status_update(
    control_socket: &Path,
    snapshot: Option<&LockPowerStatusSnapshot>,
) -> Result<()> {
    send_curtain_control_message(
        control_socket,
        &CurtainControlMessage::UpdatePowerStatus {
            snapshot: snapshot.cloned(),
        },
        "power status update",
    )
    .await
}

pub async fn request_curtain_fingerprint_status_update(
    control_socket: &Path,
    status: Option<&FingerprintStatus>,
) -> Result<()> {
    send_curtain_control_message(
        control_socket,
        &CurtainControlMessage::UpdateFingerprintStatus {
            status: status.cloned(),
        },
        "fingerprint status update",
    )
    .await
}

async fn send_curtain_control_message(
    control_socket: &Path,
    message: &CurtainControlMessage,
    label: &str,
) -> Result<()> {
    let mut payload =
        encode_message(message).with_context(|| format!("failed to encode {label}"))?;
    payload.push('\n');

    timeout(
        CURTAIN_CONTROL_TIMEOUT,
        write_curtain_control_payload(control_socket, payload.as_bytes(), label),
    )
    .await
    .with_context(|| format!("timed out sending {label}"))?
}

async fn write_curtain_control_payload(
    control_socket: &Path,
    payload: &[u8],
    label: &str,
) -> Result<()> {
    let mut stream = UnixStream::connect(control_socket).await.with_context(|| {
        format!(
            "failed to connect to curtain control socket {}",
            control_socket.display()
        )
    })?;
    stream
        .write_all(payload)
        .await
        .with_context(|| format!("failed to write {label}"))?;
    stream
        .flush()
        .await
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

pub async fn spawn_background_prewarm_helper(config_path: Option<&Path>) -> Result<Child> {
    let mut command = self_exe_command(PREWARM_PROCESS_NAME, PREWARM_SUBCOMMAND);
    if let Some(config_path) = config_path {
        command.arg(format!("--config={}", config_path.display()));
    }

    tracing::debug!("spawning background prewarm helper");

    command
        .spawn()
        .context("failed to spawn the background prewarm helper")
}

pub fn notify_socket_path() -> Result<PathBuf> {
    ipc::transient_socket_path("curtain")
}

pub fn control_socket_path() -> Result<PathBuf> {
    ipc::transient_socket_path("control")
}

fn self_exe_command(process_name: &str, subcommand: &str) -> Command {
    // Re-executing the running inode keeps daemon and curtain on one version across package upgrades.
    let mut command = Command::new(SELF_EXE);
    command.arg0(process_name).arg(subcommand);
    command
}
