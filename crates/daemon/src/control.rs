use anyhow::{Result, bail};

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
        + usize::from(options.stop)
        + usize::from(options.status)
        + usize::from(options.health)
        + usize::from(options.version)
        + usize::from(options.reload_config);
    if control_mode_count > 1 {
        bail!(
            "use only one of --lock-now, --stop, --status, --health, --version, or --reload-config at a time"
        );
    }

    let daemon_socket_path = ipc::daemon_socket_path();
    if options.stop {
        stop_running_daemon(&daemon_socket_path).await?;
        println!("stopped=true");
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
    println!(
        "config={}",
        status.config_path.as_deref().unwrap_or("defaults")
    );
    Ok(())
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
