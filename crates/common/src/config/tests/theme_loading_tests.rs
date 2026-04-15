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
    let value: toml::Value = toml::from_str(&raw).expect("default theme should parse");
    let config = AppConfig::from_default_layers().expect("default config should load");

    assert_eq!(
        config.background.blur_radius,
        theme_u8(&value, "background", "blur_radius").expect("theme blur radius")
    );
    assert_eq!(
        config.background.dim_strength,
        theme_u8(&value, "background", "dim_strength").expect("theme dim strength")
    );
    assert_eq!(
        config.weather.enabled,
        theme_bool(&value, "weather", "enabled").expect("theme weather state")
    );
    assert_eq!(
        config.battery.enabled,
        theme_bool(&value, "battery", "enabled").expect("theme battery state")
    );
    assert_eq!(
        config.visuals.avatar_size(),
        theme_u16(&value, "visuals.avatar", "size")
    );
    assert_eq!(
        config.visuals.clock_font_family(),
        theme_str(&value, "visuals.clock", "font_family")
    );
    assert_eq!(
        config.visuals.date_opacity(),
        theme_u8(&value, "visuals.date", "opacity")
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
            clock_opacity = 96
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
    assert_eq!(loaded.config.visuals.clock_opacity(), Some(96));

    fs::remove_file(path).ok();
    fs::remove_dir(dir).ok();
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
    assert_eq!(
        config.visuals.weather_alignment(),
        theme_config.visuals.weather_alignment()
    );
    assert_eq!(
        config.visuals.now_playing_opacity(),
        theme_config.visuals.now_playing_opacity()
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

    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("boracay.toml")
    );
    assert!(raw.contains("font_family = \"Nunito\""));
    assert!(raw.contains("style = \"stacked\""));
}

fn theme_value<'a>(value: &'a toml::Value, section: &str, key: &str) -> Option<&'a toml::Value> {
    let mut current = value;
    for part in section.split('.') {
        current = current.get(part)?;
    }
    current.get(key)
}

fn theme_bool(value: &toml::Value, section: &str, key: &str) -> Option<bool> {
    theme_value(value, section, key)?.as_bool()
}

fn theme_str<'a>(value: &'a toml::Value, section: &str, key: &str) -> Option<&'a str> {
    theme_value(value, section, key)?.as_str()
}

fn theme_u8(value: &toml::Value, section: &str, key: &str) -> Option<u8> {
    theme_value(value, section, key)?
        .as_integer()
        .and_then(|value| u8::try_from(value).ok())
}

fn theme_u16(value: &toml::Value, section: &str, key: &str) -> Option<u16> {
    theme_value(value, section, key)?
        .as_integer()
        .and_then(|value| u16::try_from(value).ok())
}
