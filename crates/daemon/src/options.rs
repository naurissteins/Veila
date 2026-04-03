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
}
