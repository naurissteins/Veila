use std::fs;

use super::{AppConfig, BackgroundMode, ClockFormat, InputVisualEntry, RgbColor, WeatherUnit};

#[test]
fn parses_partial_config_with_defaults() {
    let config = AppConfig::from_toml_str(
        r#"
            [background]
            color = [12, 16, 24]
        "#,
    )
    .expect("config should parse");

    assert_eq!(config.lock.acquire_timeout_seconds, 5);
    assert!(config.lock.show_username);
    assert!(config.lock.username.is_none());
    assert!(config.lock.user_hint.is_none());
    assert!(config.lock.avatar_path.is_none());
    assert_eq!(config.background.effective_mode(), BackgroundMode::Bundled);
    assert_eq!(config.background.color, RgbColor::rgb(12, 16, 24));
    assert!(config.background.path.is_none());
    assert!(
        config
            .background
            .resolved_path()
            .is_some_and(|path| path.ends_with("assets/bg/abstract-blur-blue.jpg"))
    );
    assert_eq!(config.background.blur_radius, 0);
    assert_eq!(config.background.dim_strength, 34);
    assert!(config.background.tint.is_none());
    assert_eq!(config.background.tint_opacity, 0);
    assert!(!config.weather.enabled);
    assert!(config.weather.location.is_none());
    assert!(config.weather.clone().coordinates().is_none());
    assert_eq!(config.weather.refresh_minutes, 15);
    assert_eq!(config.weather.unit, WeatherUnit::Celsius);
    assert!(matches!(config.visuals.input, InputVisualEntry::Color(_)));
    assert!(config.visuals.input_background_opacity().is_none());
    assert!(config.visuals.input_border_opacity().is_none());
    assert!(config.visuals.input_width().is_none());
    assert!(config.visuals.input_height().is_none());
    assert_eq!(config.visuals.input_radius(), 32);
    assert!(config.visuals.input_border_width().is_none());
    assert!(config.visuals.avatar_background_color().is_none());
    assert!(config.visuals.avatar_size().is_none());
    assert!(config.visuals.avatar_placeholder_padding().is_none());
    assert!(config.visuals.avatar_icon_color().is_none());
    assert!(config.visuals.avatar_ring_color().is_none());
    assert!(config.visuals.avatar_ring_width().is_none());
    assert!(config.visuals.avatar_background_opacity().is_none());
    assert!(config.visuals.username_color().is_none());
    assert!(config.visuals.username_opacity().is_none());
    assert!(config.visuals.username_size().is_none());
    assert!(config.visuals.avatar_gap().is_none());
    assert!(config.visuals.username_gap().is_none());
    assert!(config.visuals.status_gap().is_none());
    assert!(config.visuals.clock_gap().is_none());
    assert!(config.visuals.auth_stack_offset().is_none());
    assert!(config.visuals.header_top_offset().is_none());
    assert!(config.visuals.clock_font_family().is_none());
    assert!(config.visuals.clock_font_weight().is_none());
    assert_eq!(config.visuals.clock_format(), ClockFormat::TwentyFourHour);
    assert!(config.visuals.clock_meridiem_size().is_none());
    assert!(config.visuals.clock_meridiem_offset_x().is_none());
    assert!(config.visuals.clock_meridiem_offset_y().is_none());
    assert!(config.visuals.clock_color().is_none());
    assert!(config.visuals.clock_opacity().is_none());
    assert!(config.visuals.date_color().is_none());
    assert!(config.visuals.date_font_family().is_none());
    assert!(config.visuals.date_font_weight().is_none());
    assert!(config.visuals.date_opacity().is_none());
    assert!(config.visuals.clock_size().is_none());
    assert!(config.visuals.date_size().is_none());
    assert!(config.visuals.placeholder_color().is_none());
    assert!(config.visuals.placeholder_opacity().is_none());
    assert!(config.visuals.eye_icon_color().is_none());
    assert!(config.visuals.eye_icon_opacity().is_none());
    assert!(config.visuals.keyboard_color().is_none());
    assert!(config.visuals.keyboard_background_color().is_none());
    assert!(config.visuals.keyboard_background_size().is_none());
    assert!(config.visuals.keyboard_opacity().is_none());
    assert!(config.visuals.keyboard_size().is_none());
    assert!(config.visuals.keyboard_top_offset().is_none());
    assert!(config.visuals.keyboard_right_offset().is_none());
    assert!(config.visuals.weather_size().is_none());
    assert!(config.visuals.weather_temperature_color().is_none());
    assert!(config.visuals.weather_location_color().is_none());
    assert!(config.visuals.weather_temperature_font_family().is_none());
    assert!(config.visuals.weather_temperature_font_weight().is_none());
    assert!(
        config
            .visuals
            .weather_temperature_letter_spacing()
            .is_none()
    );
    assert!(config.visuals.weather_temperature_size().is_none());
    assert!(config.visuals.weather_location_size().is_none());
    assert!(config.visuals.weather_icon_size().is_none());
    assert!(config.visuals.weather_icon_gap().is_none());
    assert!(config.visuals.weather_location_gap().is_none());
    assert_eq!(
        config.visuals.weather_alignment(),
        super::WeatherAlignment::Left
    );
    assert!(config.visuals.weather_horizontal_padding().is_none());
    assert!(config.visuals.weather_left_padding().is_none());
    assert!(config.visuals.weather_bottom_padding().is_none());
    assert!(config.visuals.now_playing_title_color().is_none());
    assert!(config.visuals.now_playing_artist_color().is_none());
    assert!(config.visuals.now_playing_fade_duration_ms().is_none());
    assert!(config.visuals.now_playing_title_font_family().is_none());
    assert!(config.visuals.now_playing_artist_font_family().is_none());
    assert!(config.visuals.now_playing_title_font_weight().is_none());
    assert!(config.visuals.now_playing_artist_font_weight().is_none());
    assert!(config.visuals.now_playing_opacity().is_none());
    assert!(config.visuals.now_playing_title_opacity().is_none());
    assert!(config.visuals.now_playing_artist_opacity().is_none());
    assert!(config.visuals.now_playing_artwork_opacity().is_none());
    assert!(config.visuals.now_playing_title_size().is_none());
    assert!(config.visuals.now_playing_artist_size().is_none());
    assert!(config.visuals.now_playing_width().is_none());
    assert!(config.visuals.now_playing_text_gap().is_none());
    assert!(config.visuals.now_playing_artwork_size().is_none());
    assert!(config.visuals.now_playing_artwork_radius().is_none());
    assert!(config.visuals.now_playing_right_padding().is_none());
    assert!(config.visuals.now_playing_bottom_padding().is_none());
    assert!(config.visuals.now_playing_right_offset().is_none());
    assert!(config.visuals.now_playing_bottom_offset().is_none());
    assert!(config.visuals.status_color().is_none());
    assert!(config.visuals.status_opacity().is_none());
    assert!(config.visuals.input_mask_color().is_none());
}

#[test]
fn loads_config_from_file() {
    let dir = std::env::temp_dir().join(format!("veila-config-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        r##"
            [background]
            mode = "file"
            path = "/tmp/wallpaper.jpg"
            blur_radius = 6
            dim_strength = 40
            tint = "#080A0E99"
            tint_opacity = 12

            [lock]
            acquire_timeout_seconds = 9
            auth_backoff_base_ms = 250
            show_username = false
            username = "anonymous"
            user_hint = "Type your password"
            avatar_path = "/tmp/avatar.png"

            [weather]
            enabled = true
            location = "Riga"
            latitude = 56.9496
            longitude = 24.1052
            refresh_minutes = 20
            unit = "fahrenheit"

            [visuals]
            avatar_background_color = "rgba(24, 30, 42, 0.82)"
            input = "#FFFFFF"
            input_opacity = 10
            input_border = "#FFFFFF"
            input_border_opacity = 12
            input_width = 280
            input_height = 54
            input_radius = 20
            input_border_width = 3
            avatar_size = 92
            avatar_placeholder_padding = 12
            avatar_icon_color = "#E8EEF9"
            avatar_ring_color = "#94B2FF"
            avatar_ring_width = 3
            avatar_background_opacity = 36
            username_color = "#D7E3FF"
            username_opacity = 72
            username_size = 3
            avatar_gap = 14
            username_gap = 28
            status_gap = 18
            clock_gap = 10
            auth_stack_offset = 16
            header_top_offset = -12
            clock_font_family = "Bebas Neue"
            clock_font_weight = 700
            clock_format = "12h"
            clock_meridiem_size = 3
            clock_meridiem_offset_x = 6
            clock_meridiem_offset_y = -2
            clock_color = "#F8FBFF"
            clock_opacity = 96
            date_color = "#C8D4EC"
            date_opacity = 74
            clock_size = 4
            date_size = 3
            placeholder_color = "#8694B4"
            placeholder_opacity = 60
            eye_icon_color = "#F4F8FF"
            eye_icon_opacity = 72
            status_color = "#FFE0A0"
            status_opacity = 88
            input_mask_color = "#A9C4FF"
        "##,
    )
    .expect("config file");

    let loaded = AppConfig::load(Some(&path)).expect("config should load");

    assert_eq!(loaded.path.as_deref(), Some(path.as_path()));
    assert_eq!(loaded.config.lock.acquire_timeout_seconds, 9);
    assert_eq!(loaded.config.lock.auth_backoff_base_ms, 250);
    assert!(!loaded.config.lock.show_username);
    assert_eq!(loaded.config.lock.username.as_deref(), Some("anonymous"));
    assert_eq!(
        loaded.config.lock.avatar_path.as_deref(),
        Some(std::path::Path::new("/tmp/avatar.png"))
    );
    assert_eq!(
        loaded.config.lock.user_hint.as_deref(),
        Some("Type your password")
    );
    assert!(loaded.config.weather.enabled);
    assert_eq!(loaded.config.weather.location.as_deref(), Some("Riga"));
    assert_eq!(
        loaded.config.weather.clone().coordinates(),
        Some((56.9496, 24.1052))
    );
    assert_eq!(loaded.config.weather.refresh_minutes, 20);
    assert_eq!(loaded.config.weather.unit, WeatherUnit::Fahrenheit);
    assert_eq!(
        loaded.config.background.effective_mode(),
        BackgroundMode::File
    );
    assert_eq!(
        loaded.config.background.resolved_path().as_deref(),
        Some(std::path::Path::new("/tmp/wallpaper.jpg"))
    );
    assert_eq!(loaded.config.background.blur_radius, 6);
    assert_eq!(loaded.config.background.dim_strength, 40);
    assert_eq!(
        loaded.config.background.tint,
        Some(RgbColor::rgba(8, 10, 14, 153))
    );
    assert_eq!(loaded.config.background.tint_opacity, 12);
    assert_eq!(
        loaded.config.visuals.avatar_background_color(),
        Some(RgbColor::rgba(24, 30, 42, 209))
    );
    assert_eq!(
        loaded.config.visuals.input_background_color(),
        RgbColor::rgb(255, 255, 255)
    );
    assert_eq!(loaded.config.visuals.input_background_opacity(), Some(10));
    assert_eq!(
        loaded.config.visuals.input_border_color(),
        RgbColor::rgb(255, 255, 255)
    );
    assert_eq!(loaded.config.visuals.input_border_opacity(), Some(12));
    assert_eq!(loaded.config.visuals.input_width(), Some(280));
    assert_eq!(loaded.config.visuals.input_height(), Some(54));
    assert_eq!(loaded.config.visuals.input_radius(), 20);
    assert_eq!(loaded.config.visuals.input_border_width(), Some(3));
    assert_eq!(loaded.config.visuals.avatar_size(), Some(92));
    assert_eq!(loaded.config.visuals.avatar_placeholder_padding(), Some(12));
    assert_eq!(
        loaded.config.visuals.avatar_icon_color(),
        Some(RgbColor::rgb(232, 238, 249))
    );
    assert_eq!(
        loaded.config.visuals.avatar_ring_color(),
        Some(RgbColor::rgb(148, 178, 255))
    );
    assert_eq!(loaded.config.visuals.avatar_ring_width(), Some(3));
    assert_eq!(loaded.config.visuals.avatar_background_opacity(), Some(36));
    assert_eq!(
        loaded.config.visuals.username_color(),
        Some(RgbColor::rgb(215, 227, 255))
    );
    assert_eq!(loaded.config.visuals.username_opacity(), Some(72));
    assert_eq!(loaded.config.visuals.username_size(), Some(3));
    assert_eq!(loaded.config.visuals.avatar_gap(), Some(14));
    assert_eq!(loaded.config.visuals.username_gap(), Some(28));
    assert_eq!(loaded.config.visuals.status_gap(), Some(18));
    assert_eq!(loaded.config.visuals.clock_gap(), Some(10));
    assert_eq!(loaded.config.visuals.auth_stack_offset(), Some(16));
    assert_eq!(loaded.config.visuals.header_top_offset(), Some(-12));
    assert_eq!(
        loaded.config.visuals.clock_font_family(),
        Some("Bebas Neue")
    );
    assert_eq!(loaded.config.visuals.clock_font_weight(), Some(700));
    assert_eq!(
        loaded.config.visuals.clock_format(),
        ClockFormat::TwelveHour
    );
    assert_eq!(loaded.config.visuals.clock_meridiem_size(), Some(3));
    assert_eq!(loaded.config.visuals.clock_meridiem_offset_x(), Some(6));
    assert_eq!(loaded.config.visuals.clock_meridiem_offset_y(), Some(-2));
    assert_eq!(
        loaded.config.visuals.clock_color(),
        Some(RgbColor::rgb(248, 251, 255))
    );
    assert_eq!(loaded.config.visuals.clock_opacity(), Some(96));
    assert_eq!(
        loaded.config.visuals.date_color(),
        Some(RgbColor::rgb(200, 212, 236))
    );
    assert_eq!(loaded.config.visuals.date_opacity(), Some(74));
    assert_eq!(loaded.config.visuals.clock_size(), Some(4));
    assert_eq!(loaded.config.visuals.date_size(), Some(3));
    assert_eq!(
        loaded.config.visuals.placeholder_color(),
        Some(RgbColor::rgb(134, 148, 180))
    );
    assert_eq!(loaded.config.visuals.placeholder_opacity(), Some(60));
    assert_eq!(
        loaded.config.visuals.eye_icon_color(),
        Some(RgbColor::rgb(244, 248, 255))
    );
    assert_eq!(loaded.config.visuals.eye_icon_opacity(), Some(72));
    assert_eq!(
        loaded.config.visuals.status_color(),
        Some(RgbColor::rgb(255, 224, 160))
    );
    assert_eq!(loaded.config.visuals.status_opacity(), Some(88));
    assert_eq!(
        loaded.config.visuals.input_mask_color(),
        Some(RgbColor::rgb(169, 196, 255))
    );

    fs::remove_file(path).ok();
    fs::remove_dir(dir).ok();
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

#[test]
fn loads_nested_visual_tables_with_precedence_over_flat_keys() {
    let config = AppConfig::from_toml_str(
        r##"
            [visuals]
            input_border = "#111111"
            username_color = "#111111"
            clock_gap = 6
            foreground = "#111111"

            [visuals.input]
            background_color = "#FFFFFF"
            background_opacity = 5
            border_color = "#DDDDDD"
            border_opacity = 12
            width = 310
            height = 54
            radius = 10
            border_width = 0
            mask_color = "#A9C4FF"

            [visuals.avatar]
            size = 192
            gap = 14
            background_color = "#ffffff"
            background_opacity = 6
            placeholder_padding = 28
            ring_color = "#94B2FF"
            ring_width = 0
            icon_color = "#ffffff"

            [visuals.username]
            color = "#ffffff"
            opacity = 84
            size = 4
            gap = 28

            [visuals.clock]
            font_family = "Prototype"
            font_weight = 700
            format = "12h"
            meridiem_size = 3
            meridiem_offset_x = 6
            meridiem_offset_y = -2
            color = "#ffffff"
            opacity = 40
            size = 14
            gap = 20

            [visuals.date]
            font_family = "Geom"
            font_weight = 600
            color = "#ffffff"
            opacity = 40
            size = 2

            [visuals.placeholder]
            color = "#ffffff"
            opacity = 60

            [visuals.status]
            color = "#FFE0A0"
            opacity = 88
            gap = 18

            [visuals.eye]
            color = "#ffffff"
            opacity = 72

            [visuals.keyboard]
            background_color = "rgba(18, 22, 30, 0.32)"
            background_size = 42
            color = "#E8EEF9"
            opacity = 68
            size = 3
            top_offset = -12
            right_offset = 8

            [visuals.weather]
            size = 3
            opacity = 62
            icon_opacity = 41
            temperature_opacity = 77
            location_opacity = 53
            temperature_color = "#FFFFFF"
            location_color = "#D6E3FF"
            temperature_font_family = "Prototype"
            temperature_font_weight = 600
            temperature_letter_spacing = 2
            temperature_size = 4
            location_size = 2
            icon_size = 36
            icon_gap = 10
            location_gap = 3
            alignment = "right"
            left_offset = 12
            bottom_offset = -6
            left_padding = 56
            horizontal_padding = 64
            bottom_padding = 72

            [visuals.now_playing]
            fade_duration_ms = 320
            opacity = 72
            title_opacity = 88
            artist_opacity = 54
            artwork_opacity = 61
            title_color = "#F8FBFF"
            artist_color = "#C8D4EC"
            title_font_family = "Geom"
            artist_font_family = "Prototype"
            title_font_weight = 700
            artist_font_weight = 500
            title_size = 2
            artist_size = 1
            width = 280
            text_gap = 10
            artwork_size = 64
            artwork_radius = 16
            right_padding = 52
            bottom_padding = 56
            right_offset = -6
            bottom_offset = 10

            [visuals.layout]
            header_top_offset = -12
            auth_stack_offset = 0

            [visuals.palette]
            foreground = "rgba(255, 255, 255, 0.1)"
            muted = "rgba(255, 255, 255, 0.9)"
            pending = "rgba(255, 255, 255, 0.9)"
            rejected = "rgba(255, 255, 255, 0.9)"
        "##,
    )
    .expect("nested visual config should parse");

    assert_eq!(
        config.visuals.input_background_color(),
        RgbColor::rgb(255, 255, 255)
    );
    assert_eq!(config.visuals.input_background_opacity(), Some(5));
    assert_eq!(
        config.visuals.input_border_color(),
        RgbColor::rgb(221, 221, 221)
    );
    assert_eq!(config.visuals.input_border_opacity(), Some(12));
    assert_eq!(config.visuals.input_width(), Some(310));
    assert_eq!(config.visuals.input_height(), Some(54));
    assert_eq!(config.visuals.input_radius(), 10);
    assert_eq!(config.visuals.input_border_width(), Some(0));
    assert_eq!(
        config.visuals.avatar_background_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.avatar_size(), Some(192));
    assert_eq!(config.visuals.avatar_gap(), Some(14));
    assert_eq!(
        config.visuals.username_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.username_opacity(), Some(84));
    assert_eq!(config.visuals.username_size(), Some(4));
    assert_eq!(config.visuals.username_gap(), Some(28));
    assert_eq!(config.visuals.clock_font_family(), Some("Prototype"));
    assert_eq!(config.visuals.clock_font_weight(), Some(700));
    assert_eq!(config.visuals.clock_format(), ClockFormat::TwelveHour);
    assert_eq!(config.visuals.clock_meridiem_size(), Some(3));
    assert_eq!(config.visuals.clock_meridiem_offset_x(), Some(6));
    assert_eq!(config.visuals.clock_meridiem_offset_y(), Some(-2));
    assert_eq!(
        config.visuals.clock_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.clock_opacity(), Some(40));
    assert_eq!(config.visuals.clock_size(), Some(14));
    assert_eq!(config.visuals.clock_gap(), Some(20));
    assert_eq!(
        config.visuals.date_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.date_font_family(), Some("Geom"));
    assert_eq!(config.visuals.date_font_weight(), Some(600));
    assert_eq!(config.visuals.date_opacity(), Some(40));
    assert_eq!(config.visuals.date_size(), Some(2));
    assert_eq!(
        config.visuals.placeholder_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.placeholder_opacity(), Some(60));
    assert_eq!(
        config.visuals.status_color(),
        Some(RgbColor::rgb(255, 224, 160))
    );
    assert_eq!(config.visuals.status_opacity(), Some(88));
    assert_eq!(config.visuals.status_gap(), Some(18));
    assert_eq!(
        config.visuals.eye_icon_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(config.visuals.eye_icon_opacity(), Some(72));
    assert_eq!(
        config.visuals.keyboard_background_color(),
        Some(RgbColor::rgba(18, 22, 30, 82))
    );
    assert_eq!(config.visuals.keyboard_background_size(), Some(42));
    assert_eq!(
        config.visuals.keyboard_color(),
        Some(RgbColor::rgb(232, 238, 249))
    );
    assert_eq!(config.visuals.keyboard_opacity(), Some(68));
    assert_eq!(config.visuals.keyboard_size(), Some(3));
    assert_eq!(config.visuals.keyboard_top_offset(), Some(-12));
    assert_eq!(config.visuals.keyboard_right_offset(), Some(8));
    assert_eq!(config.visuals.weather_size(), Some(3));
    assert_eq!(config.visuals.weather_opacity(), Some(62));
    assert_eq!(config.visuals.weather_icon_opacity(), Some(41));
    assert_eq!(config.visuals.weather_temperature_opacity(), Some(77));
    assert_eq!(config.visuals.weather_location_opacity(), Some(53));
    assert_eq!(
        config.visuals.weather_temperature_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );
    assert_eq!(
        config.visuals.weather_location_color(),
        Some(RgbColor::rgb(214, 227, 255))
    );
    assert_eq!(
        config.visuals.weather_temperature_font_family(),
        Some("Prototype")
    );
    assert_eq!(config.visuals.weather_temperature_font_weight(), Some(600));
    assert_eq!(config.visuals.weather_temperature_letter_spacing(), Some(2));
    assert_eq!(config.visuals.weather_temperature_size(), Some(4));
    assert_eq!(config.visuals.weather_location_size(), Some(2));
    assert_eq!(config.visuals.weather_icon_size(), Some(36));
    assert_eq!(config.visuals.weather_icon_gap(), Some(10));
    assert_eq!(config.visuals.weather_location_gap(), Some(3));
    assert_eq!(
        config.visuals.weather_alignment(),
        super::WeatherAlignment::Right
    );
    assert_eq!(config.visuals.weather_left_offset(), Some(12));
    assert_eq!(config.visuals.weather_bottom_offset(), Some(-6));
    assert_eq!(config.visuals.weather_horizontal_padding(), Some(64));
    assert_eq!(config.visuals.weather_left_padding(), Some(56));
    assert_eq!(config.visuals.weather_bottom_padding(), Some(72));
    assert_eq!(
        config.visuals.now_playing_title_color(),
        Some(RgbColor::rgb(248, 251, 255))
    );
    assert_eq!(
        config.visuals.now_playing_artist_color(),
        Some(RgbColor::rgb(200, 212, 236))
    );
    assert_eq!(config.visuals.now_playing_fade_duration_ms(), Some(320));
    assert_eq!(config.visuals.now_playing_title_font_family(), Some("Geom"));
    assert_eq!(
        config.visuals.now_playing_artist_font_family(),
        Some("Prototype")
    );
    assert_eq!(config.visuals.now_playing_title_font_weight(), Some(700));
    assert_eq!(config.visuals.now_playing_artist_font_weight(), Some(500));
    assert_eq!(config.visuals.now_playing_opacity(), Some(72));
    assert_eq!(config.visuals.now_playing_title_opacity(), Some(88));
    assert_eq!(config.visuals.now_playing_artist_opacity(), Some(54));
    assert_eq!(config.visuals.now_playing_artwork_opacity(), Some(61));
    assert_eq!(config.visuals.now_playing_title_size(), Some(2));
    assert_eq!(config.visuals.now_playing_artist_size(), Some(1));
    assert_eq!(config.visuals.now_playing_width(), Some(280));
    assert_eq!(config.visuals.now_playing_text_gap(), Some(10));
    assert_eq!(config.visuals.now_playing_artwork_size(), Some(64));
    assert_eq!(config.visuals.now_playing_artwork_radius(), Some(16));
    assert_eq!(config.visuals.now_playing_right_padding(), Some(52));
    assert_eq!(config.visuals.now_playing_bottom_padding(), Some(56));
    assert_eq!(config.visuals.now_playing_right_offset(), Some(-6));
    assert_eq!(config.visuals.now_playing_bottom_offset(), Some(10));
    assert_eq!(config.visuals.header_top_offset(), Some(-12));
    assert_eq!(config.visuals.auth_stack_offset(), Some(0));
    assert_eq!(
        config.visuals.foreground_color(),
        RgbColor::rgba(255, 255, 255, 26)
    );
    assert_eq!(
        config.visuals.muted_color(),
        RgbColor::rgba(255, 255, 255, 230)
    );
    assert_eq!(
        config.visuals.pending_color(),
        RgbColor::rgba(255, 255, 255, 230)
    );
    assert_eq!(
        config.visuals.rejected_color(),
        RgbColor::rgba(255, 255, 255, 230)
    );
}
