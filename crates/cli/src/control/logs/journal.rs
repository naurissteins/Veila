use super::super::IDLE_SERVICE;
use crate::options::{LogOptions, LogTarget};
use std::process::Command;

pub(super) fn command(options: &LogOptions) -> Command {
    let mut command = Command::new("journalctl");
    command.args([
        "--user",
        "--boot",
        "--no-pager",
        "--output=cat",
        "-u",
        IDLE_SERVICE,
    ]);
    match options.target {
        LogTarget::All => {}
        LogTarget::Idle => {
            command.arg("_COMM=veila-idle");
        }
        LogTarget::Curtain => {
            command.arg("_COMM=veila-curtain");
        }
        LogTarget::Ui => {
            command.arg("--grep=veila_ui");
        }
    }
    if options.follow {
        command.arg("--follow");
    }
    if let Some(since) = &options.since {
        command.arg(format!("--since={}", normalize_since(since)));
    }
    if let Some(lines) = options.lines.or_else(|| {
        options
            .since
            .is_none()
            .then_some(if options.follow { 50 } else { 200 })
    }) {
        command.arg(format!("--lines={lines}"));
    }
    command
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
