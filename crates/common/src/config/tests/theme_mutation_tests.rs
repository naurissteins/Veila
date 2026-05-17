use super::*;

#[test]
fn set_theme_in_config_creates_missing_file() {
    let dir = std::env::temp_dir().join(format!("veila-set-theme-create-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    let (_theme_path, raw_theme) = read_theme_source(None, "boracay").expect("theme source");
    let theme_config = AppConfig::from_toml_str(&raw_theme).expect("theme should parse");

    let written_path = set_theme_in_config(Some(&path), "boracay").expect("theme should set");

    assert_eq!(written_path, path);
    let raw = fs::read_to_string(&written_path).expect("written config");
    assert!(raw.contains("theme = \"boracay\""));

    let loaded = AppConfig::load(Some(&written_path)).expect("config should load");
    assert_eq!(
        loaded.config.visuals.clock_font_family(),
        theme_config.visuals.clock_font_family()
    );

    fs::remove_file(written_path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn set_theme_in_config_preserves_existing_overrides() {
    let dir = std::env::temp_dir().join(format!("veila-set-theme-preserve-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        r#"
            [visuals.username]
            enabled = false

            [visuals.input]
            width = 420
        "#,
    )
    .expect("config file");

    set_theme_in_config(Some(&path), "normandy").expect("theme should set");

    let raw = fs::read_to_string(&path).expect("written config");
    assert!(raw.contains("theme = \"normandy\""));
    assert!(raw.contains("enabled = false"));
    assert!(raw.contains("width = 420"));

    let loaded = AppConfig::load(Some(&path)).expect("config should load");
    assert!(!loaded.config.visuals.username_enabled());
    assert_eq!(loaded.config.visuals.input_width(), Some(420));
    assert_eq!(
        loaded.config.visuals.clock_font_family(),
        Some("Google Sans Flex")
    );

    fs::remove_file(path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn unset_theme_in_config_removes_only_theme_key() {
    let dir = std::env::temp_dir().join(format!("veila-unset-theme-remove-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        r#"
            theme = "normandy"

            [visuals.username]
            enabled = false

            [visuals.input]
            width = 420
        "#,
    )
    .expect("config file");

    let (written_path, changed) = unset_theme_in_config(Some(&path)).expect("theme should unset");

    assert_eq!(written_path, path);
    assert!(changed);

    let raw = fs::read_to_string(&path).expect("written config");
    assert!(!raw.contains("theme ="));
    assert!(raw.contains("enabled = false"));
    assert!(raw.contains("width = 420"));

    let loaded = AppConfig::load(Some(&path)).expect("config should load");
    assert!(!loaded.config.visuals.username_enabled());
    assert_eq!(loaded.config.visuals.input_width(), Some(420));
    assert_eq!(loaded.config.visuals.clock_font_family(), Some("Geom"));

    fs::remove_file(written_path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn unset_theme_in_config_returns_not_changed_for_missing_file() {
    let dir =
        std::env::temp_dir().join(format!("veila-unset-theme-missing-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");

    let (written_path, changed) = unset_theme_in_config(Some(&path)).expect("unset should succeed");

    assert_eq!(written_path, path);
    assert!(!changed);
    assert!(!path.exists());

    fs::remove_dir(dir).ok();
}

#[test]
fn init_config_creates_theme_config() {
    let dir = std::env::temp_dir().join(format!("veila-init-create-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");

    let written_path = init_config(Some(&path), "samurai", false).expect("config should init");

    assert_eq!(written_path, path);
    let raw = fs::read_to_string(&written_path).expect("written config");
    assert!(raw.contains("theme = \"samurai\""));

    let loaded = AppConfig::load(Some(&written_path)).expect("config should load");
    assert_eq!(loaded.config.visuals.clock_font_family(), Some("Japanola"));

    fs::remove_file(written_path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn init_config_refuses_to_replace_existing_config_without_force() {
    let dir = std::env::temp_dir().join(format!("veila-init-refuse-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    fs::write(&path, "theme = \"default\"\n").expect("config file");

    let error = init_config(Some(&path), "samurai", false).expect_err("init should refuse");

    assert!(error.to_string().contains("config already exists"));
    let raw = fs::read_to_string(&path).expect("config should remain");
    assert!(raw.contains("theme = \"default\""));

    fs::remove_file(path).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn init_config_replaces_existing_config_with_force() {
    let dir = std::env::temp_dir().join(format!("veila-init-force-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        r#"
            theme = "default"

            [visuals.input]
            width = 420
        "#,
    )
    .expect("config file");

    init_config(Some(&path), "seceda", true).expect("config should init");

    let raw = fs::read_to_string(&path).expect("written config");
    assert!(raw.contains("theme = \"seceda\""));
    assert!(!raw.contains("width = 420"));

    fs::remove_file(path).ok();
    fs::remove_dir(dir).ok();
}
