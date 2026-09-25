use super::{DaemonOptions, LogTarget};
use veila_common::ipc::LatencyReportMode;

#[test]
fn parses_control_version_argument() {
    let long = DaemonOptions::parse_control_args(["veila".to_string(), "--version".to_string()])
        .expect("arguments should parse");
    let short = DaemonOptions::parse_control_args(["veila".to_string(), "-v".to_string()])
        .expect("arguments should parse");

    assert!(long.version);
    assert!(short.version);
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
    let options = DaemonOptions::parse_control_args(["veila".to_string(), "reload".to_string()])
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
fn parses_control_logs_command_defaults() {
    let options = DaemonOptions::parse_control_args(["veila".to_string(), "logs".to_string()])
        .expect("arguments should parse");

    assert!(options.logs);
    assert!(!options.logs_file);
    assert_eq!(options.logs_target, LogTarget::LockService);
    assert!(!options.logs_follow);
    assert_eq!(options.logs_since.as_deref(), None);
    assert_eq!(options.logs_lines, None);
}

#[test]
fn parses_control_logs_command_with_options() {
    let options = DaemonOptions::parse_control_args([
        "veila".to_string(),
        "logs".to_string(),
        "--follow".to_string(),
        "--since=10m".to_string(),
        "--lines".to_string(),
        "25".to_string(),
        "--curtain".to_string(),
    ])
    .expect("arguments should parse");

    assert!(options.logs);
    assert!(options.logs_follow);
    assert_eq!(options.logs_since.as_deref(), Some("10m"));
    assert_eq!(options.logs_lines, Some(25));
    assert_eq!(options.logs_target, LogTarget::Curtain);
}

#[test]
fn parses_control_logs_file_command() {
    let options = DaemonOptions::parse_control_args([
        "veila".to_string(),
        "logs".to_string(),
        "--file".to_string(),
        "--follow".to_string(),
        "--lines=100".to_string(),
    ])
    .expect("arguments should parse");

    assert!(options.logs);
    assert!(options.logs_file);
    assert!(options.logs_follow);
    assert_eq!(options.logs_lines, Some(100));
}

#[test]
fn rejects_multiple_logs_targets() {
    let error = DaemonOptions::parse_control_args([
        "veila".to_string(),
        "logs".to_string(),
        "--daemon".to_string(),
        "--idle".to_string(),
    ])
    .expect_err("multiple target filters should fail");

    assert!(error.to_string().contains("only one logs target"));
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
    let options = DaemonOptions::parse_control_args(["veila".to_string(), "doctor".to_string()])
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
