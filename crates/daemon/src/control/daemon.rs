use anyhow::{Result, bail};
use time::{OffsetDateTime, UtcOffset};
use veila_common::ipc::LatencyReportMode;

use crate::adapters::ipc;

use super::{local_build_info, term};

pub(super) async fn stop_running_daemon(daemon_socket_path: &std::path::Path) -> Result<()> {
    ensure_running_daemon(daemon_socket_path)?;

    let response = ipc::send_daemon_control_message(
        daemon_socket_path,
        &veila_common::ipc::DaemonControlMessage::Stop,
    )
    .await?;

    if response != veila_common::ipc::DaemonControlResponse::Accepted {
        bail!("daemon returned an unexpected response to --stop");
    }

    Ok(())
}

pub(super) async fn lock_running_daemon(
    daemon_socket_path: &std::path::Path,
    wait_ready: bool,
    force_emergency_ui: bool,
    latency_report: LatencyReportMode,
) -> Result<Option<(bool, Option<veila_common::ipc::LockLatencyReport>)>> {
    ensure_running_daemon(daemon_socket_path)?;

    let response = ipc::send_daemon_control_message(
        daemon_socket_path,
        &veila_common::ipc::DaemonControlMessage::LockNow {
            wait_ready,
            force_emergency_ui,
            latency_report,
        },
    )
    .await?;

    match response {
        veila_common::ipc::DaemonControlResponse::Accepted if !wait_ready => Ok(None),
        veila_common::ipc::DaemonControlResponse::Locked {
            already_active,
            latency_report,
        } if wait_ready => Ok(Some((already_active, latency_report.map(|report| *report)))),
        veila_common::ipc::DaemonControlResponse::Error { reason } => bail!(reason),
        _ => bail!("daemon returned an unexpected response to lock"),
    }
}

pub(super) async fn print_running_status(daemon_socket_path: &std::path::Path) -> Result<()> {
    ensure_running_daemon(daemon_socket_path)?;

    let response = ipc::send_daemon_control_message(
        daemon_socket_path,
        &veila_common::ipc::DaemonControlMessage::Status,
    )
    .await?;

    let veila_common::ipc::DaemonControlResponse::Status(status) = response else {
        bail!("daemon returned an unexpected response to --status");
    };

    println!("state={}", status.state);
    println!("session={}", status.session);
    println!("active_lock={}", status.active_lock);
    println!("curtain_running={}", status.curtain_running);
    println!("live_reload_available={}", status.live_reload_available);
    println!("auto_reload_enabled={}", status.auto_reload_enabled);
    println!("auto_reload_debounce_ms={}", status.auto_reload_debounce_ms);
    println!(
        "last_reload_result={}",
        status.last_reload_result.as_deref().unwrap_or("none")
    );
    println!(
        "last_reload_unix_ms={}",
        status
            .last_reload_unix_ms
            .map(|value| value.to_string())
            .as_deref()
            .unwrap_or("none")
    );
    println!(
        "last_reload_local={}",
        status
            .last_reload_unix_ms
            .and_then(format_local_unix_ms)
            .as_deref()
            .unwrap_or("none")
    );
    println!(
        "config={}",
        status.config_path.as_deref().unwrap_or("defaults")
    );
    Ok(())
}

pub(super) async fn print_running_health(daemon_socket_path: &std::path::Path) -> Result<()> {
    ensure_running_daemon(daemon_socket_path)?;

    let response = ipc::send_daemon_control_message(
        daemon_socket_path,
        &veila_common::ipc::DaemonControlMessage::Health,
    )
    .await?;

    let veila_common::ipc::DaemonControlResponse::Health(health) = response else {
        bail!("daemon returned an unexpected response to --health");
    };

    println!("health=ok");
    println!("component={}", health.component);
    println!("version={}", health.version);
    println!("build_profile={}", health.build_profile);
    println!("target_os={}", health.target_os);
    println!("target_arch={}", health.target_arch);
    Ok(())
}

pub(super) fn print_version_info() {
    let local = local_build_info();
    println!("Veila {}", local.version);
}

pub(super) async fn reload_running_config(daemon_socket_path: &std::path::Path) -> Result<()> {
    ensure_running_daemon(daemon_socket_path)?;

    let response = ipc::send_daemon_control_message(
        daemon_socket_path,
        &veila_common::ipc::DaemonControlMessage::ReloadConfig,
    )
    .await?;

    match response {
        veila_common::ipc::DaemonControlResponse::Reloaded(status) => {
            println!("{}", term::success("Config reloaded successfully"));
            println!(
                "config: {}",
                status.config_path.as_deref().unwrap_or("defaults")
            );
            println!(
                "active lock: {}",
                if status.active_lock {
                    term::accent("yes")
                } else {
                    term::dim("no")
                }
            );
            println!("effect: {}", term::accent(reload_effect_message(&status)));
            Ok(())
        }
        veila_common::ipc::DaemonControlResponse::Error { reason } => bail!(reason),
        _ => bail!("daemon returned an unexpected response to --reload-config"),
    }
}

fn reload_effect_message(status: &veila_common::ipc::DaemonReloadStatus) -> &'static str {
    match (status.active_lock, &status.live_reload) {
        (false, _) => "changes will apply on next lock",
        (true, veila_common::ipc::LiveReloadStatus::Forwarded) => "active lockscreen updated",
        (true, veila_common::ipc::LiveReloadStatus::NotActive) => {
            "active lockscreen was not updated"
        }
    }
}

fn ensure_running_daemon(daemon_socket_path: &std::path::Path) -> Result<()> {
    if !daemon_socket_path.exists() {
        bail!(
            "veilad is not running; daemon socket does not exist at {}",
            daemon_socket_path.display()
        );
    }

    Ok(())
}

fn format_local_unix_ms(unix_ms: u64) -> Option<String> {
    let unix_ns = i128::from(unix_ms).checked_mul(1_000_000)?;
    let datetime = OffsetDateTime::from_unix_timestamp_nanos(unix_ns).ok()?;
    let local = datetime.to_offset(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC));
    Some(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} {:+03}:{:02}",
        local.year(),
        u8::from(local.month()),
        local.day(),
        local.hour(),
        local.minute(),
        local.second(),
        local.offset().whole_hours(),
        local.offset().minutes_past_hour().unsigned_abs()
    ))
}

#[cfg(test)]
mod tests {
    use veila_common::ipc::{DaemonReloadStatus, LiveReloadStatus};

    use super::reload_effect_message;

    #[test]
    fn reload_effect_reports_next_lock_when_no_active_lock() {
        let status = DaemonReloadStatus {
            config_path: None,
            active_lock: false,
            reload_source: "manual".to_string(),
            live_reload: LiveReloadStatus::NotActive,
        };

        assert_eq!(
            reload_effect_message(&status),
            "changes will apply on next lock"
        );
    }

    #[test]
    fn reload_effect_reports_live_update_when_forwarded() {
        let status = DaemonReloadStatus {
            config_path: None,
            active_lock: true,
            reload_source: "manual".to_string(),
            live_reload: LiveReloadStatus::Forwarded,
        };

        assert_eq!(reload_effect_message(&status), "active lockscreen updated");
    }
}
