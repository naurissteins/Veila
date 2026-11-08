use nix::{
    sys::{prctl, signal::Signal},
    unistd::getppid,
};
use std::{io, os::unix::process::CommandExt, process::Command};

pub const FONT_QUERY_SUBCOMMAND: &str = "__font-query";
pub(super) const PARENT_ENV: &str = "VEILA_FONT_PARENT_PID";
pub(super) const QUERY_FORMAT: &str = "%{file}\u{1f}%{index}\u{1f}";

pub fn run_font_query(args: Vec<String>) -> io::Result<()> {
    let parent: i32 = std::env::var(PARENT_ENV)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|pid| *pid > 1)
        .ok_or_else(|| io::Error::other("font query requires its parent identity"))?;
    prctl::set_pdeathsig(Some(Signal::SIGKILL))?;
    // check after arming the signal to close the parent-exit race before exec
    if getppid().as_raw() != parent {
        return Err(io::Error::other("font query parent changed"));
    }
    if args.len() != 5
        || args[1] != "-f"
        || args[2] != QUERY_FORMAT
        || args[3] != "--"
        || args[4].len() > 1024
    {
        return Err(io::Error::other("invalid font query arguments"));
    }
    Err(Command::new("fc-match").args(&args[1..]).exec())
}
