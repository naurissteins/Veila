use super::*;

#[test]
fn set_theme_in_config_creates_missing_file() {
    let dir = std::env::temp_dir().join(format!("veila-set-theme-create-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");

    let written_path = set_theme_in_config(Some(&path), "boracay").expect("theme should set");

    assert_eq!(written_path, path);
    let raw = fs::read_to_string(&written_path).expect("written config");
    assert!(raw.contains("theme = \"boracay\""));

    let loaded = AppConfig::load(Some(&written_path)).expect("config should load");
    assert_eq!(loaded.config.visuals.clock_font_family(), Some("Nunito"));
    assert_eq!(
        loaded.config.visuals.clock_font_style(),
        Some(FontStyle::Italic)
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
            [lock]
            show_username = false

            [visuals.input]
            width = 420
        "#,
    )
    .expect("config file");

    set_theme_in_config(Some(&path), "shanghai").expect("theme should set");

    let raw = fs::read_to_string(&path).expect("written config");
    assert!(raw.contains("theme = \"shanghai\""));
    assert!(raw.contains("show_username = false"));
    assert!(raw.contains("width = 420"));

    let loaded = AppConfig::load(Some(&path)).expect("config should load");
    assert!(!loaded.config.lock.show_username);
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
            theme = "shanghai"

            [lock]
            show_username = false

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
    assert!(raw.contains("show_username = false"));
    assert!(raw.contains("width = 420"));

    let loaded = AppConfig::load(Some(&path)).expect("config should load");
    assert!(!loaded.config.lock.show_username);
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
