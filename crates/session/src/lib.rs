#![forbid(unsafe_code)]
mod app;
mod latency;
pub mod logging;
mod options;
mod record;
mod snapshots;
mod spawn;
pub use options::LockOptions;

pub fn run(options: LockOptions, args: Vec<String>) -> anyhow::Result<i32> {
    if options.help {
        println!(
            "Veila internal lock supervisor\nUsage: veila __lock [--daemonize] [--wait-ready] [--config=PATH] [--force-emergency-ui] [--latency-report[=verbose]] [--notify-fd=N]"
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
