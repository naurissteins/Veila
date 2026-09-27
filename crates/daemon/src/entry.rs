use anyhow::Result;

use crate::{DaemonOptions, adapters::ipc, app};

pub async fn run_daemon(options: DaemonOptions) -> Result<()> {
    if options.help {
        print_daemon_help();
        return Ok(());
    }

    let daemon_socket_path = ipc::daemon_socket_path()?;
    let control_listener = ipc::bind_single_instance_listener(&daemon_socket_path).await?;
    app::run(options, control_listener, daemon_socket_path).await
}

pub async fn run_prewarm(options: DaemonOptions) -> Result<()> {
    app::run_background_prewarm_once(options.config_path.as_deref()).await
}

fn print_daemon_help() {
    println!(
        "\
Veila daemon

Usage:
  veila daemon [options]

Options:
  -h, --help                 Show this help text
      --config=<path>        Use a specific config file
      --log-file=<path>      Append daemon logs to a file
      --session-id=<id>      Override the logind session id

Notes:
  Control the running daemon with `veila lock`, `veila status`, `veila reload`, and `veila stop`.
  --log-file overrides `[lock].log_to_file` and `[lock].log_file_path` from config.toml.
"
    );
}
