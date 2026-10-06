use std::path::PathBuf;

use anyhow::{Result, bail};
use veila_common::ipc::LatencyReportMode;

use super::{ControlCommand, ControlOptions, LockOptions, logs::parse_logs};

impl ControlOptions {
    pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self::default();
        let mut lock = LockOptions::default();
        let mut version = false;
        let mut positional = Vec::new();

        for arg in args.into_iter().skip(1) {
            if arg == "--help" || arg == "-h" {
                options.help = true;
                continue;
            }

            if arg == "--version" || arg == "-v" {
                version = true;
                continue;
            }

            if let Some(path) = arg.strip_prefix("--config=") {
                options.config_path = Some(PathBuf::from(path));
                continue;
            }

            if arg == "--wait-ready" {
                lock.wait_ready = true;
                continue;
            }

            if arg == "--force-emergency-ui" {
                lock.force_emergency_ui = true;
                continue;
            }

            if let Some(mode) = parse_latency_report_arg(&arg)? {
                lock.latency_report = mode;
                continue;
            }

            if arg.starts_with("--") && positional.is_empty() {
                bail!("unknown veila option: {arg}");
            }

            positional.push(arg);
        }

        options.command = parse_control_positionals(&positional, lock)?;
        // Help bypasses command compatibility checks, but never syntax errors.
        if !options.help {
            if version && options.command.is_some() {
                bail!("use only one veila command at a time");
            }
            if version {
                options.command = Some(ControlCommand::Version);
            }
            lock.validate_command(options.command.as_ref())?;
        }
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

fn parse_control_positionals(
    positional: &[String],
    lock: LockOptions,
) -> Result<Option<ControlCommand>> {
    let Some(command) = positional.first().map(String::as_str) else {
        return Ok(None);
    };

    let command = match command {
        "lock" => expect_no_extra_args(command, &positional[1..], ControlCommand::Lock(lock)),
        "status" => expect_no_extra_args(command, &positional[1..], ControlCommand::Status),
        "health" => expect_no_extra_args(command, &positional[1..], ControlCommand::Health),
        "doctor" => expect_no_extra_args(command, &positional[1..], ControlCommand::Doctor),
        "check-config" => {
            expect_no_extra_args(command, &positional[1..], ControlCommand::CheckConfig)
        }
        "init" => parse_init(&positional[1..]),
        "reload" => expect_no_extra_args(command, &positional[1..], ControlCommand::Reload),
        "stop" => expect_no_extra_args(command, &positional[1..], ControlCommand::Stop),
        "idle" => bail!(
            "`veila idle` was removed; set `enabled = true` in the [idle] section of config.toml and run `veila reload`"
        ),
        "logs" => parse_logs(&positional[1..]).map(ControlCommand::Logs),
        "theme" => parse_theme(&positional[1..]),
        "daemon" | "preview" => {
            bail!(
                "`{command}` must be the first argument, for example `veila {command} --config=<path>`"
            )
        }
        _ => bail!("unknown veila command: {command}"),
    }?;
    Ok(Some(command))
}

fn parse_init(args: &[String]) -> Result<ControlCommand> {
    let mut theme = None;
    let mut force = false;
    let mut index = 0;

    while let Some(arg) = args.get(index) {
        if arg == "--force" {
            force = true;
            index += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--theme=") {
            theme = Some(value.to_string());
            index += 1;
            continue;
        }

        if arg == "--theme" {
            let Some(value) = args.get(index + 1) else {
                bail!("missing value for --theme");
            };
            theme = Some(value.clone());
            index += 2;
            continue;
        }

        bail!("unexpected extra argument for init: {arg}");
    }

    Ok(ControlCommand::Init { theme, force })
}

fn parse_theme(args: &[String]) -> Result<ControlCommand> {
    let Some(command) = args.first().map(String::as_str) else {
        bail!("missing theme command");
    };

    match command {
        "list" => expect_no_extra_args("theme list", &args[1..], ControlCommand::ThemeList),
        "current" => {
            expect_no_extra_args("theme current", &args[1..], ControlCommand::ThemeCurrent)
        }
        "unset" => expect_no_extra_args("theme unset", &args[1..], ControlCommand::ThemeUnset),
        "print" => {
            let Some(theme) = args.get(1) else {
                bail!("missing theme name for theme print");
            };
            if args.len() > 2 {
                bail!("unexpected extra argument for theme print: {}", args[2]);
            }
            Ok(ControlCommand::ThemePrint(theme.clone()))
        }
        "set" => {
            let Some(theme) = args.get(1) else {
                bail!("missing theme name for theme set");
            };
            if args.len() > 2 {
                bail!("unexpected extra argument for theme set: {}", args[2]);
            }
            Ok(ControlCommand::ThemeSet(theme.clone()))
        }
        _ => bail!("unknown theme command: {command}"),
    }
}

fn expect_no_extra_args(
    command: &str,
    args: &[String],
    selected: ControlCommand,
) -> Result<ControlCommand> {
    if let Some(extra) = args.first() {
        bail!("unexpected extra argument for {command}: {extra}");
    }
    Ok(selected)
}
