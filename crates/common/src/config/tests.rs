use std::fs;

use super::{AppConfig, InputVisualEntry, RgbColor};

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
    assert_eq!(config.background.color, RgbColor::rgb(12, 16, 24));
    assert!(config.background.path.is_none());
    assert_eq!(config.background.blur_radius, 0);
    assert_eq!(config.background.dim_strength, 34);
    assert!(config.background.tint.is_none());
    assert_eq!(config.background.tint_opacity, 0);
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
    assert!(config.visuals.clock_color().is_none());
    assert!(config.visuals.clock_opacity().is_none());
    assert!(config.visuals.date_color().is_none());
    assert!(config.visuals.date_opacity().is_none());
    assert!(config.visuals.clock_size().is_none());
    assert!(config.visuals.date_size().is_none());
    assert!(config.visuals.placeholder_color().is_none());
    assert!(config.visuals.placeholder_opacity().is_none());
    assert!(config.visuals.eye_icon_color().is_none());
    assert!(config.visuals.eye_icon_opacity().is_none());
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
            color = "#ffffff"
            opacity = 40
            size = 14
            gap = 20

            [visuals.date]
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
