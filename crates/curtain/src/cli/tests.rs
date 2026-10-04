use veila_common::{WeatherCondition, ipc::LatencyReportMode};

use crate::{CurtainOptions, PreviewClockTime};

#[test]
fn parses_notify_socket_argument() {
    let options = CurtainOptions::parse_args([
        "veila-curtain".to_string(),
        "--notify-socket=/tmp/veila.sock".to_string(),
        "--daemon-socket=/tmp/veila-auth.sock".to_string(),
        "--control-socket=/tmp/veila-control.sock".to_string(),
        "--config=/tmp/veila.toml".to_string(),
        "--force-emergency-ui".to_string(),
        "--latency-report".to_string(),
        "--preview-png=/tmp/veila-preview.png".to_string(),
        "--preview-size=1920x1080".to_string(),
        "--preview-artwork=/tmp/cover.png".to_string(),
        "--preview-title=After Dark".to_string(),
        "--preview-artist=Mr.Kitty".to_string(),
        "--preview-username=guest".to_string(),
        "--preview-hide-widgets".to_string(),
        "--preview-hide-weather".to_string(),
        "--preview-hide-battery".to_string(),
        "--preview-hide-now-playing".to_string(),
        "--preview-hide-keyboard-label".to_string(),
        "--preview-weather-location=Tokyo".to_string(),
        "--preview-weather-condition=rain".to_string(),
        "--preview-weather-temperature=7".to_string(),
        "--preview-battery-percent=84".to_string(),
        "--preview-battery-charging=true".to_string(),
        "--preview-time=21:54".to_string(),
    ])
    .expect("arguments should parse");

    assert_eq!(
        options.notify_socket.as_deref(),
        Some(std::path::Path::new("/tmp/veila.sock"))
    );
    assert_eq!(
        options.daemon_socket.as_deref(),
        Some(std::path::Path::new("/tmp/veila-auth.sock"))
    );
    assert_eq!(
        options.control_socket.as_deref(),
        Some(std::path::Path::new("/tmp/veila-control.sock"))
    );
    assert_eq!(
        options.config_path.as_deref(),
        Some(std::path::Path::new("/tmp/veila.toml"))
    );
    assert!(options.force_emergency_ui);
    assert_eq!(options.latency_report, LatencyReportMode::Basic);
    assert_eq!(
        options.preview_png.as_deref(),
        Some(std::path::Path::new("/tmp/veila-preview.png"))
    );
    assert_eq!(
        options.preview_size,
        Some(veila_renderer::FrameSize::new(1920, 1080))
    );
    assert_eq!(
        options.preview_artwork.as_deref(),
        Some(std::path::Path::new("/tmp/cover.png"))
    );
    assert_eq!(options.preview_title.as_deref(), Some("After Dark"));
    assert_eq!(options.preview_artist.as_deref(), Some("Mr.Kitty"));
    assert_eq!(options.preview_username.as_deref(), Some("guest"));
    assert!(options.preview_hide_widgets);
    assert!(options.preview_hide_weather);
    assert!(options.preview_hide_battery);
    assert!(options.preview_hide_now_playing);
    assert!(options.preview_hide_keyboard_label);
    assert_eq!(options.preview_weather_location.as_deref(), Some("Tokyo"));
    assert_eq!(
        options.preview_weather_condition,
        Some(WeatherCondition::Rain)
    );
    assert_eq!(options.preview_weather_temperature_celsius, Some(7));
    assert_eq!(options.preview_battery_percent, Some(84));
    assert_eq!(options.preview_battery_charging, Some(true));
    assert_eq!(
        options.preview_time,
        Some(PreviewClockTime {
            hour: 21,
            minute: 54
        })
    );
}

#[test]
fn parses_verbose_latency_report_argument() {
    let options = CurtainOptions::parse_args([
        "veila-curtain".to_string(),
        "--lock".to_string(),
        "--latency-report=verbose".to_string(),
    ])
    .expect("arguments should parse");

    assert_eq!(options.latency_report, LatencyReportMode::Verbose);
}

#[test]
fn parses_space_separated_preview_arguments() {
    let options = CurtainOptions::parse_args([
        "veila-curtain".to_string(),
        "--preview-png".to_string(),
        "/tmp/veila-preview.png".to_string(),
        "--preview-size".to_string(),
        "1920x1080".to_string(),
        "--preview-title".to_string(),
        "After Dark".to_string(),
        "--preview-artist".to_string(),
        "Mr.Kitty".to_string(),
        "--preview-username".to_string(),
        "guest".to_string(),
        "--preview-hide-widgets".to_string(),
        "--preview-hide-weather".to_string(),
        "--preview-hide-battery".to_string(),
        "--preview-hide-now-playing".to_string(),
        "--preview-hide-keyboard-label".to_string(),
        "--preview-weather-location".to_string(),
        "Tokyo".to_string(),
    ])
    .expect("arguments should parse");

    assert_eq!(
        options.preview_png.as_deref(),
        Some(std::path::Path::new("/tmp/veila-preview.png"))
    );
    assert_eq!(
        options.preview_size,
        Some(veila_renderer::FrameSize::new(1920, 1080))
    );
    assert_eq!(options.preview_title.as_deref(), Some("After Dark"));
    assert_eq!(options.preview_artist.as_deref(), Some("Mr.Kitty"));
    assert_eq!(options.preview_username.as_deref(), Some("guest"));
    assert!(options.preview_hide_widgets);
    assert!(options.preview_hide_weather);
    assert!(options.preview_hide_battery);
    assert!(options.preview_hide_now_playing);
    assert!(options.preview_hide_keyboard_label);
    assert_eq!(options.preview_weather_location.as_deref(), Some("Tokyo"));
}

#[test]
fn rejects_missing_space_separated_option_value() {
    let error =
        CurtainOptions::parse_args(["veila-curtain".to_string(), "--preview-png".to_string()])
            .expect_err("missing value should fail");

    assert!(error.to_string().contains("--preview-png requires a value"));
}

#[test]
fn parses_help_arguments() {
    let long = CurtainOptions::parse_args(["veila-curtain".to_string(), "--help".to_string()])
        .expect("arguments should parse");
    let short = CurtainOptions::parse_args(["veila-curtain".to_string(), "-h".to_string()])
        .expect("arguments should parse");

    assert!(long.help);
    assert!(short.help);
}

#[test]
fn parses_direct_lock_argument() {
    let options = CurtainOptions::parse_args([
        "veila-curtain".to_string(),
        "--lock".to_string(),
        "--force-emergency-ui".to_string(),
    ])
    .expect("arguments should parse");

    assert!(options.lock);
    assert!(options.force_emergency_ui);
}
