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
fn parses_lock_screen_off_seconds() {
    let config = AppConfig::from_toml_str(
        r#"
            [lock]
            screen_off_seconds = 10
        "#,
    )
    .expect("config should parse");

    assert_eq!(config.lock.screen_off_seconds, Some(10));
}

#[test]
fn parses_lock_suspend_seconds() {
    let config = AppConfig::from_toml_str(
        r#"
            [lock]
            suspend_seconds = 300
        "#,
    )
    .expect("config should parse");

    assert_eq!(config.lock.suspend_seconds, Some(300));
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
fn partial_input_table_keeps_explicit_fill_and_default_disabled_border() {
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
    assert_eq!(
        config.visuals.input_border_color(),
        RgbColor::rgba(255, 255, 255, 0)
    );
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
fn expands_home_in_background_paths() {
    let home = std::env::var("HOME").expect("HOME should be set for path expansion");
    let config = AppConfig::from_toml_str(
        r#"
            [background]
            mode = "file"
            path = "~/Pictures/wallpapers/default.jpg"

            [[background.outputs]]
            name = "DP-1"
            path = "~/Pictures/wallpapers/left.jpg"
        "#,
    )
    .expect("config should parse");

    assert_eq!(
        config.background.resolved_path().as_deref(),
        Some(
            std::path::PathBuf::from(&home)
                .join("Pictures/wallpapers/default.jpg")
                .as_path()
        )
    );
    assert_eq!(
        config
            .background
            .resolved_path_for_output(Some("DP-1"))
            .as_deref(),
        Some(
            std::path::PathBuf::from(&home)
                .join("Pictures/wallpapers/left.jpg")
                .as_path()
        )
    );
}

#[test]
fn infers_file_mode_from_background_slideshow() {
    let config = AppConfig::from_toml_str(
        r#"
            [background.slideshow]
            files = ["/tmp/one.jpg", "/tmp/two.png"]
        "#,
    )
    .expect("config should parse");

    assert_eq!(config.background.effective_mode(), BackgroundMode::File);
    assert!(config.background.resolved_path().is_none());
    assert!(config.background.slideshow_enabled());
}

#[test]
fn resolves_random_background_slideshow_initial_path_from_seed() {
    let temp_dir = std::env::temp_dir().join(format!(
        "veila-background-slideshow-seed-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).expect("temp dir");
    let first = temp_dir.join("one.jpg");
    let second = temp_dir.join("two.jpg");
    let third = temp_dir.join("three.jpg");
    std::fs::write(&first, b"jpg").expect("first file");
    std::fs::write(&second, b"jpg").expect("second file");
    std::fs::write(&third, b"jpg").expect("third file");

    let config = AppConfig::from_toml_str(&format!(
        r#"
            [background.slideshow]
            files = ["{}", "{}", "{}"]
            order = "random"
        "#,
        first.display(),
        second.display(),
        third.display()
    ))
    .expect("config should parse");

    assert_eq!(
        config
            .background
            .resolved_slideshow_initial_path_with_seed(0)
            .expect("initial slideshow path should resolve")
            .as_deref(),
        Some(first.as_path())
    );
    assert_eq!(
        config
            .background
            .resolved_slideshow_initial_path_with_seed(1)
            .expect("initial slideshow path should resolve")
            .as_deref(),
        Some(second.as_path())
    );
    assert_eq!(
        config
            .background
            .resolved_slideshow_initial_path_with_seed(2)
            .expect("initial slideshow path should resolve")
            .as_deref(),
        Some(third.as_path())
    );

    std::fs::remove_file(first).ok();
    std::fs::remove_file(second).ok();
    std::fs::remove_file(third).ok();
    std::fs::remove_dir(temp_dir).ok();
}

#[test]
fn resolves_background_slideshow_files_and_directory() {
    let temp_dir =
        std::env::temp_dir().join(format!("veila-background-slideshow-{}", std::process::id()));
    let slideshow_dir = temp_dir.join("slides");
    std::fs::create_dir_all(&slideshow_dir).expect("slideshow dir");

    let explicit = temp_dir.join("explicit.jpg");
    let duplicate = slideshow_dir.join("00-duplicate.jpg");
    let second = slideshow_dir.join("01-second.png");
    let ignored = slideshow_dir.join("notes.txt");

    std::fs::write(&explicit, b"jpg").expect("explicit file");
    std::fs::write(&duplicate, b"jpg").expect("duplicate file");
    std::fs::write(&second, b"png").expect("second file");
    std::fs::write(&ignored, b"txt").expect("ignored file");

    let config = AppConfig::from_toml_str(&format!(
        r#"
            [background.slideshow]
            files = ["{}", "{}"]
            directory = "{}"
            order = "random"
            change_every_seconds = 45
        "#,
        explicit.display(),
        duplicate.display(),
        slideshow_dir.display()
    ))
    .expect("config should parse");

    let slideshow = config
        .background
        .slideshow
        .as_ref()
        .expect("slideshow config should exist");
    assert_eq!(
        slideshow.order,
        crate::config::BackgroundSlideshowOrder::Random
    );
    assert_eq!(
        slideshow.mode,
        crate::config::BackgroundSlideshowMode::Timed
    );
    assert_eq!(slideshow.change_interval().as_secs(), 45);
    assert_eq!(
        config
            .background
            .resolved_slideshow_paths()
            .expect("slideshow paths should resolve"),
        vec![explicit.clone(), duplicate.clone(), second.clone()]
    );
    assert_eq!(
        config
            .background
            .resolved_slideshow_initial_path_with_seed(0)
            .expect("slideshow initial path should resolve")
            .as_deref(),
        Some(explicit.as_path())
    );

    std::fs::remove_file(explicit).ok();
    std::fs::remove_file(duplicate).ok();
    std::fs::remove_file(second).ok();
    std::fs::remove_file(ignored).ok();
    std::fs::remove_dir(slideshow_dir).ok();
    std::fs::remove_dir(temp_dir).ok();
}

#[test]
fn parses_background_slideshow_lock_only_mode() {
    let config = AppConfig::from_toml_str(
        r#"
            [background.slideshow]
            files = ["/tmp/one.jpg", "/tmp/two.jpg"]
            order = "random"
            mode = "lock_only"
        "#,
    )
    .expect("config should parse");

    let slideshow = config
        .background
        .slideshow
        .as_ref()
        .expect("slideshow config should exist");
    assert_eq!(
        slideshow.mode,
        crate::config::BackgroundSlideshowMode::LockOnly
    );
    assert!(!slideshow.rotates_while_locked());
}

#[test]
fn parses_background_file_scaling_mode() {
    let config = AppConfig::from_toml_str(
        r#"
            [background]
            mode = "file"
            path = "/tmp/wallpaper.jpg"
            scaling = "fit"
        "#,
    )
    .expect("config should parse");

    assert_eq!(config.background.effective_mode(), BackgroundMode::File);
    assert_eq!(config.background.scaling, BackgroundScaling::Fit);
}

#[test]
fn parses_single_output_ui_mode() {
    let config = AppConfig::from_toml_str(
        r#"
            [visuals.outputs]
            ui_mode = "single"
            ui_output = "  DP-2  "
        "#,
    )
    .expect("config should parse");

    assert_eq!(config.visuals.output_ui_mode(), OutputUiMode::Single);
    assert_eq!(config.visuals.ui_output_name(), Some("DP-2"));
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

#[test]
fn gradient_mode_uses_configured_corner_colors() {
    let config = AppConfig::from_toml_str(
        r##"
            [background]
            mode = "gradient"

            [background.gradient]
            top_left = "#AA44FF"
            top_right = "#33BBFF"
            bottom_left = "#66E2FF"
            bottom_right = "#7744FF"
        "##,
    )
    .expect("config should parse");

    assert_eq!(config.background.effective_mode(), BackgroundMode::Gradient);
    assert!(config.background.resolved_path().is_none());

    let gradient = config
        .background
        .resolved_gradient()
        .expect("gradient should resolve");
    assert_eq!(gradient.top_left, RgbColor::rgb(170, 68, 255));
    assert_eq!(gradient.top_right, RgbColor::rgb(51, 187, 255));
    assert_eq!(gradient.bottom_left, RgbColor::rgb(102, 226, 255));
    assert_eq!(gradient.bottom_right, RgbColor::rgb(119, 68, 255));
}

#[test]
fn radial_mode_uses_configured_colors_and_position() {
    let config = AppConfig::from_toml_str(
        r##"
            [background]
            mode = "radial"

            [background.radial]
            center = "#F7F9FF"
            edge = "#3F2B7A"
            center_x = 44
            center_y = 36
            radius = 92
        "##,
    )
    .expect("config should parse");

    assert_eq!(config.background.effective_mode(), BackgroundMode::Radial);
    assert!(config.background.resolved_path().is_none());

    let radial = config
        .background
        .resolved_radial()
        .expect("radial config should resolve");
    assert_eq!(radial.center, RgbColor::rgb(247, 249, 255));
    assert_eq!(radial.edge, RgbColor::rgb(63, 43, 122));
    assert_eq!(radial.center_x, 44);
    assert_eq!(radial.center_y, 36);
    assert_eq!(radial.radius, 92);
}

#[test]
fn layered_mode_uses_base_and_blobs() {
    let config = AppConfig::from_toml_str(
        r##"
            [background]
            mode = "layered"

            [background.layered.base]
            mode = "gradient"

            [background.layered.base.gradient]
            top_left = "#AA44FF"
            top_right = "#33BBFF"
            bottom_left = "#66E2FF"
            bottom_right = "#7744FF"

            [[background.layered.blobs]]
            color = "#FFFFFF"
            opacity = 16
            x = 18
            y = 12
            size = 42

            [[background.layered.blobs]]
            color = "#7C4DFF"
            opacity = 22
            x = 82
            y = 78
            size = 50
        "##,
    )
    .expect("config should parse");

    assert_eq!(config.background.effective_mode(), BackgroundMode::Layered);
    assert!(config.background.resolved_path().is_none());

    let layered = config
        .background
        .resolved_layered()
        .expect("layered config should resolve");
    assert_eq!(
        layered.base.effective_mode(),
        crate::config::BackgroundLayeredBaseMode::Gradient
    );
    let base_gradient = layered
        .base
        .gradient
        .as_ref()
        .expect("layered gradient base should exist");
    assert_eq!(base_gradient.top_left, RgbColor::rgb(170, 68, 255));
    assert_eq!(base_gradient.top_right, RgbColor::rgb(51, 187, 255));
    assert_eq!(base_gradient.bottom_left, RgbColor::rgb(102, 226, 255));
    assert_eq!(base_gradient.bottom_right, RgbColor::rgb(119, 68, 255));
    assert_eq!(layered.blobs.len(), 2);
    assert_eq!(layered.blobs[0].color, RgbColor::rgb(255, 255, 255));
    assert_eq!(layered.blobs[0].opacity, 16);
    assert_eq!(layered.blobs[0].x, 18);
    assert_eq!(layered.blobs[0].y, 12);
    assert_eq!(layered.blobs[0].size, 42);
    assert_eq!(layered.blobs[1].color, RgbColor::rgb(124, 77, 255));
    assert_eq!(layered.blobs[1].opacity, 22);
    assert_eq!(layered.blobs[1].x, 82);
    assert_eq!(layered.blobs[1].y, 78);
    assert_eq!(layered.blobs[1].size, 50);
}

#[test]
fn legacy_bundled_mode_resolves_as_gradient() {
    let config = AppConfig::from_toml_str(
        r##"
            [background]
            mode = "bundled"

            [background.gradient]
            top_left = "#A85BFF"
            top_right = "#39B8FF"
            bottom_left = "#6FE2FF"
            bottom_right = "#6F4CFF"
        "##,
    )
    .expect("config should parse");

    assert_eq!(config.background.effective_mode(), BackgroundMode::Gradient);
    assert!(config.background.resolved_path().is_none());

    let gradient = config
        .background
        .resolved_gradient()
        .expect("legacy bundled mode should resolve a gradient");
    assert_eq!(gradient.top_left, RgbColor::rgb(168, 91, 255));
    assert_eq!(gradient.top_right, RgbColor::rgb(57, 184, 255));
    assert_eq!(gradient.bottom_left, RgbColor::rgb(111, 226, 255));
    assert_eq!(gradient.bottom_right, RgbColor::rgb(111, 76, 255));
}
