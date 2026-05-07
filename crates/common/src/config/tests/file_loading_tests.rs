use super::*;

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

            [battery]
            enabled = true
            refresh_seconds = 45
            mock_percent = 84
            mock_charging = true

            [visuals]
            avatar_background_color = "rgba(24, 30, 42, 0.82)"
            input = "#FFFFFF1A"
            input_border = "#FFFFFF1F"
            input_font_family = "Geom"
            input_font_weight = 600
            input_font_style = "italic"
            input_font_size = 3
            input_width = 280
            input_height = 54
            input_radius = 20
            input_border_width = 3
            avatar_size = 92
            avatar_placeholder_padding = 12
            avatar_icon_color = "#E8EEF9"
            avatar_ring_color = "#94B2FF"
            avatar_ring_width = 3
            username_color = "#D7E3FFB8"
            username_size = 3
            auth_stack_offset = 16
            header_top_offset = -12
            identity_gap = 26
            center_stack_style = "identity-hero-input"
            clock_font_family = "Bebas Neue"
            clock_font_weight = 700
            clock_font_style = "italic"
            clock_style = "stacked"
            clock_format = "12h"
            clock_meridiem_size = 3
            clock_meridiem_offset_x = 6
            clock_meridiem_offset_y = -2
            clock_color = "#F8FBFFF5"
            date_color = "#C8D4ECBD"
            clock_size = 4
            date_size = 3
            placeholder_color = "#8694B499"
            eye_icon_color = "#F4F8FFB8"
            status_color = "#FFE0A0E0"
            input_mask_color = "#A9C4FF"

            [visuals.avatar]
            halign = "center"
            valign = "center"
            x = 0
            y = 32

            [visuals.username]
            halign = "center"
            valign = "center"
            x = 0
            y = 220

            [visuals.clock]
            halign = "left"
            valign = "top"
            x = 24
            y = 40

            [visuals.date]
            halign = "left"
            valign = "top"
            x = 24
            y = 156

            [visuals.status]
            halign = "center"
            valign = "bottom"
            x = 0
            y = -24
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
    assert!(loaded.config.battery.enabled);
    assert_eq!(loaded.config.battery.refresh_seconds, 45);
    assert_eq!(loaded.config.battery.mock_percent, Some(84));
    assert_eq!(loaded.config.battery.mock_charging, Some(true));
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
    assert_eq!(
        loaded.config.visuals.avatar_background_color(),
        Some(RgbColor::rgba(24, 30, 42, 209))
    );
    assert_eq!(
        loaded.config.visuals.input_background_color(),
        RgbColor::rgba(255, 255, 255, 26)
    );
    assert_eq!(loaded.config.visuals.input_font_family(), Some("Geom"));
    assert_eq!(loaded.config.visuals.input_font_weight(), Some(600));
    assert_eq!(
        loaded.config.visuals.input_font_style(),
        Some(FontStyle::Italic)
    );
    assert_eq!(loaded.config.visuals.input_font_size(), Some(3));
    assert_eq!(
        loaded.config.visuals.input_border_color(),
        RgbColor::rgba(255, 255, 255, 31)
    );
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
    assert_eq!(
        loaded.config.visuals.username_color(),
        Some(RgbColor::rgba(215, 227, 255, 184))
    );
    assert_eq!(loaded.config.visuals.username_size(), Some(3));
    assert_eq!(
        loaded.config.visuals.status_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Center),
            valign: Some(VerticalAlign::Bottom),
            x: Some(0),
            y: Some(-24),
        }
    );
    assert_eq!(loaded.config.visuals.auth_stack_offset(), Some(16));
    assert_eq!(loaded.config.visuals.header_top_offset(), Some(-12));
    assert_eq!(loaded.config.visuals.identity_gap(), Some(26));
    assert_eq!(
        loaded.config.visuals.center_stack_style(),
        CenterStackStyle::IdentityHeroInput
    );
    assert_eq!(
        loaded.config.visuals.clock_font_family(),
        Some("Bebas Neue")
    );
    assert_eq!(loaded.config.visuals.clock_font_weight(), Some(700));
    assert_eq!(
        loaded.config.visuals.clock_font_style(),
        Some(FontStyle::Italic)
    );
    assert_eq!(loaded.config.visuals.clock_style(), ClockStyle::Stacked);
    assert_eq!(
        loaded.config.visuals.clock_format(),
        ClockFormat::TwelveHour
    );
    assert_eq!(loaded.config.visuals.clock_meridiem_size(), Some(3));
    assert_eq!(loaded.config.visuals.clock_meridiem_offset_x(), Some(6));
    assert_eq!(loaded.config.visuals.clock_meridiem_offset_y(), Some(-2));
    assert_eq!(
        loaded.config.visuals.clock_color(),
        Some(RgbColor::rgba(248, 251, 255, 245))
    );
    assert_eq!(
        loaded.config.visuals.avatar_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Center),
            valign: Some(VerticalAlign::Center),
            x: Some(0),
            y: Some(32),
        }
    );
    assert_eq!(
        loaded.config.visuals.username_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Center),
            valign: Some(VerticalAlign::Center),
            x: Some(0),
            y: Some(220),
        }
    );
    assert_eq!(
        loaded.config.visuals.clock_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Left),
            valign: Some(VerticalAlign::Top),
            x: Some(24),
            y: Some(40),
        }
    );
    assert_eq!(
        loaded.config.visuals.date_position(),
        WidgetPositionConfig {
            halign: Some(HorizontalAlign::Left),
            valign: Some(VerticalAlign::Top),
            x: Some(24),
            y: Some(156),
        }
    );
    assert_eq!(
        loaded.config.visuals.date_color(),
        Some(RgbColor::rgba(200, 212, 236, 189))
    );
    assert_eq!(loaded.config.visuals.clock_size(), Some(4));
    assert_eq!(loaded.config.visuals.date_size(), Some(3));
    assert_eq!(
        loaded.config.visuals.placeholder_color(),
        Some(RgbColor::rgba(134, 148, 180, 153))
    );
    assert_eq!(
        loaded.config.visuals.eye_icon_color(),
        Some(RgbColor::rgba(244, 248, 255, 184))
    );
    assert_eq!(
        loaded.config.visuals.status_color(),
        Some(RgbColor::rgba(255, 224, 160, 224))
    );
    assert_eq!(
        loaded.config.visuals.input_mask_color(),
        Some(RgbColor::rgb(169, 196, 255))
    );

    fs::remove_file(path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn loads_include_files_before_main_config_overrides() {
    let dir = std::env::temp_dir().join(format!("veila-include-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let include_path = dir.join("matugen.toml");
    let config_path = dir.join("config.toml");

    fs::write(
        &include_path,
        r##"
            theme = "ignored"
            include = "ignored.toml"

            [visuals.palette]
            foreground = "#F1E8D9"
            muted = "#B8AFA3"
            panel = "#17130F"

            [visuals.clock]
            color = "#F1E8D9"
            size = 42

            [visuals.input]
            background_color = "#17130F"
            border_color = "#D69F72"
        "##,
    )
    .expect("include file");
    fs::write(
        &config_path,
        r##"
            include = ["matugen.toml", "missing.toml"]

            [visuals.clock]
            size = 52
        "##,
    )
    .expect("config file");

    let loaded = AppConfig::load(Some(&config_path)).expect("config should load");

    assert_eq!(
        loaded.config.visuals.foreground_color(),
        RgbColor::rgb(241, 232, 217)
    );
    assert_eq!(
        loaded.config.visuals.clock_color(),
        Some(RgbColor::rgb(241, 232, 217))
    );
    assert_eq!(loaded.config.visuals.clock_size(), Some(52));
    assert_eq!(
        loaded.config.visuals.input_background_color(),
        RgbColor::rgb(23, 19, 15)
    );
    assert_eq!(
        loaded.config.visuals.input_border_color(),
        RgbColor::rgb(214, 159, 114)
    );

    let include_paths =
        active_include_source_paths(Some(&config_path)).expect("include paths should resolve");
    assert_eq!(include_paths, vec![include_path, dir.join("missing.toml")]);

    fs::remove_file(config_path).ok();
    fs::remove_file(dir.join("matugen.toml")).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn existing_invalid_include_file_fails_config_load() {
    let dir = std::env::temp_dir().join(format!("veila-invalid-include-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let include_path = dir.join("broken.toml");
    let config_path = dir.join("config.toml");

    fs::write(&include_path, b"[visuals.clock\nsize = 42\n").expect("include file");
    fs::write(&config_path, b"include = [\"broken.toml\"]\n").expect("config file");

    let error = AppConfig::load(Some(&config_path)).expect_err("config should fail");
    assert!(matches!(error, VeilaError::Config(_)));

    fs::remove_file(config_path).ok();
    fs::remove_file(include_path).ok();
    fs::remove_dir(dir).ok();
}
