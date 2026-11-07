use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use veila_common::ipc::LatencyReportMode;

#[derive(Debug, Default)]
pub struct LockOptions {
    pub config: Option<PathBuf>,
    pub daemonize: bool,
    pub wait_ready: bool,
    pub emergency: bool,
    pub latency: LatencyReportMode,
    pub notify_fd: Option<i32>,
    pub background_fd: Option<i32>,
    pub help: bool,
}

impl LockOptions {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self::default();
        let mut args = args.into_iter().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => options.help = true,
                "--daemonize" => options.daemonize = true,
                "--wait-ready" => options.wait_ready = true,
                "--force-emergency-ui" => options.emergency = true,
                "--latency-report" => options.latency = LatencyReportMode::Basic,
                "--latency-report=verbose" => options.latency = LatencyReportMode::Verbose,
                "--latency-report=basic" => options.latency = LatencyReportMode::Basic,
                _ => {
                    let (key, value) = if let Some(pair) = arg.split_once('=') {
                        (pair.0.to_owned(), pair.1.to_owned())
                    } else {
                        (arg, args.next().context("option requires a value")?)
                    };
                    if value.starts_with("--") || value.is_empty() {
                        bail!("{key} requires a value");
                    }
                    match key.as_str() {
                        "--config" => options.config = Some(value.into()),
                        "--notify-fd" => options.notify_fd = Some(parse_fd(&value)?),
                        "--background-fd" => options.background_fd = Some(parse_fd(&value)?),
                        _ => bail!("unknown lock supervisor argument: {key}"),
                    }
                }
            }
        }
        if options.background_fd.is_some() && options.daemonize {
            bail!("background mode cannot daemonize again");
        }
        if options.notify_fd.is_some() && options.notify_fd == options.background_fd {
            bail!("notification descriptors must differ");
        }
        Ok(options)
    }
}
fn parse_fd(value: &str) -> Result<i32> {
    let fd: i32 = value.parse().context("invalid notification descriptor")?;
    if fd < 3 {
        bail!("notification descriptor must be at least 3");
    }
    Ok(fd)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<LockOptions> {
        LockOptions::parse(
            std::iter::once("veila")
                .chain(args.iter().copied())
                .map(str::to_owned),
        )
    }
    #[test]
    fn parses_internal_lock_options_and_both_value_forms() {
        let o = parse(&[
            "--daemonize",
            "--wait-ready",
            "--config",
            "/tmp/config",
            "--latency-report=verbose",
            "--force-emergency-ui",
            "--notify-fd=9",
        ])
        .expect("parse");
        assert!(o.daemonize && o.wait_ready && o.emergency);
        assert_eq!(o.config, Some("/tmp/config".into()));
        assert_eq!(o.notify_fd, Some(9));
        assert!(o.latency.is_verbose());
    }
    #[test]
    fn rejects_missing_invalid_and_conflicting_options() {
        for args in [
            &["--config"][..],
            &["--notify-fd=0"],
            &["--notify-fd=bad"],
            &["--background-fd=3", "--daemonize"],
            &["--background-fd=3", "--notify-fd=3"],
            &["--unknown=1"],
            &["--config", "--wait-ready"],
            &["--config="],
        ] {
            assert!(parse(args).is_err(), "{args:?}");
        }
    }
}
