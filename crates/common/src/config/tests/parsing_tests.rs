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
fn parses_lock_file_logging_settings() {
    let config = AppConfig::from_toml_str(
        r#"
            [lock]
            log_to_file = true
            log_file_path = "~/.cache/veila/debug.log"
        "#,
    )
    .expect("config should parse");

    assert!(config.lock.log_to_file);
    assert_eq!(
        config.lock.log_file_path,
        std::path::PathBuf::from("~/.cache/veila/debug.log")
    );
}

#[test]
fn parses_now_playing_player_filters() {
    let config = AppConfig::from_toml_str(
        r#"
            [now_playing]
            include_players = ["Spotify", "mpv"]
            exclude_players = ["Firefox", "Chromium"]
        "#,
    )
    .expect("config should parse");

    assert_eq!(
        config.now_playing.include_players,
        vec![String::from("Spotify"), String::from("mpv")]
    );
    assert_eq!(
        config.now_playing.exclude_players,
        vec![String::from("Firefox"), String::from("Chromium")]
    );
}

#[test]
fn partial_input_table_keeps_default_translucent_background_and_disabled_border() {
    let config = AppConfig::from_toml_str(
        r##"
            [visuals.input]
            background_color = "#FFFFFF"
        "##,
    )
    .expect("config should parse");

    assert_eq!(
        config.visuals.input_background_color(),
        RgbColor::rgb(255, 255, 255)
    );
    assert_eq!(config.visuals.input_background_opacity(), Some(5));
    assert_eq!(
        config.visuals.input_border_color(),
        RgbColor::rgb(255, 255, 255)
    );
    assert_eq!(config.visuals.input_border_opacity(), Some(0));
    assert_eq!(config.visuals.input_border_width(), Some(0));
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
fn parses_per_output_background_overrides_with_default_fallback() {
    let config = AppConfig::from_toml_str(
        r#"
            [background]
            mode = "file"
            path = "/tmp/default.jpg"

            [[background.outputs]]
            name = "DP-1"
            path = "/tmp/left.jpg"

            [[background.outputs]]
            name = "HDMI-A-1"
            path = "/tmp/right.jpg"
        "#,
    )
    .expect("config should parse");

    assert_eq!(config.background.outputs.len(), 2);
    assert_eq!(
        config
            .background
            .resolved_path_for_output(Some("DP-1"))
            .as_deref(),
        Some(std::path::Path::new("/tmp/left.jpg"))
    );
    assert_eq!(
        config
            .background
            .resolved_path_for_output(Some("HDMI-A-1"))
            .as_deref(),
        Some(std::path::Path::new("/tmp/right.jpg"))
    );
    assert_eq!(
        config
            .background
            .resolved_path_for_output(Some("eDP-1"))
            .as_deref(),
        Some(std::path::Path::new("/tmp/default.jpg"))
    );
    assert_eq!(
        config.background.resolved_path_for_output(None).as_deref(),
        Some(std::path::Path::new("/tmp/default.jpg"))
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
