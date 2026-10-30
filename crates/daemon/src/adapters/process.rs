use std::{
    path::{Path, PathBuf},
    process::Stdio,
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
    ipc::{
        CurtainControlMessage, CurtainInitialSnapshots, IPC_MAX_LINE_BYTES, LatencyReportMode,
        LockPowerStatusSnapshot, encode_message,
    },
};

use super::ipc;
mod curtain_handle;
pub(crate) use curtain_handle::{CurtainExit, CurtainHandle, probe_curtain};

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
    owner_record: &Path,
    config_path: Option<&Path>,
    initial_background_path: Option<&Path>,
    force_emergency_ui: bool,
    latency_report: LatencyReportMode,
) -> Result<Child> {
    let mut command = self_exe_command(CURTAIN_PROCESS_NAME, CURTAIN_SUBCOMMAND);
    command.arg("--owner-gate").stdin(Stdio::piped());
    command.arg(format!("--notify-socket={}", notify_socket.display()));
    command.arg(format!("--daemon-socket={}", daemon_socket.display()));
    command.arg(format!("--control-socket={}", control_socket.display()));
    command.arg(format!("--owner-record={}", owner_record.display()));
    if let Some(config_path) = config_path {
        command.arg(format!("--config={}", config_path.display()));
    }
    if let Some(initial_background_path) = initial_background_path {
        command.arg(format!(
            "--initial-background-path={}",
            initial_background_path.display()
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

pub async fn release_curtain_owner_gate(
    child: &mut Child,
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
) -> Result<()> {
    let payload =
        initial_snapshot_payload(weather_snapshot, battery_snapshot, now_playing_snapshot)?;
    let mut stdin = child
        .stdin
        .take()
        .context("curtain ownership gate is missing")?;
    // The gate byte comes last, so a partial write cannot start an unowned lock.
    stdin
        .write_all(payload.as_bytes())
        .await
        .context("failed to send initial curtain snapshots")?;
    stdin
        .write_all(&[1])
        .await
        .context("failed to release curtain ownership gate")
}

fn initial_snapshot_payload(
    weather_snapshot: Option<&WeatherSnapshot>,
    battery_snapshot: Option<&BatterySnapshot>,
    now_playing_snapshot: Option<&NowPlayingSnapshot>,
) -> Result<String> {
    let snapshots = CurtainInitialSnapshots {
        weather: weather_snapshot.cloned(),
        battery: battery_snapshot.cloned(),
        now_playing: now_playing_snapshot.cloned(),
    };
    let mut payload = encode_message(&snapshots).context("failed to encode initial snapshots")?;
    if payload.len() >= IPC_MAX_LINE_BYTES {
        tracing::warn!("initial widget snapshots exceed IPC limit; starting without them");
        payload = encode_message(&CurtainInitialSnapshots::default())?;
    }
    payload.push('\n');
    Ok(payload)
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

#[cfg(test)]
mod tests {
    use veila_common::{
        NowPlayingSnapshot,
        ipc::{CurtainInitialSnapshots, IPC_MAX_LINE_BYTES, decode_message},
    };

    use super::initial_snapshot_payload;

    #[test]
    fn oversized_snapshot_is_omitted_from_startup_payload() {
        let snapshot = NowPlayingSnapshot {
            title: "x".repeat(IPC_MAX_LINE_BYTES),
            artist: None,
            artwork_path: None,
            fetched_at_unix: 0,
        };
        let payload =
            initial_snapshot_payload(None, None, Some(&snapshot)).expect("bounded startup payload");
        let decoded: CurtainInitialSnapshots =
            decode_message(payload.trim_end()).expect("snapshot payload JSON");
        assert_eq!(decoded, CurtainInitialSnapshots::default());
    }
}
