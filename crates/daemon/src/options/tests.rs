use super::{ControlCommand, ControlOptions, LockOptions, LogTarget};
use veila_common::ipc::LatencyReportMode;

#[test]
fn parses_control_version_argument() {
    let long = ControlOptions::parse_args(["veila".to_string(), "--version".to_string()])
        .expect("arguments should parse");
    let short = ControlOptions::parse_args(["veila".to_string(), "-v".to_string()])
        .expect("arguments should parse");

    assert_eq!(long.command, Some(ControlCommand::Version));
    assert_eq!(short.command, Some(ControlCommand::Version));
}

#[test]
fn parses_control_lock_command() {
    let options = ControlOptions::parse_args(["veila".to_string(), "lock".to_string()])
        .expect("arguments should parse");

    assert_eq!(
        options.command,
        Some(ControlCommand::Lock(LockOptions::default()))
    );
}

#[test]
fn parses_control_lock_command_with_wait_ready() {
    let options = ControlOptions::parse_args([
        "veila".to_string(),
        "--wait-ready".to_string(),
        "--force-emergency-ui".to_string(),
        "--latency-report".to_string(),
        "lock".to_string(),
    ])
    .expect("arguments should parse");

    assert_eq!(
        options.command,
        Some(ControlCommand::Lock(LockOptions {
            wait_ready: true,
            force_emergency_ui: true,
            latency_report: LatencyReportMode::Basic
        }))
    );
}

#[test]
fn parses_control_lock_command_with_verbose_latency_report() {
    let options = ControlOptions::parse_args([
        "veila".to_string(),
        "--wait-ready".to_string(),
        "--latency-report=verbose".to_string(),
        "lock".to_string(),
    ])
    .expect("arguments should parse");

    assert_eq!(
        options.command,
        Some(ControlCommand::Lock(LockOptions {
            wait_ready: true,
            force_emergency_ui: false,
            latency_report: LatencyReportMode::Verbose
        }))
    );
}

#[test]
fn parses_control_reload_command() {
    let options = ControlOptions::parse_args(["veila".to_string(), "reload".to_string()])
        .expect("arguments should parse");

    assert_eq!(options.command, Some(ControlCommand::Reload));
}

#[test]
fn parses_control_logs_command_defaults() {
    let options = ControlOptions::parse_args(["veila".to_string(), "logs".to_string()])
        .expect("arguments should parse");

    let Some(ControlCommand::Logs(options)) = options.command else {
        panic!("expected logs command");
    };
    assert!(!options.file);
    assert_eq!(options.target, LogTarget::LockService);
    assert!(!options.follow);
    assert_eq!(options.since.as_deref(), None);
    assert_eq!(options.lines, None);
}

#[test]
fn parses_control_logs_command_with_options() {
    let options = ControlOptions::parse_args([
        "veila".to_string(),
        "logs".to_string(),
        "--follow".to_string(),
        "--since=10m".to_string(),
        "--lines".to_string(),
        "25".to_string(),
        "--curtain".to_string(),
    ])
    .expect("arguments should parse");

    let Some(ControlCommand::Logs(options)) = options.command else {
        panic!("expected logs command");
    };
    assert!(options.follow);
    assert_eq!(options.since.as_deref(), Some("10m"));
    assert_eq!(options.lines, Some(25));
    assert_eq!(options.target, LogTarget::Curtain);
}

#[test]
fn parses_control_file_command() {
    let options = ControlOptions::parse_args([
        "veila".to_string(),
        "logs".to_string(),
        "--file".to_string(),
        "--follow".to_string(),
        "--lines=100".to_string(),
    ])
    .expect("arguments should parse");

    let Some(ControlCommand::Logs(options)) = options.command else {
        panic!("expected logs command");
    };
    assert!(options.file);
    assert!(options.follow);
    assert_eq!(options.lines, Some(100));
}

#[test]
fn rejects_multiple_targets() {
    let error = ControlOptions::parse_args([
        "veila".to_string(),
        "logs".to_string(),
        "--daemon".to_string(),
        "--curtain".to_string(),
    ])
    .expect_err("multiple target filters should fail");

    assert!(error.to_string().contains("only one logs target"));
}

#[test]
fn parses_control_doctor_command() {
    let options = ControlOptions::parse_args(["veila".to_string(), "doctor".to_string()])
        .expect("arguments should parse");

    assert_eq!(options.command, Some(ControlCommand::Doctor));
}

#[test]
fn parses_control_check_config_command() {
    let options = ControlOptions::parse_args(["veila".to_string(), "check-config".to_string()])
        .expect("arguments should parse");

    assert_eq!(options.command, Some(ControlCommand::CheckConfig));
}

#[test]
fn parses_control_init_command() {
    let options = ControlOptions::parse_args(["veila".to_string(), "init".to_string()])
        .expect("arguments should parse");

    assert_eq!(
        options.command,
        Some(ControlCommand::Init {
            theme: None,
            force: false
        })
    );
}

#[test]
fn parses_control_init_command_with_force_and_theme_equals() {
    let options = ControlOptions::parse_args([
        "veila".to_string(),
        "init".to_string(),
        "--force".to_string(),
        "--theme=santorini".to_string(),
    ])
    .expect("arguments should parse");

    assert_eq!(
        options.command,
        Some(ControlCommand::Init {
            theme: Some("santorini".into()),
            force: true
        })
    );
}

#[test]
fn parses_control_init_command_with_space_theme() {
    let options = ControlOptions::parse_args([
        "veila".to_string(),
        "init".to_string(),
        "--theme".to_string(),
        "window".to_string(),
    ])
    .expect("arguments should parse");

    assert_eq!(
        options.command,
        Some(ControlCommand::Init {
            theme: Some("window".into()),
            force: false
        })
    );
}

#[test]
fn rejects_control_init_missing_theme_value() {
    let error = ControlOptions::parse_args([
        "veila".to_string(),
        "init".to_string(),
        "--theme".to_string(),
    ])
    .expect_err("missing theme value should fail");

    assert!(error.to_string().contains("missing value for --theme"));
}

#[test]
fn parses_control_theme_set_command() {
    let options = ControlOptions::parse_args([
        "veila".to_string(),
        "theme".to_string(),
        "set".to_string(),
        "normandy".to_string(),
    ])
    .expect("arguments should parse");

    assert_eq!(
        options.command,
        Some(ControlCommand::ThemeSet("normandy".into()))
    );
}

#[test]
fn parses_control_config_argument_after_command() {
    let options = ControlOptions::parse_args([
        "veila".to_string(),
        "theme".to_string(),
        "current".to_string(),
        "--config=/tmp/veila.toml".to_string(),
    ])
    .expect("arguments should parse");

    assert_eq!(options.command, Some(ControlCommand::ThemeCurrent));
    assert_eq!(
        options.config_path.as_deref(),
        Some(std::path::Path::new("/tmp/veila.toml"))
    );
}

#[test]
fn rejects_control_daemon_only_option() {
    let error =
        ControlOptions::parse_args(["veila".to_string(), "--log-file=/tmp/veila.log".to_string()])
            .expect_err("daemon-only option should fail");

    assert!(error.to_string().contains("unknown veila option"));
}

#[test]
fn removed_idle_command_points_to_config() {
    let error = ControlOptions::parse_args(["veila".to_string(), "idle".to_string()])
        .expect_err("idle command was removed");

    assert!(error.to_string().contains("[idle] section of config.toml"));
}
