use super::*;

#[test]
fn lists_bundled_theme_names() {
    let themes = bundled_theme_names().expect("bundled themes should load");

    assert!(!themes.is_empty());
    assert!(themes.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(themes.iter().all(|theme| !theme.ends_with(".toml")));
}

#[test]
fn loads_bundled_theme_before_user_overrides() {
    let dir = std::env::temp_dir().join(format!("veila-theme-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
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

    assert_eq!(loaded.config.visuals.clock_font_family(), Some("Nunito"));
    assert_eq!(loaded.config.visuals.clock_font_weight(), Some(800));
    assert_eq!(
        loaded.config.visuals.clock_font_style(),
        Some(FontStyle::Italic)
    );
    assert_eq!(loaded.config.visuals.clock_size(), Some(16));
    assert_eq!(
        loaded.config.visuals.weather_alignment(),
        WeatherAlignment::Left
    );
    assert_eq!(
        loaded.config.visuals.now_playing_title_color(),
        Some(RgbColor::rgb(255, 255, 255))
    );

    fs::remove_file(path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn loads_second_bundled_theme() {
    let dir = std::env::temp_dir().join(format!("veila-theme-shanghai-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        r#"
            theme = "shanghai"
        "#,
    )
    .expect("write config");

    let config = AppConfig::load_from_file(&path).expect("config should load");

    assert_eq!(config.background.color, RgbColor::rgb(0, 0, 0));
    assert_eq!(config.background.blur_radius, 12);
    assert_eq!(config.visuals.clock_font_family(), Some("Google Sans Flex"));
    assert_eq!(config.visuals.clock_font_weight(), Some(400));
    assert_eq!(
        config.visuals.date_color(),
        Some(RgbColor::rgb(200, 216, 242))
    );
    assert_eq!(
        config.visuals.keyboard_background_color(),
        Some(RgbColor::rgba(255, 255, 255, 13))
    );
    assert_eq!(config.visuals.weather_alignment(), WeatherAlignment::Left);
    assert_eq!(config.visuals.now_playing_opacity(), Some(72));
}

#[test]
fn loads_user_theme_from_config_directory() {
    let dir = std::env::temp_dir().join(format!("veila-user-theme-{}", std::process::id()));
    let themes_dir = dir.join("themes");
    fs::create_dir_all(&themes_dir).expect("temp dir");
    let path = dir.join("config.toml");
    fs::write(
        themes_dir.join("custom.toml"),
        r#"
            [visuals.clock]
            font_family = "Google Sans Flex"
            opacity = 61
        "#,
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
    assert_eq!(loaded.config.visuals.clock_opacity(), Some(61));
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

    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("boracay.toml")
    );
    assert!(raw.contains("font_family = \"Nunito\""));
    assert!(raw.contains("style = \"stacked\""));
}
