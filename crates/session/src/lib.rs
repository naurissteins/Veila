#![forbid(unsafe_code)]
pub mod adapters;
mod app;
mod duplicate;
mod idle;
mod integrations;
pub use idle::{IdleOptions, run as run_idle};
mod latency;
pub mod logging;
mod options;
pub mod output_probe;
mod prewarm;
mod process_status;
pub use process_status::{ProcessStatus, RuntimeStatus, reload, runtime_status};
mod record;
pub mod services;
mod spawn;
pub use options::LockOptions;

pub fn run(options: LockOptions, args: Vec<String>) -> anyhow::Result<i32> {
    if options.help {
        println!(
            "Veila screen lock\nUsage: veila lock [--daemonize] [--wait-ready] [--config=PATH] [--force-emergency-ui] [--latency-report[=verbose]] [--notify-fd=N]"
        );
        return Ok(0);
    }
    if options.daemonize {
        return spawn::background(options, args);
    }
    if options.background_fd.is_some() {
        nix::unistd::setsid()?;
    }
    app::run(options)
}

pub async fn run_prewarm(path: Option<&std::path::Path>) -> anyhow::Result<()> {
    let loaded = veila_common::AppConfig::load(path)?;
    prewarm::run_background_prewarm_once(loaded.config).await;
    Ok(())
}
