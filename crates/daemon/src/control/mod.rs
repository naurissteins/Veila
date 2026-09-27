mod config;
mod daemon;
mod doctor;
mod init;
mod logs;
mod term;
mod theme;

use anyhow::{Result, bail};

use crate::{
    DaemonOptions,
    adapters::{ipc, process::DAEMON_PROCESS_NAME},
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

pub async fn run_control(options: DaemonOptions) -> Result<()> {
    if options.help {
        print_control_help();
        return Ok(());
    }

    let control_mode_count = usize::from(options.lock_now)
        + usize::from(options.current_theme)
        + usize::from(options.print_theme.is_some())
        + usize::from(options.set_theme.is_some())
        + usize::from(options.unset_theme)
        + usize::from(options.stop)
        + usize::from(options.list_themes)
        + usize::from(options.status)
        + usize::from(options.health)
        + usize::from(options.doctor)
        + usize::from(options.check_config)
        + usize::from(options.init_config)
        + usize::from(options.version)
        + usize::from(options.reload_config)
        + usize::from(options.logs);
    if control_mode_count > 1 {
        bail!("use only one veila command at a time");
    }
    if options.wait_ready && !options.lock_now {
        bail!("--wait-ready can only be used with `veila lock`");
    }
    if options.force_emergency_ui && !options.lock_now {
        bail!("--force-emergency-ui can only be used with `veila lock`");
    }
    if options.latency_report.is_enabled() && !options.lock_now {
        bail!("--latency-report can only be used with `veila lock`");
    }

    if options.version {
        print_version_info();
        return Ok(());
    }

    if options.init_config {
        init_config(
            options.config_path.as_deref(),
            options.init_theme.as_deref(),
            options.init_force,
        )?;
        return Ok(());
    }

    if options.current_theme {
        print_current_theme(options.config_path.as_deref())?;
        return Ok(());
    }

    if let Some(theme) = options.print_theme.as_deref() {
        print_theme_source(theme, options.config_path.as_deref())?;
        return Ok(());
    }

    if options.list_themes {
        print_available_themes()?;
        return Ok(());
    }

    if options.check_config {
        print_config_validation(options.config_path.as_deref())?;
        return Ok(());
    }

    if options.logs {
        print_logs(&options)?;
        return Ok(());
    }

    let daemon_socket_path = ipc::daemon_socket_path()?;

    if let Some(theme) = options.set_theme.as_deref() {
        set_theme_and_reload(theme, options.config_path.as_deref(), &daemon_socket_path).await?;
        return Ok(());
    }

    if options.unset_theme {
        unset_theme_and_reload(options.config_path.as_deref(), &daemon_socket_path).await?;
        return Ok(());
    }

    if options.lock_now {
        let already_active = lock_running_daemon(
            &daemon_socket_path,
            options.wait_ready,
            options.force_emergency_ui,
            options.latency_report,
        )
        .await?;
        if options.wait_ready {
            println!("lock_ready=true");
            let (already_active, latency_report) = already_active.unwrap_or((false, None));
            println!("already_active={already_active}");
            if let Some(latency_report) = latency_report {
                print_latency_report(&latency_report, options.latency_report.is_verbose());
            }
        } else {
            println!("lock_requested=true");
        }
        return Ok(());
    }

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

    if options.doctor {
        print_doctor_report(
            options.config_path.as_deref(),
            options.session_id.as_deref(),
        )
        .await;
        return Ok(());
    }

    if options.reload_config {
        reload_running_config(&daemon_socket_path).await?;
        return Ok(());
    }

    print_control_help();
    Ok(())
}

fn print_control_help() {
    println!(
        "\
Veila screen locker

Usage:
  veila <command> [options]

General:
  -h, --help                 Show this help text
  -v, --version              Print the local Veila version
      --config=<path>        Use a specific config file for theme/config commands
      --force-emergency-ui   Combine with `lock` to test the emergency unlock prompt
      --latency-report[=verbose]
                             Combine with `lock --wait-ready` to print startup timings

Commands:
  daemon [options]           Start the daemon (see `veila daemon --help`)
  preview --preview-png=PATH Render the lockscreen to a PNG without locking
  lock [--wait-ready]        Ask the running daemon to lock now
  status                     Print daemon runtime status
  health                     Print daemon build and platform info
  doctor                     Check local runtime prerequisites without locking
  check-config               Validate config files without starting the daemon
  init [--theme NAME]        Create config.toml with a starting theme
       [--force]             Replace an existing config.toml
  reload                     Ask the running daemon to reload config from disk
  stop                       Stop the running daemon
  logs [--follow]            Show recent systemd user journal logs
       [--file]
       [--since WHEN]
       [--lines N]
       [--daemon|--curtain|--ui|--all]

Themes:
  theme list                 List bundled themes
  theme current              Print the active theme selection
  theme print <name>         Print a theme source file
  theme set <name>           Set the active theme in config.toml
  theme unset                Remove the top-level theme key from config.toml

Notes:
  Control commands never start the daemon. Start it with `veila daemon`, a user service, or your compositor config.
  `--wait-ready` can be combined with `veila lock` to block until the secure lock is active.
  Idle and lock-before-sleep locking are configured in the [idle] section of config.toml.
"
    );
}

fn print_latency_report(report: &veila_common::ipc::LockLatencyReport, verbose: bool) {
    println!("latency_report=true");
    println!("daemon_config_load_ms={}", report.daemon_config_load_ms);
    println!("daemon_socket_setup_ms={}", report.socket_setup_ms);
    println!("curtain_spawn_ms={}", report.curtain_spawn_ms);
    println!("curtain_ready_wait_ms={}", report.curtain_ready_wait_ms);
    println!("activation_total_ms={}", report.activation_total_ms);
    if verbose {
        println!("latency_report_mode=verbose");
        println!("daemon_config_load_us={}", report.daemon_config_load_us);
        println!("daemon_socket_setup_us={}", report.socket_setup_us);
        println!("curtain_spawn_us={}", report.curtain_spawn_us);
        println!("curtain_ready_wait_us={}", report.curtain_ready_wait_us);
        println!("activation_total_us={}", report.activation_total_us);
    }

    let Some(curtain) = report.curtain.as_ref() else {
        println!("curtain_report=unavailable");
        return;
    };

    println!("curtain_report=ok");
    println!("curtain_wayland_connect_ms={}", curtain.wayland_connect_ms);
    if verbose {
        println!("curtain_wayland_connect_us={}", curtain.wayland_connect_us);
    }
    println!("curtain_registry_ms={}", curtain.registry_ms);
    if verbose {
        println!("curtain_registry_us={}", curtain.registry_us);
    }
    println!("curtain_event_loop_ms={}", curtain.event_loop_ms);
    if verbose {
        println!("curtain_event_loop_us={}", curtain.event_loop_us);
    }
    println!("curtain_app_init_ms={}", curtain.app_init_ms);
    if verbose {
        println!("curtain_app_init_us={}", curtain.app_init_us);
    }
    println!("curtain_lock_request_ms={}", curtain.lock_request_ms);
    if verbose {
        println!("curtain_lock_request_us={}", curtain.lock_request_us);
    }
    println!(
        "curtain_startup_prepared_ms={}",
        curtain.startup_prepared_ms
    );
    if verbose {
        println!(
            "curtain_startup_prepared_us={}",
            curtain.startup_prepared_us
        );
    }
    println!(
        "first_surface_configured_ms={}",
        optional_ms(curtain.first_surface_configured_ms)
    );
    if verbose {
        println!(
            "first_surface_configured_us={}",
            optional_us(curtain.first_surface_configured_us)
        );
    }
    println!(
        "all_surfaces_configured_ms={}",
        optional_ms(curtain.all_surfaces_configured_ms)
    );
    if verbose {
        println!(
            "all_surfaces_configured_us={}",
            optional_us(curtain.all_surfaces_configured_us)
        );
        println!(
            "placeholder_committed_us={}",
            optional_us(curtain.placeholder_committed_us)
        );
    }
    println!(
        "session_locked_ms={}",
        optional_ms(curtain.session_locked_ms)
    );
    if verbose {
        println!(
            "session_locked_us={}",
            optional_us(curtain.session_locked_us)
        );
    }
    println!("first_frame_ms={}", optional_ms(curtain.first_frame_ms));
    if verbose {
        println!("first_frame_us={}", optional_us(curtain.first_frame_us));
    }
    println!(
        "ready_notified_ms={}",
        optional_ms(curtain.ready_notified_ms)
    );
    if verbose {
        println!(
            "ready_notified_us={}",
            optional_us(curtain.ready_notified_us)
        );
        print_verbose_latency_summary(report);
    }
    println!("surface_count={}", curtain.surface_count);
}

fn optional_ms(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("none"))
}

fn optional_us(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("none"))
}

fn print_verbose_latency_summary(report: &veila_common::ipc::LockLatencyReport) {
    let Some(curtain) = report.curtain.as_ref() else {
        return;
    };

    if let (Some(first_frame_us), Some(session_locked_us)) =
        (curtain.first_frame_us, curtain.session_locked_us)
    {
        println!(
            "first_frame_to_session_locked_us={}",
            session_locked_us.saturating_sub(first_frame_us)
        );
    }

    if let (Some(configured_us), Some(session_locked_us)) = (
        curtain.all_surfaces_configured_us,
        curtain.session_locked_us,
    ) {
        println!(
            "all_surfaces_to_session_locked_us={}",
            session_locked_us.saturating_sub(configured_us)
        );
    }
}
