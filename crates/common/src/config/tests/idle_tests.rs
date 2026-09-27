use super::AppConfig;

#[test]
fn idle_defaults_keep_idle_off_and_sleep_lock_on() {
    let config = AppConfig::from_toml_str("").expect("empty config should parse");

    assert!(!config.idle.enabled);
    assert_eq!(config.idle.lock_after_seconds, 300);
    assert!(config.idle.lock_before_sleep);
    assert_eq!(config.idle.effective_lock_after_seconds(), None);
}

#[test]
fn parses_idle_config() {
    let config = AppConfig::from_toml_str(
        r#"
            [idle]
            enabled = true
            lock_after_seconds = 600
            lock_before_sleep = false
        "#,
    )
    .expect("config should parse");

    assert_eq!(config.idle.effective_lock_after_seconds(), Some(600));
    assert!(!config.idle.lock_before_sleep);
    assert!(config.idle.lock_after_in_range());
}

#[test]
fn clamps_out_of_range_idle_timeouts() {
    let too_short = AppConfig::from_toml_str("[idle]\nenabled = true\nlock_after_seconds = 0\n")
        .expect("config should parse");
    let too_long =
        AppConfig::from_toml_str("[idle]\nenabled = true\nlock_after_seconds = 999999999\n")
            .expect("config should parse");

    assert_eq!(too_short.idle.effective_lock_after_seconds(), Some(5));
    assert!(!too_short.idle.lock_after_in_range());
    assert_eq!(too_long.idle.effective_lock_after_seconds(), Some(86_400));
    assert!(!too_long.idle.lock_after_in_range());
}
