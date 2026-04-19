use std::path::PathBuf;

use anyhow::{Result, bail};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonOptions {
    pub config_path: Option<PathBuf>,
    pub log_file_path: Option<PathBuf>,
    pub session_id: Option<String>,
    pub help: bool,
    pub current_theme: bool,
    pub print_theme: Option<String>,
    pub set_theme: Option<String>,
    pub unset_theme: bool,
    pub lock_now: bool,
    pub stop: bool,
    pub list_themes: bool,
    pub status: bool,
    pub health: bool,
    pub version: bool,
    pub reload_config: bool,
}

impl DaemonOptions {
    pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Self> {
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

            if arg == "--current-theme" {
                options.current_theme = true;
                continue;
            }

            if let Some(theme) = arg.strip_prefix("--print-theme=") {
                options.print_theme = Some(theme.to_string());
                continue;
            }

            if let Some(theme) = arg.strip_prefix("--set-theme=") {
                options.set_theme = Some(theme.to_string());
                continue;
            }

            if arg == "--unset-theme" {
                options.unset_theme = true;
                continue;
            }

            if arg == "--lock-now" {
                options.lock_now = true;
                continue;
            }

            if arg == "--stop" {
                options.stop = true;
                continue;
            }

            if arg == "--list-themes" {
                options.list_themes = true;
                continue;
            }

            if arg == "--status" {
                options.status = true;
                continue;
            }

            if arg == "--health" {
                options.health = true;
                continue;
            }

            if arg == "--version" {
                options.version = true;
                continue;
            }

            if arg == "--reload-config" {
                options.reload_config = true;
                continue;
            }

            bail!("unknown daemon argument: {arg}");
        }

        Ok(options)
    }

    pub fn parse_control_args(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self::default();
        let mut positional = Vec::new();

        for arg in args.into_iter().skip(1) {
            if arg == "--help" || arg == "-h" {
                options.help = true;
                continue;
            }

            if arg == "--version" {
                options.version = true;
                continue;
            }

            if let Some(path) = arg.strip_prefix("--config=") {
                options.config_path = Some(PathBuf::from(path));
                continue;
            }

            if arg.starts_with("--") {
                bail!("unknown veila option: {arg}");
            }

            positional.push(arg);
        }

        apply_control_positionals(&mut options, &positional)?;
        Ok(options)
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
        "reload" => {
            expect_no_extra_args(command, &positional[1..], || options.reload_config = true)
        }
        "stop" => expect_no_extra_args(command, &positional[1..], || options.stop = true),
        "theme" => apply_theme_positionals(options, &positional[1..]),
        _ => bail!("unknown veila command: {command}"),
    }
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

#[cfg(test)]
mod tests {
    use super::DaemonOptions;

    #[test]
    fn parses_config_argument() {
        let options = DaemonOptions::parse_args([
            "veilad".to_string(),
            "--config=/tmp/veila.toml".to_string(),
        ])
        .expect("arguments should parse");

        assert_eq!(
            options.config_path.as_deref(),
            Some(std::path::Path::new("/tmp/veila.toml"))
        );
    }

    #[test]
    fn parses_help_arguments() {
        let long = DaemonOptions::parse_args(["veilad".to_string(), "--help".to_string()])
            .expect("arguments should parse");
        let short = DaemonOptions::parse_args(["veilad".to_string(), "-h".to_string()])
            .expect("arguments should parse");

        assert!(long.help);
        assert!(short.help);
    }

    #[test]
    fn parses_session_id_argument() {
        let options =
            DaemonOptions::parse_args(["veilad".to_string(), "--session-id=c2".to_string()])
                .expect("arguments should parse");

        assert_eq!(options.session_id.as_deref(), Some("c2"));
    }

    #[test]
    fn parses_log_file_argument() {
        let options = DaemonOptions::parse_args([
            "veilad".to_string(),
            "--log-file=/tmp/veilad.log".to_string(),
        ])
        .expect("arguments should parse");

        assert_eq!(
            options.log_file_path.as_deref(),
            Some(std::path::Path::new("/tmp/veilad.log"))
        );
    }

    #[test]
    fn parses_lock_now_argument() {
        let options = DaemonOptions::parse_args(["veilad".to_string(), "--lock-now".to_string()])
            .expect("arguments should parse");

        assert!(options.lock_now);
    }

    #[test]
    fn parses_stop_argument() {
        let options = DaemonOptions::parse_args(["veilad".to_string(), "--stop".to_string()])
            .expect("arguments should parse");

        assert!(options.stop);
    }

    #[test]
    fn parses_list_themes_argument() {
        let options =
            DaemonOptions::parse_args(["veilad".to_string(), "--list-themes".to_string()])
                .expect("arguments should parse");

        assert!(options.list_themes);
    }

    #[test]
    fn parses_set_theme_argument() {
        let options =
            DaemonOptions::parse_args(["veilad".to_string(), "--set-theme=beach".to_string()])
                .expect("arguments should parse");

        assert_eq!(options.set_theme.as_deref(), Some("beach"));
    }

    #[test]
    fn parses_print_theme_argument() {
        let options =
            DaemonOptions::parse_args(["veilad".to_string(), "--print-theme=beach".to_string()])
                .expect("arguments should parse");

        assert_eq!(options.print_theme.as_deref(), Some("beach"));
    }

    #[test]
    fn parses_current_theme_argument() {
        let options =
            DaemonOptions::parse_args(["veilad".to_string(), "--current-theme".to_string()])
                .expect("arguments should parse");

        assert!(options.current_theme);
    }

    #[test]
    fn parses_unset_theme_argument() {
        let options =
            DaemonOptions::parse_args(["veilad".to_string(), "--unset-theme".to_string()])
                .expect("arguments should parse");

        assert!(options.unset_theme);
    }

    #[test]
    fn parses_status_argument() {
        let options = DaemonOptions::parse_args(["veilad".to_string(), "--status".to_string()])
            .expect("arguments should parse");

        assert!(options.status);
    }

    #[test]
    fn parses_reload_config_argument() {
        let options =
            DaemonOptions::parse_args(["veilad".to_string(), "--reload-config".to_string()])
                .expect("arguments should parse");

        assert!(options.reload_config);
    }

    #[test]
    fn parses_health_argument() {
        let options = DaemonOptions::parse_args(["veilad".to_string(), "--health".to_string()])
            .expect("arguments should parse");

        assert!(options.health);
    }

    #[test]
    fn parses_version_argument() {
        let options = DaemonOptions::parse_args(["veilad".to_string(), "--version".to_string()])
            .expect("arguments should parse");

        assert!(options.version);
    }

    #[test]
    fn parses_control_lock_command() {
        let options = DaemonOptions::parse_control_args(["veila".to_string(), "lock".to_string()])
            .expect("arguments should parse");

        assert!(options.lock_now);
    }

    #[test]
    fn parses_control_reload_command() {
        let options =
            DaemonOptions::parse_control_args(["veila".to_string(), "reload".to_string()])
                .expect("arguments should parse");

        assert!(options.reload_config);
    }

    #[test]
    fn parses_control_theme_set_command() {
        let options = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "theme".to_string(),
            "set".to_string(),
            "normandy".to_string(),
        ])
        .expect("arguments should parse");

        assert_eq!(options.set_theme.as_deref(), Some("normandy"));
    }

    #[test]
    fn parses_control_config_argument_after_command() {
        let options = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "theme".to_string(),
            "current".to_string(),
            "--config=/tmp/veila.toml".to_string(),
        ])
        .expect("arguments should parse");

        assert!(options.current_theme);
        assert_eq!(
            options.config_path.as_deref(),
            Some(std::path::Path::new("/tmp/veila.toml"))
        );
    }

    #[test]
    fn rejects_control_daemon_only_option() {
        let error = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "--log-file=/tmp/veilad.log".to_string(),
        ])
        .expect_err("daemon-only option should fail");

        assert!(error.to_string().contains("unknown veila option"));
    }
}
