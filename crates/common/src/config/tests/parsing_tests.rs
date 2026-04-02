use super::*;

#[test]
fn parses_widget_enable_flags() {
    let config = AppConfig::from_toml_str(
        r#"
            [visuals.avatar]
            enabled = false

            [visuals.username]
            enabled = false

            [visuals.clock]
            enabled = false

            [visuals.date]
            enabled = false

            [visuals.placeholder]
            enabled = false

            [visuals.status]
            enabled = false

            [visuals.eye]
            enabled = false

            [visuals.caps_lock]
            enabled = false

            [visuals.keyboard]
            enabled = false

            [visuals.battery]
            enabled = false

            [visuals.weather]
            enabled = false

            [visuals.now_playing]
            enabled = false
        "#,
    )
    .expect("config should parse");

    assert!(!config.visuals.avatar_enabled());
    assert!(!config.visuals.username_enabled());
    assert!(!config.visuals.clock_enabled());
    assert!(!config.visuals.date_enabled());
    assert!(!config.visuals.placeholder_enabled());
    assert!(!config.visuals.status_enabled());
    assert!(!config.visuals.eye_enabled());
    assert!(!config.visuals.caps_lock_enabled());
    assert!(!config.visuals.keyboard_enabled());
    assert!(!config.visuals.battery_enabled());
    assert!(!config.visuals.weather_enabled());
    assert!(!config.visuals.now_playing_enabled());
}

#[test]
fn parses_full_layer_width_keyword() {
    let config = AppConfig::from_toml_str(
        r#"
            [visuals.layer]
            enabled = true
            width = "full"
        "#,
    )
    .expect("config should parse");

    assert!(config.visuals.layer_enabled());
    assert!(config.visuals.layer_full_width());
    assert_eq!(config.visuals.layer_width(), None);
    assert_eq!(
        config.visuals.layer.as_ref().and_then(|layer| layer.width),
        Some(LayerWidth::Keyword(LayerWidthKeyword::Full))
    );
}

#[test]
fn parses_full_layer_height_keyword() {
    let config = AppConfig::from_toml_str(
        r#"
            [visuals.layer]
            enabled = true
            height = "full"
        "#,
    )
    .expect("config should parse");

    assert!(config.visuals.layer_enabled());
    assert!(config.visuals.layer_full_height());
    assert_eq!(config.visuals.layer_height(), None);
    assert_eq!(
        config.visuals.layer.as_ref().and_then(|layer| layer.height),
        Some(LayerHeight::Keyword(LayerHeightKeyword::Full))
    );
}

#[test]
fn parses_layer_vertical_alignment() {
    let config = AppConfig::from_toml_str(
        r#"
            [visuals.layer]
            enabled = true
            vertical_alignment = "bottom"
            offset_y = 18
        "#,
    )
    .expect("config should parse");

    assert_eq!(
        config.visuals.layer_vertical_alignment(),
        LayerVerticalAlignment::Bottom
    );
    assert_eq!(config.visuals.layer_offset_y(), Some(18));
}

#[test]
fn parses_lock_auto_reload_config_flag() {
    let config = AppConfig::from_toml_str(
        r#"
            [lock]
            auto_reload_config = true
            auto_reload_debounce_ms = 180
        "#,
    )
    .expect("config should parse");

    assert!(config.lock.auto_reload_config);
    assert_eq!(config.lock.auto_reload_debounce_ms, 180);
}

#[test]
fn infers_file_mode_from_legacy_background_path() {
    let config = AppConfig::from_toml_str(
        r#"
            [background]
            path = "/tmp/wallpaper.jpg"
        "#,
    )
    .expect("config should parse");

    assert_eq!(config.background.effective_mode(), BackgroundMode::File);
    assert_eq!(
        config.background.resolved_path().as_deref(),
        Some(std::path::Path::new("/tmp/wallpaper.jpg"))
    );
}

#[test]
fn solid_mode_disables_background_image_resolution() {
    let config = AppConfig::from_toml_str(
        r#"
            [background]
            mode = "solid"
            path = "/tmp/wallpaper.jpg"
        "#,
    )
    .expect("config should parse");

    assert_eq!(config.background.effective_mode(), BackgroundMode::Solid);
    assert!(config.background.resolved_path().is_none());
}
