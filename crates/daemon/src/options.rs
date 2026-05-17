use std::path::PathBuf;

use anyhow::{Result, bail};
use veila_common::ipc::LatencyReportMode;

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
    pub force_emergency_ui: bool,
    pub latency_report: LatencyReportMode,
    pub wait_ready: bool,
    pub stop: bool,
    pub list_themes: bool,
    pub status: bool,
    pub health: bool,
    pub doctor: bool,
    pub check_config: bool,
    pub init_config: bool,
    pub init_force: bool,
    pub init_theme: Option<String>,
    pub version: bool,
    pub reload_config: bool,
    pub background_prewarm_only: bool,
    pub idle: bool,
    pub idle_lock_after_seconds: Option<u64>,
    pub idle_lock_before_sleep: bool,
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

            if arg == "--force-emergency-ui" {
                options.force_emergency_ui = true;
                continue;
            }

            if let Some(mode) = parse_latency_report_arg(&arg)? {
                options.latency_report = mode;
                continue;
            }

            if arg == "--wait-ready" {
                options.wait_ready = true;
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

            if arg == "--doctor" {
                options.doctor = true;
                continue;
            }

            if arg == "--check-config" {
                options.check_config = true;
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

            if arg == "--background-prewarm-only" {
                options.background_prewarm_only = true;
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
        "theme" => apply_theme_positionals(options, &positional[1..]),
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

#[cfg(test)]
mod tests {
    use super::DaemonOptions;
    use veila_common::ipc::LatencyReportMode;

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
        let options = DaemonOptions::parse_args([
            "veilad".to_string(),
            "--lock-now".to_string(),
            "--force-emergency-ui".to_string(),
            "--latency-report".to_string(),
        ])
        .expect("arguments should parse");

        assert!(options.lock_now);
        assert!(options.force_emergency_ui);
        assert_eq!(options.latency_report, LatencyReportMode::Basic);
    }

    #[test]
    fn parses_verbose_latency_report_argument() {
        let options = DaemonOptions::parse_args([
            "veilad".to_string(),
            "--lock-now".to_string(),
            "--latency-report=verbose".to_string(),
        ])
        .expect("arguments should parse");

        assert_eq!(options.latency_report, LatencyReportMode::Verbose);
    }

    #[test]
    fn parses_wait_ready_argument() {
        let options = DaemonOptions::parse_args([
            "veilad".to_string(),
            "--lock-now".to_string(),
            "--wait-ready".to_string(),
        ])
        .expect("arguments should parse");

        assert!(options.lock_now);
        assert!(options.wait_ready);
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
    fn parses_doctor_argument() {
        let options = DaemonOptions::parse_args(["veilad".to_string(), "--doctor".to_string()])
            .expect("arguments should parse");

        assert!(options.doctor);
    }

    #[test]
    fn parses_check_config_argument() {
        let options =
            DaemonOptions::parse_args(["veilad".to_string(), "--check-config".to_string()])
                .expect("arguments should parse");

        assert!(options.check_config);
    }

    #[test]
    fn parses_version_argument() {
        let options = DaemonOptions::parse_args(["veilad".to_string(), "--version".to_string()])
            .expect("arguments should parse");

        assert!(options.version);
    }

    #[test]
    fn parses_background_prewarm_only_argument() {
        let options = DaemonOptions::parse_args([
            "veilad".to_string(),
            "--background-prewarm-only".to_string(),
        ])
        .expect("arguments should parse");

        assert!(options.background_prewarm_only);
    }

    #[test]
    fn parses_control_lock_command() {
        let options = DaemonOptions::parse_control_args(["veila".to_string(), "lock".to_string()])
            .expect("arguments should parse");

        assert!(options.lock_now);
    }

    #[test]
    fn parses_control_lock_command_with_wait_ready() {
        let options = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "--wait-ready".to_string(),
            "--force-emergency-ui".to_string(),
            "--latency-report".to_string(),
            "lock".to_string(),
        ])
        .expect("arguments should parse");

        assert!(options.lock_now);
        assert!(options.wait_ready);
        assert!(options.force_emergency_ui);
        assert_eq!(options.latency_report, LatencyReportMode::Basic);
    }

    #[test]
    fn parses_control_lock_command_with_verbose_latency_report() {
        let options = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "--wait-ready".to_string(),
            "--latency-report=verbose".to_string(),
            "lock".to_string(),
        ])
        .expect("arguments should parse");

        assert!(options.lock_now);
        assert!(options.wait_ready);
        assert_eq!(options.latency_report, LatencyReportMode::Verbose);
    }

    #[test]
    fn parses_control_reload_command() {
        let options =
            DaemonOptions::parse_control_args(["veila".to_string(), "reload".to_string()])
                .expect("arguments should parse");

        assert!(options.reload_config);
    }

    #[test]
    fn parses_control_idle_command() {
        let options = DaemonOptions::parse_control_args(["veila".to_string(), "idle".to_string()])
            .expect("arguments should parse");

        assert!(options.idle);
        assert_eq!(options.idle_lock_after_seconds, None);
    }

    #[test]
    fn parses_control_idle_command_with_lock_after_equals() {
        let options = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "idle".to_string(),
            "--lock-after=600".to_string(),
        ])
        .expect("arguments should parse");

        assert!(options.idle);
        assert_eq!(options.idle_lock_after_seconds, Some(600));
    }

    #[test]
    fn parses_control_idle_command_with_lock_after_space() {
        let options = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "idle".to_string(),
            "--lock-after".to_string(),
            "60".to_string(),
        ])
        .expect("arguments should parse");

        assert!(options.idle);
        assert_eq!(options.idle_lock_after_seconds, Some(60));
    }

    #[test]
    fn parses_control_idle_command_with_lock_before_sleep() {
        let options = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "idle".to_string(),
            "--lock-before-sleep".to_string(),
        ])
        .expect("arguments should parse");

        assert!(options.idle);
        assert!(options.idle_lock_before_sleep);
    }

    #[test]
    fn parses_control_idle_command_with_combined_options() {
        let options = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "idle".to_string(),
            "--lock-after=120".to_string(),
            "--lock-before-sleep".to_string(),
        ])
        .expect("arguments should parse");

        assert!(options.idle);
        assert_eq!(options.idle_lock_after_seconds, Some(120));
        assert!(options.idle_lock_before_sleep);
    }

    #[test]
    fn rejects_zero_idle_lock_after() {
        let error = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "idle".to_string(),
            "--lock-after=0".to_string(),
        ])
        .expect_err("zero timeout should fail");

        assert!(error.to_string().contains("at least 1 second"));
    }

    #[test]
    fn parses_control_doctor_command() {
        let options =
            DaemonOptions::parse_control_args(["veila".to_string(), "doctor".to_string()])
                .expect("arguments should parse");

        assert!(options.doctor);
    }

    #[test]
    fn parses_control_check_config_command() {
        let options =
            DaemonOptions::parse_control_args(["veila".to_string(), "check-config".to_string()])
                .expect("arguments should parse");

        assert!(options.check_config);
    }

    #[test]
    fn parses_control_init_command() {
        let options = DaemonOptions::parse_control_args(["veila".to_string(), "init".to_string()])
            .expect("arguments should parse");

        assert!(options.init_config);
        assert!(!options.init_force);
        assert_eq!(options.init_theme.as_deref(), None);
    }

    #[test]
    fn parses_control_init_command_with_force_and_theme_equals() {
        let options = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "init".to_string(),
            "--force".to_string(),
            "--theme=santorini".to_string(),
        ])
        .expect("arguments should parse");

        assert!(options.init_config);
        assert!(options.init_force);
        assert_eq!(options.init_theme.as_deref(), Some("santorini"));
    }

    #[test]
    fn parses_control_init_command_with_space_theme() {
        let options = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "init".to_string(),
            "--theme".to_string(),
            "window".to_string(),
        ])
        .expect("arguments should parse");

        assert!(options.init_config);
        assert_eq!(options.init_theme.as_deref(), Some("window"));
    }

    #[test]
    fn rejects_control_init_missing_theme_value() {
        let error = DaemonOptions::parse_control_args([
            "veila".to_string(),
            "init".to_string(),
            "--theme".to_string(),
        ])
        .expect_err("missing theme value should fail");

        assert!(error.to_string().contains("missing value for --theme"));
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
