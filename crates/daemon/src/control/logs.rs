use std::{
    collections::VecDeque,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    process::Command,
};

use anyhow::{Context, Result, bail};
use veila_common::AppConfig;

use super::DAEMON_SERVICE;
use crate::{
    DaemonOptions,
    adapters::process::{CURTAIN_PROCESS_NAME, DAEMON_PROCESS_NAME},
    logging::normalize_log_file_path,
    options::LogTarget,
};

pub fn print_logs(options: &DaemonOptions) -> Result<()> {
    if options.logs_file {
        return print_file_logs(options);
    }

    let mut command = Command::new("journalctl");
    command
        .arg("--user")
        .arg("--boot")
        .arg("--no-pager")
        .arg("--output=cat");

    apply_target(&mut command, options.logs_target);

    if options.logs_follow {
        command.arg("--follow");
    }

    if let Some(since) = options.logs_since.as_deref() {
        command.arg(format!("--since={}", normalize_since(since)));
    } else {
        command.arg(format!(
            "--lines={}",
            options
                .logs_lines
                .unwrap_or(if options.logs_follow { 50 } else { 200 })
        ));
    }

    if let Some(lines) = options.logs_lines
        && options.logs_since.is_some()
    {
        command.arg(format!("--lines={lines}"));
    }

    let status = command
        .status()
        .context("failed to run journalctl; veila logs reads the systemd user journal")?;
    if !status.success() {
        bail!("journalctl exited with {status}");
    }

    Ok(())
}

fn print_file_logs(options: &DaemonOptions) -> Result<()> {
    if options.logs_since.is_some() {
        bail!("--since is only supported for journal logs");
    }

    if !matches!(
        options.logs_target,
        LogTarget::LockService | LogTarget::Daemon
    ) {
        bail!("component filters are only supported for journal logs");
    }

    let loaded = AppConfig::load(options.config_path.as_deref())
        .context("failed to load config for file log path")?;
    let configured_path = normalize_log_file_path(&loaded.config.lock.log_file_path);

    if !loaded.config.lock.log_to_file {
        println!("file logging is not enabled");
        println!();
        println!("Add this to your Veila config:");
        println!();
        println!("[lock]");
        println!("log_to_file = true");
        println!(
            "log_file_path = \"{}\"",
            loaded.config.lock.log_file_path.display()
        );
        println!();
        println!("Then restart the Veila daemon so it starts writing to the file.");
        return Ok(());
    }

    if !configured_path.exists() {
        println!("file logging is enabled, but the log file does not exist yet");
        println!("path={}", configured_path.display());
        println!();
        println!("Restart the Veila daemon and try again after it has written its first log line.");
        return Ok(());
    }

    let lines = options
        .logs_lines
        .unwrap_or(if options.logs_follow { 50 } else { 200 });

    if options.logs_follow {
        follow_file_logs(&configured_path, lines)?;
    } else {
        print_recent_file_lines(&configured_path, lines)?;
    }

    Ok(())
}

fn follow_file_logs(path: &Path, lines: u32) -> Result<()> {
    let status = Command::new("tail")
        .arg("-n")
        .arg(lines.to_string())
        .arg("-f")
        .arg(path)
        .status()
        .with_context(|| format!("failed to run tail for {}", path.display()))?;
    if !status.success() {
        bail!("tail exited with {status}");
    }

    Ok(())
}

fn print_recent_file_lines(path: &Path, lines: u32) -> Result<()> {
    if lines == 0 {
        return Ok(());
    }

    let file =
        File::open(path).with_context(|| format!("failed to open log file {}", path.display()))?;
    let mut recent = VecDeque::with_capacity(lines as usize);

    for line in BufReader::new(file).lines() {
        if recent.len() == lines as usize {
            recent.pop_front();
        }
        recent.push_back(
            line.with_context(|| format!("failed to read log file {}", path.display()))?,
        );
    }

    for line in recent {
        println!("{line}");
    }

    Ok(())
}

fn apply_target(command: &mut Command, target: LogTarget) {
    match target {
        LogTarget::LockService => {
            command.arg("-u").arg(DAEMON_SERVICE);
        }
        LogTarget::Daemon => {
            command
                .arg("-u")
                .arg(DAEMON_SERVICE)
                .arg(format!("_COMM={DAEMON_PROCESS_NAME}"));
        }
        LogTarget::Curtain => {
            command
                .arg("-u")
                .arg(DAEMON_SERVICE)
                .arg(format!("_COMM={CURTAIN_PROCESS_NAME}"));
        }
        LogTarget::Ui => {
            command.arg("-u").arg(DAEMON_SERVICE).arg("--grep=veila_ui");
        }
    }
}

fn normalize_since(value: &str) -> String {
    let trimmed = value.trim();
    let Some((number, unit)) = split_duration_suffix(trimmed) else {
        return trimmed.to_string();
    };

    let unit_name = match unit {
        "s" => "seconds",
        "m" => "minutes",
        "h" => "hours",
        "d" => "days",
        "w" => "weeks",
        _ => return trimmed.to_string(),
    };

    format!("{number} {unit_name} ago")
}

fn split_duration_suffix(value: &str) -> Option<(&str, &str)> {
    let unit_start = value.find(|character: char| !character.is_ascii_digit())?;
    if unit_start == 0 || unit_start == value.len() {
        return None;
    }

    let (number, unit) = value.split_at(unit_start);
    if unit.chars().count() != 1 {
        return None;
    }

    Some((number, unit))
}

#[cfg(test)]
mod tests {
    use super::normalize_since;

    #[test]
    fn leaves_journalctl_since_text_unchanged() {
        assert_eq!(normalize_since("today"), "today");
        assert_eq!(normalize_since("2026-05-17 12:00"), "2026-05-17 12:00");
    }

    #[test]
    fn expands_short_since_durations() {
        assert_eq!(normalize_since("10m"), "10 minutes ago");
        assert_eq!(normalize_since("2h"), "2 hours ago");
    }
}
