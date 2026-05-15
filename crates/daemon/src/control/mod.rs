mod config;
mod daemon;
mod doctor;
mod term;
mod theme;

use anyhow::{Result, bail};

use crate::{DaemonOptions, adapters::ipc, app};

use config::print_config_validation;
use daemon::{
    lock_running_daemon, print_running_health, print_running_status, print_version_info,
    reload_running_config, stop_running_daemon,
};
use doctor::print_doctor_report;
use theme::{
    print_available_themes, print_current_theme, print_theme_source, set_theme_and_reload,
    unset_theme_and_reload,
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

    if options.background_prewarm_only {
        app::run_background_prewarm_once(options.config_path.as_deref()).await?;
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
        + usize::from(options.version)
        + usize::from(options.reload_config);
    if control_mode_count > 1 {
        bail!(
            "use only one of --lock-now, --current-theme, --print-theme, --set-theme, --unset-theme, --stop, --list-themes, --status, --health, --doctor, --check-config, --version, or --reload-config at a time"
        );
    }
    if options.wait_ready && !options.lock_now {
        bail!("--wait-ready can only be used with --lock-now");
    }
    if options.force_emergency_ui && !options.lock_now {
        bail!("--force-emergency-ui can only be used with --lock-now");
    }
    if options.latency_report.is_enabled() && !options.lock_now {
        bail!("--latency-report can only be used with --lock-now");
    }

    let daemon_socket_path = ipc::daemon_socket_path();
    if options.current_theme {
        print_current_theme(options.config_path.as_deref())?;
        return Ok(());
    }

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

    if options.doctor {
        print_doctor_report(
            options.config_path.as_deref(),
            options.session_id.as_deref(),
        )
        .await;
        return Ok(());
    }

    if options.check_config {
        print_config_validation(options.config_path.as_deref())?;
        return Ok(());
    }

    if options.version {
        print_version_info();
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
                    &veila_common::ipc::DaemonControlMessage::LockNow {
                        wait_ready: options.wait_ready,
                        force_emergency_ui: options.force_emergency_ui,
                        latency_report: options.latency_report,
                    },
                )
                .await?;
                match response {
                    veila_common::ipc::DaemonControlResponse::Accepted => {}
                    veila_common::ipc::DaemonControlResponse::Locked { .. }
                        if options.wait_ready => {}
                    veila_common::ipc::DaemonControlResponse::Error { reason } => bail!(reason),
                    _ => bail!("daemon did not acknowledge forwarded lock request"),
                }
                tracing::info!(path = %daemon_socket_path.display(), "forwarded lock request to running daemon");
                Ok(())
            } else {
                Err(error)
            }
        }
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
        + usize::from(options.version)
        + usize::from(options.reload_config);
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

    let daemon_socket_path = ipc::daemon_socket_path();
    if options.version {
        print_version_info();
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

    if let Some(theme) = options.set_theme.as_deref() {
        set_theme_and_reload(theme, options.config_path.as_deref(), &daemon_socket_path).await?;
        return Ok(());
    }

    if options.unset_theme {
        unset_theme_and_reload(options.config_path.as_deref(), &daemon_socket_path).await?;
        return Ok(());
    }

    if options.list_themes {
        print_available_themes()?;
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

    if options.check_config {
        print_config_validation(options.config_path.as_deref())?;
        return Ok(());
    }

    if options.reload_config {
        reload_running_config(&daemon_socket_path).await?;
        return Ok(());
    }

    print_control_help();
    Ok(())
}

fn print_help() {
    println!(
        "\
Veila daemon

Usage:
  {name} [options]

General:
  -h, --help                 Show this help text
      --version              Print the local Veila version
      --config=<path>        Use a specific config file
      --log-file=<path>      Append daemon logs to a file when starting the daemon
      --session-id=<id>      Override the logind session id

Legacy control:
      --lock-now             Trigger an immediate lock
      --wait-ready           Return only after the secure lock is active
      --force-emergency-ui   Lock with the built-in emergency unlock prompt
      --latency-report[=verbose]
                             Print startup timing details after --wait-ready
      --reload-config        Ask a running daemon to reload config from disk
      --status               Print daemon runtime status
      --health               Print daemon build and platform info
      --doctor               Check local runtime prerequisites without locking
      --check-config         Validate config files without starting the daemon
      --stop                 Stop the running daemon

Themes:
      --list-themes          List bundled themes
      --current-theme        Print the active theme selection
      --print-theme=<name>   Print a theme source file
      --set-theme=<name>     Set the active theme in config.toml
      --unset-theme          Remove the top-level theme key from config.toml

Notes:
  Prefer `veila lock`, `veila status`, `veila reload`, and `veila theme ...` for user-facing control.
  Only one control action can be used at a time.
  If no control action is given, {name} starts the daemon.
  --log-file only affects that daemon-start path.
  --set-theme creates config.toml automatically if it does not exist.
",
        name = component_name()
    );
}

fn print_control_help() {
    println!(
        "\
Veila control CLI

Usage:
  veila <command> [options]

General:
  -h, --help                 Show this help text
      --version              Print the local Veila version
      --config=<path>        Use a specific config file for theme/config commands
      --force-emergency-ui   Combine with `lock` to test the emergency unlock prompt
      --latency-report[=verbose]
                             Combine with `lock --wait-ready` to print startup timings

Commands:
  lock [--wait-ready]        Ask the running daemon to lock now
  status                     Print daemon runtime status
  health                     Print daemon build and platform info
  doctor                     Check local runtime prerequisites without locking
  check-config               Validate config files without starting the daemon
  reload                     Ask the running daemon to reload config from disk
  stop                       Stop the running daemon

Themes:
  theme list                 List bundled themes
  theme current              Print the active theme selection
  theme print <name>         Print a theme source file
  theme set <name>           Set the active theme in config.toml
  theme unset                Remove the top-level theme key from config.toml

Notes:
  This command never starts the daemon. Start it with `veilad`, a user service, or your compositor config.
  `--wait-ready` can be combined with `veila lock` to block until the secure lock is active.
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
