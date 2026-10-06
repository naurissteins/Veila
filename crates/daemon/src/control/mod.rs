mod config;
mod daemon;
mod doctor;
mod help;
mod init;
mod latency;
mod logs;
mod term;
mod theme;

use anyhow::Result;

use crate::{
    ControlOptions,
    adapters::{ipc, process::DAEMON_PROCESS_NAME},
    options::{ControlCommand, LockOptions},
};

use config::print_config_validation;
use daemon::{
    lock_running_daemon, print_running_health, print_running_status, print_version_info,
    reload_running_config, stop_running_daemon,
};
use doctor::print_doctor_report;
use init::init_config;
use logs::print_logs;
use theme::{
    print_available_themes, print_current_theme, print_theme_source, set_theme_and_reload,
    unset_theme_and_reload,
};

const DAEMON_SERVICE: &str = "veila.service";

pub fn local_build_info() -> veila_common::ipc::DaemonHealth {
    veila_common::ipc::DaemonHealth {
        component: DAEMON_PROCESS_NAME.to_string(),
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

pub async fn run_control(options: ControlOptions) -> Result<()> {
    if options.help {
        help::print_control_help();
        return Ok(());
    }
    let config_path = options.config_path.as_deref();
    match options.command {
        Some(ControlCommand::Version) => {
            print_version_info();
            Ok(())
        }
        Some(ControlCommand::Init { theme, force }) => {
            init_config(config_path, theme.as_deref(), force)
        }
        Some(ControlCommand::ThemeCurrent) => print_current_theme(config_path),
        Some(ControlCommand::ThemePrint(theme)) => print_theme_source(&theme, config_path),
        Some(ControlCommand::ThemeList) => print_available_themes(),
        Some(ControlCommand::CheckConfig) => print_config_validation(config_path),
        Some(ControlCommand::Logs(logs)) => print_logs(config_path, &logs),
        Some(ControlCommand::ThemeSet(theme)) => {
            set_theme_and_reload(&theme, config_path, &ipc::daemon_socket_path()?).await
        }
        Some(ControlCommand::ThemeUnset) => {
            unset_theme_and_reload(config_path, &ipc::daemon_socket_path()?).await
        }
        Some(ControlCommand::Lock(lock)) => request_lock(&ipc::daemon_socket_path()?, lock).await,
        Some(ControlCommand::Stop) => {
            stop_running_daemon(&ipc::daemon_socket_path()?).await?;
            println!("stopped=true");
            Ok(())
        }
        Some(ControlCommand::Status) => print_running_status(&ipc::daemon_socket_path()?).await,
        Some(ControlCommand::Health) => print_running_health(&ipc::daemon_socket_path()?).await,
        Some(ControlCommand::Doctor) => {
            let _socket_path = ipc::daemon_socket_path()?;
            print_doctor_report(config_path, None).await;
            Ok(())
        }
        Some(ControlCommand::Reload) => reload_running_config(&ipc::daemon_socket_path()?).await,
        None => {
            let _socket_path = ipc::daemon_socket_path()?;
            help::print_control_help();
            Ok(())
        }
    }
}

async fn request_lock(socket_path: &std::path::Path, options: LockOptions) -> Result<()> {
    let already_active = lock_running_daemon(
        socket_path,
        options.wait_ready,
        options.force_emergency_ui,
        options.latency_report,
    )
    .await?;
    if options.wait_ready {
        println!("lock_ready=true");
        let (already_active, report) = already_active.unwrap_or((false, None));
        println!("already_active={already_active}");
        if let Some(report) = report {
            latency::print_latency_report(&report, options.latency_report.is_verbose());
        }
    } else {
        println!("lock_requested=true");
    }
    Ok(())
}
