use std::path::PathBuf;

use anyhow::{Result, bail};

use super::DaemonOptions;

impl DaemonOptions {
    pub fn parse_daemon_args(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self::default();

        for arg in args.into_iter().skip(1) {
            if arg == "--help" || arg == "-h" {
                options.help = true;
                continue;
            }

            if let Some(path) = arg.strip_prefix("--config=") {
                options.config_path = Some(PathBuf::from(path));
                continue;
            }

            if let Some(path) = arg.strip_prefix("--log-file=") {
                options.log_file_path = Some(PathBuf::from(path));
                continue;
            }

            if let Some(session_id) = arg.strip_prefix("--session-id=") {
                options.session_id = Some(session_id.to_string());
                continue;
            }

            if let Some(replacement) = removed_flag_replacement(&arg) {
                bail!("`{arg}` is no longer a daemon option; use `{replacement}` instead");
            }

            bail!("unknown daemon argument: {arg}");
        }

        Ok(options)
    }
}

fn removed_flag_replacement(arg: &str) -> Option<&'static str> {
    let flag = arg.split_once('=').map_or(arg, |(flag, _)| flag);
    let replacement = match flag {
        "--lock-now" | "--wait-ready" | "--force-emergency-ui" | "--latency-report" => "veila lock",
        "--status" => "veila status",
        "--health" => "veila health",
        "--doctor" => "veila doctor",
        "--check-config" => "veila check-config",
        "--reload-config" => "veila reload",
        "--stop" => "veila stop",
        "--version" | "-v" => "veila --version",
        "--list-themes" => "veila theme list",
        "--current-theme" => "veila theme current",
        "--print-theme" => "veila theme print <name>",
        "--set-theme" => "veila theme set <name>",
        "--unset-theme" => "veila theme unset",
        _ => return None,
    };
    Some(replacement)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::DaemonOptions;

    fn parse(args: &[&str]) -> anyhow::Result<DaemonOptions> {
        DaemonOptions::parse_daemon_args(
            std::iter::once("veila")
                .chain(args.iter().copied())
                .map(String::from),
        )
    }

    #[test]
    fn parses_daemon_start_options() {
        let options = parse(&[
            "--config=/tmp/veila.toml",
            "--log-file=/tmp/veila.log",
            "--session-id=c2",
        ])
        .expect("arguments should parse");

        assert_eq!(
            options.config_path.as_deref(),
            Some(Path::new("/tmp/veila.toml"))
        );
        assert_eq!(
            options.log_file_path.as_deref(),
            Some(Path::new("/tmp/veila.log"))
        );
        assert_eq!(options.session_id.as_deref(), Some("c2"));
        assert!(!options.help);
    }

    #[test]
    fn parses_help_arguments() {
        assert!(parse(&["--help"]).expect("long help").help);
        assert!(parse(&["-h"]).expect("short help").help);
    }

    #[test]
    fn empty_arguments_start_the_daemon_with_defaults() {
        assert_eq!(parse(&[]).expect("no arguments"), DaemonOptions::default());
    }

    #[test]
    fn removed_control_flags_point_to_veila_commands() {
        let cases = [
            ("--lock-now", "veila lock"),
            ("--latency-report=verbose", "veila lock"),
            ("--status", "veila status"),
            ("--reload-config", "veila reload"),
            ("--set-theme=beach", "veila theme set <name>"),
            ("--background-prewarm-only", "unknown daemon argument"),
        ];

        for (flag, expected) in cases {
            let error = parse(&[flag]).expect_err("removed flag should fail");
            assert!(
                error.to_string().contains(expected),
                "{flag}: unexpected error {error}"
            );
        }
    }

    #[test]
    fn rejects_unknown_arguments() {
        let error = parse(&["lock"]).expect_err("positional should fail");

        assert!(error.to_string().contains("unknown daemon argument: lock"));
    }
}
