use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum LogTarget {
    #[default]
    LockService,
    Daemon,
    Curtain,
    Ui,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LogOptions {
    pub(crate) file: bool,
    pub(crate) follow: bool,
    pub(crate) since: Option<String>,
    pub(crate) lines: Option<u32>,
    pub(crate) target: LogTarget,
}

pub(super) fn parse_logs(args: &[String]) -> Result<LogOptions> {
    let mut options = LogOptions::default();
    let mut explicit_target = false;
    let mut index = 0;

    while let Some(arg) = args.get(index) {
        if arg == "--follow" || arg == "-f" {
            options.follow = true;
            index += 1;
            continue;
        }

        if arg == "--file" {
            options.file = true;
            index += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--since=") {
            options.since = Some(value.to_string());
            index += 1;
            continue;
        }

        if arg == "--since" {
            let Some(value) = args.get(index + 1) else {
                bail!("missing value for --since");
            };
            options.since = Some(value.clone());
            index += 2;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--lines=") {
            options.lines = Some(parse_log_lines(value)?);
            index += 1;
            continue;
        }

        if arg == "--lines" || arg == "-n" {
            let Some(value) = args.get(index + 1) else {
                bail!("missing value for {arg}");
            };
            options.lines = Some(parse_log_lines(value)?);
            index += 2;
            continue;
        }

        if let Some(target) = parse_log_target_flag(arg) {
            if explicit_target {
                bail!("use only one logs target filter at a time");
            }
            options.target = target;
            explicit_target = true;
            index += 1;
            continue;
        }

        bail!("unexpected extra argument for logs: {arg}");
    }

    Ok(options)
}

fn parse_log_lines(value: &str) -> Result<u32> {
    value
        .parse::<u32>()
        .map_err(|_| anyhow::anyhow!("--lines must be a non-negative integer"))
}

fn parse_log_target_flag(arg: &str) -> Option<LogTarget> {
    match arg {
        "--all" => Some(LogTarget::LockService),
        "--daemon" => Some(LogTarget::Daemon),
        "--curtain" => Some(LogTarget::Curtain),
        "--ui" => Some(LogTarget::Ui),
        _ => None,
    }
}
