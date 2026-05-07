use super::*;

#[test]
fn lists_bundled_theme_names() {
    let themes = bundled_theme_names().expect("bundled themes should load");

    assert!(!themes.is_empty());
    assert!(themes.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(themes.iter().all(|theme| !theme.ends_with(".toml")));
}

#[test]
fn loads_bundled_default_theme_as_default_layer() {
    let (_path, raw) = read_theme_source(None, "default").expect("default theme should load");
    let theme_config = AppConfig::from_toml_str(&raw).expect("default theme should parse");
    let config = AppConfig::from_default_layers().expect("default config should load");

    assert_eq!(
        config.background.blur_radius,
        theme_config.background.blur_radius
    );
    assert_eq!(
        config.background.dim_strength,
        theme_config.background.dim_strength
    );
    assert_eq!(config.weather.enabled, theme_config.weather.enabled);
    assert_eq!(config.battery.enabled, theme_config.battery.enabled);
    assert_eq!(
        config.visuals.avatar_size(),
        theme_config.visuals.avatar_size()
    );
    assert_eq!(
        config.visuals.clock_font_family(),
        theme_config.visuals.clock_font_family()
    );
    assert_eq!(
        config.visuals.date_color(),
        theme_config.visuals.date_color()
    );
}

#[test]
fn flat_visual_overrides_win_over_bundled_default_theme_layer() {
    let dir = std::env::temp_dir().join(format!("veila-flat-default-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        r##"
            [visuals]
            avatar_background_color = "rgba(24, 30, 42, 0.82)"
            clock_font_family = "Bebas Neue"
            clock_color = "#F8FBFFF5"
        "##,
    )
    .expect("config file");

    let loaded = AppConfig::load(Some(&path)).expect("config should load");

    assert_eq!(
        loaded.config.visuals.avatar_background_color(),
        Some(RgbColor::rgba(24, 30, 42, 209))
    );
    assert_eq!(
        loaded.config.visuals.clock_font_family(),
        Some("Bebas Neue")
    );
    assert_eq!(
        loaded.config.visuals.clock_color(),
        Some(RgbColor::rgba(248, 251, 255, 245))
    );

    fs::remove_file(path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn loads_bundled_theme_before_user_overrides() {
    let dir = std::env::temp_dir().join(format!("veila-theme-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    let (_theme_path, raw_theme) = read_theme_source(None, "boracay").expect("theme source");
    let theme_config = AppConfig::from_toml_str(&raw_theme).expect("theme should parse");
    fs::write(
        &path,
        r#"
            theme = "boracay"

            [visuals.clock]
            size = 16
        "#,
    )
    .expect("config file");

    let loaded = AppConfig::load(Some(&path)).expect("config should load");

    assert_eq!(
        loaded.config.visuals.clock_font_family(),
        theme_config.visuals.clock_font_family()
    );
    assert_eq!(
        loaded.config.visuals.clock_font_weight(),
        theme_config.visuals.clock_font_weight()
    );
    assert_eq!(loaded.config.visuals.clock_size(), Some(16));
    assert_eq!(
        loaded.config.visuals.weather_icon_position(),
        theme_config.visuals.weather_icon_position()
    );
    assert_eq!(
        loaded.config.visuals.now_playing_title_color(),
        theme_config.visuals.now_playing_title_color()
    );

    fs::remove_file(path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn loads_second_bundled_theme() {
    let dir = std::env::temp_dir().join(format!("veila-theme-normandy-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    let (_theme_path, raw_theme) = read_theme_source(None, "normandy").expect("theme source");
    let theme_config = AppConfig::from_toml_str(&raw_theme).expect("theme should parse");
    fs::write(
        &path,
        r#"
            theme = "normandy"
        "#,
    )
    .expect("write config");

    let config = AppConfig::load_from_file(&path).expect("config should load");

    assert_eq!(config.background.color, theme_config.background.color);
    assert_eq!(
        config.background.blur_radius,
        theme_config.background.blur_radius
    );
    assert_eq!(
        config.visuals.clock_font_family(),
        theme_config.visuals.clock_font_family()
    );
    assert_eq!(
        config.visuals.clock_font_weight(),
        theme_config.visuals.clock_font_weight()
    );
    assert_eq!(
        config.visuals.date_color(),
        theme_config.visuals.date_color()
    );
    assert_eq!(
        config.visuals.keyboard_background_color(),
        theme_config.visuals.keyboard_background_color()
    );
    assert_eq!(config.weather.enabled, theme_config.weather.enabled);
    assert_eq!(
        config.visuals.weather_icon_position(),
        theme_config.visuals.weather_icon_position()
    );
    assert_eq!(
        config.visuals.now_playing_title_color(),
        theme_config.visuals.now_playing_title_color()
    );
    assert_eq!(
        config.visuals.now_playing_artist_color(),
        theme_config.visuals.now_playing_artist_color()
    );

    fs::remove_file(path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn loads_user_theme_from_config_directory() {
    let dir = std::env::temp_dir().join(format!("veila-user-theme-{}", std::process::id()));
    let themes_dir = dir.join("themes");
    fs::create_dir_all(&themes_dir).expect("temp dir");
    let path = dir.join("config.toml");
    fs::write(
        themes_dir.join("custom.toml"),
        r##"
            [visuals.clock]
            font_family = "Google Sans Flex"
            color = "#FFFFFF9C"
        "##,
    )
    .expect("theme file");
    fs::write(
        &path,
        r#"
            theme = "custom"

            [visuals.clock]
            size = 17
        "#,
    )
    .expect("config file");

    let loaded = AppConfig::load(Some(&path)).expect("config should load");

    assert_eq!(
        loaded.config.visuals.clock_font_family(),
        Some("Google Sans Flex")
    );
    assert_eq!(
        loaded.config.visuals.clock_color(),
        Some(RgbColor::rgba(255, 255, 255, 156))
    );
    assert_eq!(loaded.config.visuals.clock_size(), Some(17));

    fs::remove_file(themes_dir.join("custom.toml")).ok();
    fs::remove_dir(themes_dir).ok();
    fs::remove_file(path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn resolves_active_user_theme_source_path() {
    let dir = std::env::temp_dir().join(format!("veila-active-theme-{}", std::process::id()));
    let themes_dir = dir.join("themes");
    fs::create_dir_all(&themes_dir).expect("temp dir");
    let config_path = dir.join("config.toml");
    let theme_path = themes_dir.join("custom.toml");
    fs::write(&theme_path, "[visuals.clock]\nsize = 17\n").expect("theme file");
    fs::write(&config_path, "theme = \"custom\"\n").expect("config file");

    let resolved =
        active_theme_source_path(Some(&config_path)).expect("theme source should resolve");

    assert_eq!(resolved.as_deref(), Some(theme_path.as_path()));

    fs::remove_file(theme_path).ok();
    fs::remove_dir(themes_dir).ok();
    fs::remove_file(config_path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn resolves_active_theme_name() {
    let dir = std::env::temp_dir().join(format!("veila-active-theme-name-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let config_path = dir.join("config.toml");
    fs::write(&config_path, "theme = \"custom\"\n").expect("config file");

    let resolved = active_theme_name(Some(&config_path)).expect("theme name should resolve");

    assert_eq!(resolved.as_deref(), Some("custom"));

    fs::remove_file(config_path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn errors_for_unknown_theme_preset() {
    let dir = std::env::temp_dir().join(format!("veila-missing-theme-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        r#"
            theme = "missing_theme"
        "#,
    )
    .expect("config file");

    let error = AppConfig::load(Some(&path)).expect_err("theme should fail");

    assert!(matches!(error, VeilaError::ThemeNotFound(theme) if theme == "missing_theme"));

    fs::remove_file(path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn reads_bundled_theme_source() {
    let (path, raw) = read_theme_source(None, "boracay").expect("theme source should load");
    let config = AppConfig::from_toml_str(&raw).expect("theme should parse");

    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("boracay.toml")
    );
    assert!(raw.contains("[visuals.clock]"));
    assert!(config.visuals.clock_font_family().is_some());
}
