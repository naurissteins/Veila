use anyhow::{Result, bail};
use time::{OffsetDateTime, UtcOffset};

use crate::{DaemonOptions, adapters::ipc, app};

/// Returns the component identifier used by logs and process supervision.
pub const fn component_name() -> &'static str {
    "veilad"
}

/// Returns machine-readable build information for the local binary.
pub fn local_build_info() -> veila_common::ipc::DaemonHealth {
    veila_common::ipc::DaemonHealth {
        component: component_name().to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_profile: if cfg!(debug_assertions) {
            "debug".to_string()
        } else {
            "release".to_string()
        },
        target_os: std::env::consts::OS.to_string(),
        target_arch: std::env::consts::ARCH.to_string(),
    }
}

/// Starts the daemon runtime.
pub async fn run(options: DaemonOptions) -> Result<()> {
    let control_mode_count = usize::from(options.lock_now)
        + usize::from(options.print_theme.is_some())
        + usize::from(options.set_theme.is_some())
        + usize::from(options.unset_theme)
        + usize::from(options.stop)
        + usize::from(options.list_themes)
        + usize::from(options.status)
        + usize::from(options.health)
        + usize::from(options.version)
        + usize::from(options.reload_config);
    if control_mode_count > 1 {
        bail!(
            "use only one of --lock-now, --print-theme, --set-theme, --unset-theme, --stop, --list-themes, --status, --health, --version, or --reload-config at a time"
        );
    }

    let daemon_socket_path = ipc::daemon_socket_path();
    if let Some(theme) = options.print_theme.as_deref() {
        print_theme_source(theme, options.config_path.as_deref())?;
        return Ok(());
    }

    if let Some(theme) = options.set_theme.as_deref() {
        set_theme_and_reload(theme, options.config_path.as_deref(), &daemon_socket_path).await?;
        return Ok(());
    }

    if options.unset_theme {
        unset_theme_and_reload(options.config_path.as_deref(), &daemon_socket_path).await?;
        return Ok(());
    }

    if options.stop {
        stop_running_daemon(&daemon_socket_path).await?;
        println!("stopped=true");
        return Ok(());
    }

    if options.list_themes {
        print_available_themes()?;
        return Ok(());
    }

    if options.status {
        print_running_status(&daemon_socket_path).await?;
        return Ok(());
    }

    if options.health {
        print_running_health(&daemon_socket_path).await?;
        return Ok(());
    }

    if options.version {
        print_version_info(&daemon_socket_path).await;
        return Ok(());
    }

    if options.reload_config {
        reload_running_config(&daemon_socket_path).await?;
        return Ok(());
    }

    match ipc::bind_single_instance_listener(&daemon_socket_path).await {
        Ok(control_listener) => app::run(options, control_listener, daemon_socket_path).await,
        Err(error) => {
            if options.lock_now && daemon_socket_path.exists() {
                let response = ipc::send_daemon_control_message(
                    &daemon_socket_path,
                    &veila_common::ipc::DaemonControlMessage::LockNow,
                )
                .await?;
                if response != veila_common::ipc::DaemonControlResponse::Accepted {
                    bail!("daemon did not acknowledge forwarded lock request");
                }
                tracing::info!(path = %daemon_socket_path.display(), "forwarded lock request to running daemon");
                Ok(())
            } else {
                Err(error)
            }
        }
    }
}

async fn stop_running_daemon(daemon_socket_path: &std::path::Path) -> Result<()> {
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

fn print_available_themes() -> Result<()> {
    for theme in veila_common::config::bundled_theme_names()? {
        println!("{theme}");
    }
    Ok(())
}

fn print_theme_source(theme: &str, config_path: Option<&std::path::Path>) -> Result<()> {
    let (path, raw) = veila_common::config::read_theme_source(config_path, theme)?;
    println!("theme={theme}");
    println!("source={}", path.display());
    println!();
    print!("{raw}");
    Ok(())
}

async fn print_running_status(daemon_socket_path: &std::path::Path) -> Result<()> {
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

async fn print_running_health(daemon_socket_path: &std::path::Path) -> Result<()> {
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

async fn print_version_info(daemon_socket_path: &std::path::Path) {
    let local = local_build_info();
    println!("local_component={}", local.component);
    println!("local_version={}", local.version);
    println!("local_build_profile={}", local.build_profile);
    println!("local_target_os={}", local.target_os);
    println!("local_target_arch={}", local.target_arch);

    match ipc::send_daemon_control_message(
        daemon_socket_path,
        &veila_common::ipc::DaemonControlMessage::Health,
    )
    .await
    {
        Ok(veila_common::ipc::DaemonControlResponse::Health(daemon)) => {
            println!("daemon_reachable=true");
            println!("daemon_component={}", daemon.component);
            println!("daemon_version={}", daemon.version);
            println!("daemon_build_profile={}", daemon.build_profile);
            println!("daemon_target_os={}", daemon.target_os);
            println!("daemon_target_arch={}", daemon.target_arch);
        }
        Ok(_) => {
            println!("daemon_reachable=false");
            println!("daemon_error=unexpected-health-response");
        }
        Err(error) => {
            println!("daemon_reachable=false");
            println!("daemon_error={}", error);
        }
    }
}

async fn reload_running_config(daemon_socket_path: &std::path::Path) -> Result<()> {
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

async fn set_theme_and_reload(
    theme: &str,
    config_path: Option<&std::path::Path>,
    daemon_socket_path: &std::path::Path,
) -> Result<()> {
    let written_path = veila_common::config::set_theme_in_config(config_path, theme)?;
    println!("theme={theme}");
    println!("config={}", written_path.display());

    if !daemon_socket_path.exists() {
        println!("live_reload=not-running");
        return Ok(());
    }

    let response = ipc::send_daemon_control_message(
        daemon_socket_path,
        &veila_common::ipc::DaemonControlMessage::ReloadConfig,
    )
    .await;

    match response {
        Ok(veila_common::ipc::DaemonControlResponse::Reloaded(status)) => {
            let daemon_config = status.config_path.as_deref().unwrap_or("defaults");
            println!("daemon_config={daemon_config}");
            println!(
                "daemon_config_matches={}",
                status
                    .config_path
                    .as_deref()
                    .is_some_and(|path| path == written_path.to_string_lossy())
            );
            println!("reload_source={}", status.reload_source);
            println!(
                "live_reload={}",
                match status.live_reload {
                    veila_common::ipc::LiveReloadStatus::NotActive => "not-active",
                    veila_common::ipc::LiveReloadStatus::Forwarded => "forwarded",
                }
            );
        }
        Ok(veila_common::ipc::DaemonControlResponse::Error { reason }) => {
            println!("live_reload=error");
            println!("reload_error={reason}");
        }
        Ok(_) => {
            println!("live_reload=error");
            println!("reload_error=unexpected-daemon-response");
        }
        Err(error) => {
            println!("live_reload=error");
            println!("reload_error={error}");
        }
    }

    Ok(())
}

async fn unset_theme_and_reload(
    config_path: Option<&std::path::Path>,
    daemon_socket_path: &std::path::Path,
) -> Result<()> {
    let (written_path, changed) = veila_common::config::unset_theme_in_config(config_path)?;
    println!("config={}", written_path.display());
    println!("theme_removed={changed}");

    if !changed {
        println!("live_reload=not-needed");
        return Ok(());
    }

    if !daemon_socket_path.exists() {
        println!("live_reload=not-running");
        return Ok(());
    }

    let response = ipc::send_daemon_control_message(
        daemon_socket_path,
        &veila_common::ipc::DaemonControlMessage::ReloadConfig,
    )
    .await;

    match response {
        Ok(veila_common::ipc::DaemonControlResponse::Reloaded(status)) => {
            let daemon_config = status.config_path.as_deref().unwrap_or("defaults");
            println!("daemon_config={daemon_config}");
            println!(
                "daemon_config_matches={}",
                status
                    .config_path
                    .as_deref()
                    .is_some_and(|path| path == written_path.to_string_lossy())
            );
            println!("reload_source={}", status.reload_source);
            println!(
                "live_reload={}",
                match status.live_reload {
                    veila_common::ipc::LiveReloadStatus::NotActive => "not-active",
                    veila_common::ipc::LiveReloadStatus::Forwarded => "forwarded",
                }
            );
        }
        Ok(veila_common::ipc::DaemonControlResponse::Error { reason }) => {
            println!("live_reload=error");
            println!("reload_error={reason}");
        }
        Ok(_) => {
            println!("live_reload=error");
            println!("reload_error=unexpected-daemon-response");
        }
        Err(error) => {
            println!("live_reload=error");
            println!("reload_error={error}");
        }
    }

    Ok(())
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
