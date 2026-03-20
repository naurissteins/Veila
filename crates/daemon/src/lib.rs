#![forbid(unsafe_code)]

//! Daemon entrypoints for Kwylock lock orchestration.

mod adapters;
mod app;
mod domain;

use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::adapters::ipc;

/// Returns the component identifier used by logs and process supervision.
pub const fn component_name() -> &'static str {
    "kwylockd"
}

/// Returns machine-readable build information for the local binary.
pub fn local_build_info() -> kwylock_common::ipc::DaemonHealth {
    kwylock_common::ipc::DaemonHealth {
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonOptions {
    pub config_path: Option<PathBuf>,
    pub session_id: Option<String>,
    pub lock_now: bool,
    pub status: bool,
    pub health: bool,
    pub version: bool,
    pub reload_config: bool,
}

impl DaemonOptions {
    pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self::default();

        for arg in args.into_iter().skip(1) {
            if let Some(path) = arg.strip_prefix("--config=") {
                options.config_path = Some(PathBuf::from(path));
                continue;
            }

            if let Some(session_id) = arg.strip_prefix("--session-id=") {
                options.session_id = Some(session_id.to_string());
                continue;
            }

            if arg == "--lock-now" {
                options.lock_now = true;
                continue;
            }

            if arg == "--status" {
                options.status = true;
                continue;
            }

            if arg == "--health" {
                options.health = true;
                continue;
            }

            if arg == "--version" {
                options.version = true;
                continue;
            }

            if arg == "--reload-config" {
                options.reload_config = true;
                continue;
            }

            bail!("unknown daemon argument: {arg}");
        }

        Ok(options)
    }
}

/// Starts the daemon runtime.
pub async fn run(options: DaemonOptions) -> anyhow::Result<()> {
    let control_mode_count = usize::from(options.lock_now)
        + usize::from(options.status)
        + usize::from(options.health)
        + usize::from(options.version)
        + usize::from(options.reload_config);
    if control_mode_count > 1 {
        bail!(
            "use only one of --lock-now, --status, --health, --version, or --reload-config at a time"
        );
    }

    let daemon_socket_path = ipc::daemon_socket_path();
    if options.status {
        if !daemon_socket_path.exists() {
            bail!(
                "kwylockd is not running; daemon socket does not exist at {}",
                daemon_socket_path.display()
            );
        }

        let response = ipc::send_daemon_control_message(
            &daemon_socket_path,
            &kwylock_common::ipc::DaemonControlMessage::Status,
        )
        .await?;

        let kwylock_common::ipc::DaemonControlResponse::Status(status) = response else {
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
        return Ok(());
    }

    if options.health {
        if !daemon_socket_path.exists() {
            bail!(
                "kwylockd is not running; daemon socket does not exist at {}",
                daemon_socket_path.display()
            );
        }

        let response = ipc::send_daemon_control_message(
            &daemon_socket_path,
            &kwylock_common::ipc::DaemonControlMessage::Health,
        )
        .await?;

        let kwylock_common::ipc::DaemonControlResponse::Health(health) = response else {
            bail!("daemon returned an unexpected response to --health");
        };

        println!("health=ok");
        println!("component={}", health.component);
        println!("version={}", health.version);
        println!("build_profile={}", health.build_profile);
        println!("target_os={}", health.target_os);
        println!("target_arch={}", health.target_arch);
        return Ok(());
    }

    if options.version {
        let local = local_build_info();
        println!("local_component={}", local.component);
        println!("local_version={}", local.version);
        println!("local_build_profile={}", local.build_profile);
        println!("local_target_os={}", local.target_os);
        println!("local_target_arch={}", local.target_arch);

        match ipc::send_daemon_control_message(
            &daemon_socket_path,
            &kwylock_common::ipc::DaemonControlMessage::Health,
        )
        .await
        {
            Ok(kwylock_common::ipc::DaemonControlResponse::Health(daemon)) => {
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

        return Ok(());
    }

    if options.reload_config {
        if !daemon_socket_path.exists() {
            bail!(
                "kwylockd is not running; daemon socket does not exist at {}",
                daemon_socket_path.display()
            );
        }

        let response = ipc::send_daemon_control_message(
            &daemon_socket_path,
            &kwylock_common::ipc::DaemonControlMessage::ReloadConfig,
        )
        .await?;

        match response {
            kwylock_common::ipc::DaemonControlResponse::Reloaded(status) => {
                println!(
                    "config={}",
                    status.config_path.as_deref().unwrap_or("defaults")
                );
                println!("active_lock={}", status.active_lock);
                println!(
                    "live_reload={}",
                    match status.live_reload {
                        kwylock_common::ipc::LiveReloadStatus::NotActive => "not-active",
                        kwylock_common::ipc::LiveReloadStatus::Forwarded => "forwarded",
                    }
                );
                return Ok(());
            }
            kwylock_common::ipc::DaemonControlResponse::Error { reason } => {
                bail!(reason);
            }
            _ => bail!("daemon returned an unexpected response to --reload-config"),
        }
    }

    match ipc::bind_single_instance_listener(&daemon_socket_path).await {
        Ok(control_listener) => app::run(options, control_listener, daemon_socket_path).await,
        Err(error) => {
            if options.lock_now && daemon_socket_path.exists() {
                let response = ipc::send_daemon_control_message(
                    &daemon_socket_path,
                    &kwylock_common::ipc::DaemonControlMessage::LockNow,
                )
                .await?;
                if response != kwylock_common::ipc::DaemonControlResponse::Accepted {
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

#[cfg(test)]
mod tests {
    use super::DaemonOptions;

    #[test]
    fn parses_config_argument() {
        let options = DaemonOptions::parse_args([
            "kwylockd".to_string(),
            "--config=/tmp/kwylock.toml".to_string(),
        ])
        .expect("arguments should parse");

        assert_eq!(
            options.config_path.as_deref(),
            Some(std::path::Path::new("/tmp/kwylock.toml"))
        );
    }

    #[test]
    fn parses_session_id_argument() {
        let options =
            DaemonOptions::parse_args(["kwylockd".to_string(), "--session-id=c2".to_string()])
                .expect("arguments should parse");

        assert_eq!(options.session_id.as_deref(), Some("c2"));
    }

    #[test]
    fn parses_lock_now_argument() {
        let options = DaemonOptions::parse_args(["kwylockd".to_string(), "--lock-now".to_string()])
            .expect("arguments should parse");

        assert!(options.lock_now);
    }

    #[test]
    fn parses_status_argument() {
        let options = DaemonOptions::parse_args(["kwylockd".to_string(), "--status".to_string()])
            .expect("arguments should parse");

        assert!(options.status);
    }

    #[test]
    fn parses_reload_config_argument() {
        let options =
            DaemonOptions::parse_args(["kwylockd".to_string(), "--reload-config".to_string()])
                .expect("arguments should parse");

        assert!(options.reload_config);
    }

    #[test]
    fn parses_health_argument() {
        let options = DaemonOptions::parse_args(["kwylockd".to_string(), "--health".to_string()])
            .expect("arguments should parse");

        assert!(options.health);
    }

    #[test]
    fn parses_version_argument() {
        let options = DaemonOptions::parse_args(["kwylockd".to_string(), "--version".to_string()])
            .expect("arguments should parse");

        assert!(options.version);
    }
}
