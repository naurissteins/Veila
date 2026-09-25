use std::path::PathBuf;

use anyhow::{Result, bail};
use veila_common::ipc::LatencyReportMode;

use super::{DaemonOptions, LogTarget};

impl DaemonOptions {
    pub fn parse_control_args(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self::default();
        let mut positional = Vec::new();

        for arg in args.into_iter().skip(1) {
            if arg == "--help" || arg == "-h" {
                options.help = true;
                continue;
            }

            if arg == "--version" || arg == "-v" {
                options.version = true;
                continue;
            }

            if let Some(path) = arg.strip_prefix("--config=") {
                options.config_path = Some(PathBuf::from(path));
                continue;
            }

            if arg == "--wait-ready" {
                options.wait_ready = true;
                continue;
            }

            if arg == "--force-emergency-ui" {
                options.force_emergency_ui = true;
                continue;
            }

            if let Some(mode) = parse_latency_report_arg(&arg)? {
                options.latency_report = mode;
                continue;
            }

            if arg.starts_with("--") && positional.is_empty() {
                bail!("unknown veila option: {arg}");
            }

            positional.push(arg);
        }

        apply_control_positionals(&mut options, &positional)?;
        Ok(options)
    }
}

fn parse_latency_report_arg(arg: &str) -> Result<Option<LatencyReportMode>> {
    if arg == "--latency-report" {
        return Ok(Some(LatencyReportMode::Basic));
    }

    let Some(mode) = arg.strip_prefix("--latency-report=") else {
        return Ok(None);
    };

    match mode {
        "basic" => Ok(Some(LatencyReportMode::Basic)),
        "verbose" => Ok(Some(LatencyReportMode::Verbose)),
        _ => bail!("unknown latency report mode: {mode}"),
    }
}

fn apply_control_positionals(options: &mut DaemonOptions, positional: &[String]) -> Result<()> {
    let Some(command) = positional.first().map(String::as_str) else {
        return Ok(());
    };

    match command {
        "lock" => expect_no_extra_args(command, &positional[1..], || options.lock_now = true),
        "status" => expect_no_extra_args(command, &positional[1..], || options.status = true),
        "health" => expect_no_extra_args(command, &positional[1..], || options.health = true),
        "doctor" => expect_no_extra_args(command, &positional[1..], || options.doctor = true),
        "check-config" => {
            expect_no_extra_args(command, &positional[1..], || options.check_config = true)
        }
        "init" => apply_init_positionals(options, &positional[1..]),
        "reload" => {
            expect_no_extra_args(command, &positional[1..], || options.reload_config = true)
        }
        "stop" => expect_no_extra_args(command, &positional[1..], || options.stop = true),
        "idle" => apply_idle_positionals(options, &positional[1..]),
        "logs" => apply_logs_positionals(options, &positional[1..]),
        "theme" => apply_theme_positionals(options, &positional[1..]),
        "daemon" | "preview" => {
            bail!(
                "`{command}` must be the first argument, for example `veila {command} --config=<path>`"
            )
        }
        _ => bail!("unknown veila command: {command}"),
    }
}

fn apply_init_positionals(options: &mut DaemonOptions, args: &[String]) -> Result<()> {
    options.init_config = true;
    let mut index = 0;

    while let Some(arg) = args.get(index) {
        if arg == "--force" {
            options.init_force = true;
            index += 1;
            continue;
        }

        if let Some(theme) = arg.strip_prefix("--theme=") {
            options.init_theme = Some(theme.to_string());
            index += 1;
            continue;
        }

        if arg == "--theme" {
            let Some(theme) = args.get(index + 1) else {
                bail!("missing value for --theme");
            };
            options.init_theme = Some(theme.clone());
            index += 2;
            continue;
        }

        bail!("unexpected extra argument for init: {arg}");
    }

    Ok(())
}

fn apply_idle_positionals(options: &mut DaemonOptions, args: &[String]) -> Result<()> {
    options.idle = true;
    let mut index = 0;

    while let Some(arg) = args.get(index) {
        if let Some(value) = arg.strip_prefix("--lock-after=") {
            options.idle_lock_after_seconds = Some(parse_nonzero_seconds(value, "--lock-after")?);
            index += 1;
            continue;
        }

        if arg == "--lock-after" {
            let Some(value) = args.get(index + 1) else {
                bail!("missing value for --lock-after");
            };
            options.idle_lock_after_seconds = Some(parse_nonzero_seconds(value, "--lock-after")?);
            index += 2;
            continue;
        }

        if arg == "--lock-before-sleep" {
            options.idle_lock_before_sleep = true;
            index += 1;
            continue;
        }

        bail!("unexpected extra argument for idle: {arg}");
    }

    Ok(())
}

fn apply_logs_positionals(options: &mut DaemonOptions, args: &[String]) -> Result<()> {
    options.logs = true;
    let mut explicit_target = false;
    let mut index = 0;

    while let Some(arg) = args.get(index) {
        if arg == "--follow" || arg == "-f" {
            options.logs_follow = true;
            index += 1;
            continue;
        }

        if arg == "--file" {
            options.logs_file = true;
            index += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--since=") {
            options.logs_since = Some(value.to_string());
            index += 1;
            continue;
        }

        if arg == "--since" {
            let Some(value) = args.get(index + 1) else {
                bail!("missing value for --since");
            };
            options.logs_since = Some(value.clone());
            index += 2;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--lines=") {
            options.logs_lines = Some(parse_log_lines(value)?);
            index += 1;
            continue;
        }

        if arg == "--lines" || arg == "-n" {
            let Some(value) = args.get(index + 1) else {
                bail!("missing value for {arg}");
            };
            options.logs_lines = Some(parse_log_lines(value)?);
            index += 2;
            continue;
        }

        if let Some(target) = parse_log_target_flag(arg) {
            if explicit_target {
                bail!("use only one logs target filter at a time");
            }
            options.logs_target = target;
            explicit_target = true;
            index += 1;
            continue;
        }

        bail!("unexpected extra argument for logs: {arg}");
    }

    Ok(())
}

fn parse_log_lines(value: &str) -> Result<u32> {
    value
        .parse::<u32>()
        .map_err(|_| anyhow::anyhow!("--lines must be a non-negative integer"))
}

fn parse_log_target_flag(arg: &str) -> Option<LogTarget> {
    match arg {
        "--all" => Some(LogTarget::All),
        "--daemon" => Some(LogTarget::Daemon),
        "--curtain" => Some(LogTarget::Curtain),
        "--ui" => Some(LogTarget::Ui),
        "--idle" => Some(LogTarget::Idle),
        _ => None,
    }
}

fn parse_nonzero_seconds(value: &str, label: &str) -> Result<u64> {
    let seconds = value
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("{label} must be a positive integer number of seconds"))?;
    if seconds == 0 {
        bail!("{label} must be at least 1 second");
    }
    Ok(seconds)
}

fn apply_theme_positionals(options: &mut DaemonOptions, args: &[String]) -> Result<()> {
    let Some(command) = args.first().map(String::as_str) else {
        bail!("missing theme command");
    };

    match command {
        "list" => expect_no_extra_args("theme list", &args[1..], || options.list_themes = true),
        "current" => expect_no_extra_args("theme current", &args[1..], || {
            options.current_theme = true;
        }),
        "unset" => expect_no_extra_args("theme unset", &args[1..], || options.unset_theme = true),
        "print" => {
            let Some(theme) = args.get(1) else {
                bail!("missing theme name for theme print");
            };
            if args.len() > 2 {
                bail!("unexpected extra argument for theme print: {}", args[2]);
            }
            options.print_theme = Some(theme.clone());
            Ok(())
        }
        "set" => {
            let Some(theme) = args.get(1) else {
                bail!("missing theme name for theme set");
            };
            if args.len() > 2 {
                bail!("unexpected extra argument for theme set: {}", args[2]);
            }
            options.set_theme = Some(theme.clone());
            Ok(())
        }
        _ => bail!("unknown theme command: {command}"),
    }
}

fn expect_no_extra_args(command: &str, args: &[String], apply: impl FnOnce()) -> Result<()> {
    if let Some(extra) = args.first() {
        bail!("unexpected extra argument for {command}: {extra}");
    }
    apply();
    Ok(())
}
