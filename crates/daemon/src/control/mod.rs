mod daemon;
mod theme;

use anyhow::{Result, bail};

use crate::{DaemonOptions, adapters::ipc, app};

use daemon::{
    print_running_health, print_running_status, print_version_info, reload_running_config,
    stop_running_daemon,
};
use theme::{
    print_available_themes, print_theme_source, set_theme_and_reload, unset_theme_and_reload,
};

pub const fn component_name() -> &'static str {
    "veilad"
}

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

pub async fn run(options: DaemonOptions) -> Result<()> {
    if options.help {
        print_help();
        return Ok(());
    }

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

fn print_help() {
    println!(
        "\
Veila daemon and control CLI

Usage:
  {name} [options]

General:
  -h, --help                 Show this help text
      --version              Print local and running daemon version info
      --config=<path>        Use a specific config file
      --log-file=<path>      Append daemon logs to a file when starting the daemon
      --session-id=<id>      Override the logind session id

Daemon control:
      --lock-now             Trigger an immediate lock
      --reload-config        Ask a running daemon to reload config from disk
      --status               Print daemon runtime status
      --health               Print daemon build and platform info
      --stop                 Stop the running daemon

Themes:
      --list-themes          List bundled themes
      --print-theme=<name>   Print a theme source file
      --set-theme=<name>     Set the active theme in config.toml
      --unset-theme          Remove the top-level theme key from config.toml

Notes:
  Only one control action can be used at a time.
  If no control action is given, {name} starts the daemon.
  --log-file only affects that daemon-start path.
  --set-theme creates config.toml automatically if it does not exist.
",
        name = component_name()
    );
}
