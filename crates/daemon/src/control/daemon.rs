use anyhow::{Result, bail};
use time::{OffsetDateTime, UtcOffset};

use crate::adapters::ipc;

use super::local_build_info;

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

pub(super) async fn lock_running_daemon(daemon_socket_path: &std::path::Path) -> Result<()> {
    ensure_running_daemon(daemon_socket_path)?;

    let response = ipc::send_daemon_control_message(
        daemon_socket_path,
        &veila_common::ipc::DaemonControlMessage::LockNow,
    )
    .await?;

    if response != veila_common::ipc::DaemonControlResponse::Accepted {
        bail!("daemon returned an unexpected response to lock");
    }

    Ok(())
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
            println!(
                "config={}",
                status.config_path.as_deref().unwrap_or("defaults")
            );
            println!("active_lock={}", status.active_lock);
            println!("reload_source={}", status.reload_source);
            println!(
                "live_reload={}",
                match status.live_reload {
                    veila_common::ipc::LiveReloadStatus::NotActive => "not-active",
                    veila_common::ipc::LiveReloadStatus::Forwarded => "forwarded",
                }
            );
            Ok(())
        }
        veila_common::ipc::DaemonControlResponse::Error { reason } => bail!(reason),
        _ => bail!("daemon returned an unexpected response to --reload-config"),
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
