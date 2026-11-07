#![forbid(unsafe_code)]

mod cli;
mod logging;

use std::{ffi::CString, future::Future};

use anyhow::{Context, Result};
use nix::sys::prctl;
use veila_auth::PAM_HELPER_PROCESS_NAME;
use veila_curtain::CurtainOptions;
use veila_daemon::{
    CURTAIN_PROCESS_NAME, ControlOptions, DAEMON_PROCESS_NAME, DaemonOptions, PREWARM_PROCESS_NAME,
};

use cli::{Invocation, Mode};

fn main() -> Result<()> {
    let Invocation { mode, args } = Invocation::parse(std::env::args().collect());

    match mode {
        Mode::Lock => {
            harden_process("veila-lock")?;
            let options = veila_session::LockOptions::parse(args.clone())?;
            logging::init_stderr();
            let code = veila_session::run(options, args)?;
            std::process::exit(code);
        }
        Mode::Control => block_on(veila_daemon::run_control(ControlOptions::parse_args(args)?)),
        Mode::Daemon | Mode::LegacyDaemon => run_daemon(args, mode == Mode::LegacyDaemon),
        Mode::Prewarm => {
            harden_process(PREWARM_PROCESS_NAME)?;
            let options = DaemonOptions::parse_daemon_args(args)?;
            logging::init_stderr();
            block_on(veila_daemon::run_prewarm(options))
        }
        Mode::FontQuery => {
            harden_process("veila-fonts")?;
            Ok(veila_renderer::text::run_font_query(args)?)
        }
        Mode::PamHelper => {
            harden_process(PAM_HELPER_PROCESS_NAME)?;
            Ok(veila_auth::run_helper()?)
        }
        Mode::Curtain => {
            harden_process(CURTAIN_PROCESS_NAME)?;
            let options = CurtainOptions::parse_args(args)?;
            logging::init_stderr();
            if !options.help {
                tracing::info!(component = CURTAIN_PROCESS_NAME, "starting curtain");
            }
            match veila_curtain::run(options) {
                Err(error) if error.downcast_ref::<veila_curtain::ExitFailure>().is_some() => {
                    let code = error
                        .downcast_ref::<veila_curtain::ExitFailure>()
                        .map_or(1, |exit| exit.0);
                    tracing::error!(%error,"curtain stopped");
                    std::process::exit(code);
                }
                result => result,
            }
        }
        Mode::Preview => {
            let options = CurtainOptions::parse_args(args)?;
            logging::init_stderr();
            veila_curtain::run_preview(options)
        }
    }
}

fn run_daemon(args: Vec<String>, legacy_name: bool) -> Result<()> {
    harden_process(DAEMON_PROCESS_NAME)?;
    let options = DaemonOptions::parse_daemon_args(args)?;
    let log_file = veila_daemon::daemon_log_file_path(&options)?;
    let _log_guard = logging::init_daemon(log_file.as_deref())?;

    if legacy_name {
        tracing::warn!("`veilad` is deprecated; start the daemon with `veila daemon`");
    }
    if !options.help {
        tracing::info!(component = DAEMON_PROCESS_NAME, "starting daemon");
    }

    block_on(veila_daemon::run_daemon(options))
}

// Secret-handling modes must not be dumpable or ptrace-able by same-UID processes.
fn harden_process(name: &str) -> Result<()> {
    prctl::set_dumpable(false).context("failed to disable process dumpability")?;
    let name = CString::new(name).context("process name contains a NUL byte")?;
    prctl::set_name(&name).context("failed to set process name")
}

fn block_on(future: impl Future<Output = Result<()>>) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to start the async runtime")?
        .block_on(future)
}
