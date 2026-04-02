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

            [battery]
            enabled = true
            refresh_seconds = 45
            mock_percent = 84
            mock_charging = true

            [visuals]
            avatar_background_color = "rgba(24, 30, 42, 0.82)"
            input = "#FFFFFF"
            input_opacity = 10
            input_border = "#FFFFFF"
            input_border_opacity = 12
            input_font_family = "Geom"
            input_font_weight = 600
            input_font_style = "italic"
            input_font_size = 3
            input_width = 280
            input_height = 54
            input_radius = 20
            input_border_width = 3
            avatar_size = 92
            avatar_offset_y = 18
            avatar_placeholder_padding = 12
            avatar_icon_color = "#E8EEF9"
            avatar_ring_color = "#94B2FF"
            avatar_ring_width = 3
            avatar_background_opacity = 36
            username_color = "#D7E3FF"
            username_opacity = 72
            username_size = 3
            username_offset_y = -12
            avatar_gap = 14
            username_gap = 28
            status_gap = 18
            clock_gap = 10
            auth_stack_offset = 16
            header_top_offset = -12
            identity_gap = 26
            center_stack_order = "auth-hero"
            center_stack_style = "identity-hero-input"
            clock_font_family = "Bebas Neue"
            clock_font_weight = 700
            clock_font_style = "italic"
            clock_style = "stacked"
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
    assert_eq!(loaded.config.background.tint_opacity, 12);
    assert_eq!(
        loaded.config.visuals.avatar_background_color(),
        Some(RgbColor::rgba(24, 30, 42, 209))
    );
    assert_eq!(
        loaded.config.visuals.input_background_color(),
        RgbColor::rgb(255, 255, 255)
    );
    assert_eq!(loaded.config.visuals.input_font_family(), Some("Geom"));
    assert_eq!(loaded.config.visuals.input_font_weight(), Some(600));
    assert_eq!(
        loaded.config.visuals.input_font_style(),
        Some(FontStyle::Italic)
    );
    assert_eq!(loaded.config.visuals.input_font_size(), Some(3));
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
    assert_eq!(loaded.config.visuals.avatar_offset_y(), Some(18));
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
    assert_eq!(loaded.config.visuals.username_offset_y(), Some(-12));
    assert_eq!(loaded.config.visuals.avatar_gap(), Some(14));
    assert_eq!(loaded.config.visuals.username_gap(), Some(28));
    assert_eq!(loaded.config.visuals.status_gap(), Some(18));
    assert_eq!(loaded.config.visuals.clock_gap(), Some(10));
    assert_eq!(loaded.config.visuals.auth_stack_offset(), Some(16));
    assert_eq!(loaded.config.visuals.header_top_offset(), Some(-12));
    assert_eq!(loaded.config.visuals.identity_gap(), Some(26));
    assert_eq!(
        loaded.config.visuals.center_stack_order(),
        CenterStackOrder::AuthHero
    );
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
