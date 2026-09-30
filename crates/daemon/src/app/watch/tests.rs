use std::{fs, time::UNIX_EPOCH};

use veila_common::AppConfig;

use super::{AutoReloadTrigger, AutoReloadWatcher, effective_auto_reload_debounce_ms};

#[test]
fn triggers_on_config_change_after_debounce() {
    let unique = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("veila-auto-reload-watch-{unique}"));
    fs::create_dir_all(&root).expect("dir");
    let config_path = root.join("config.toml");
    fs::write(&config_path, b"[lock]\nauto_reload_config = true\n").expect("config");

    let loaded = veila_common::LoadedConfig {
        path: Some(config_path.clone()),
        config: AppConfig::load_from_file(&config_path).expect("load"),
    };
    let mut watcher = AutoReloadWatcher::new(Some(&config_path), &loaded);

    fs::write(&config_path, b"[lock]\nauto_reload_config = false\n").expect("config");
    assert_eq!(watcher.poll(Some(&config_path), &loaded), None);
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_eq!(
        watcher.poll(Some(&config_path), &loaded),
        Some(AutoReloadTrigger::Config)
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn clamps_auto_reload_debounce_to_supported_range() {
    let low = veila_common::LoadedConfig {
        path: None,
        config: AppConfig::from_toml_str(
            r#"
                [lock]
                auto_reload_debounce_ms = 100
            "#,
        )
        .expect("low config"),
    };
    let high = veila_common::LoadedConfig {
        path: None,
        config: AppConfig::from_toml_str(
            r#"
                [lock]
                auto_reload_debounce_ms = 8000
            "#,
        )
        .expect("high config"),
    };

    assert_eq!(effective_auto_reload_debounce_ms(&low), 250);
    assert_eq!(effective_auto_reload_debounce_ms(&high), 5_000);
}

#[test]
fn triggers_on_theme_change_after_debounce() {
    let unique = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("veila-auto-reload-theme-{unique}"));
    let themes_dir = root.join("themes");
    fs::create_dir_all(&themes_dir).expect("dir");
    let config_path = root.join("config.toml");
    let theme_path = themes_dir.join("custom.toml");
    fs::write(&theme_path, b"[visuals.clock]\nfont_size = 88\n").expect("theme");
    fs::write(
        &config_path,
        b"theme = \"custom\"\n\n[lock]\nauto_reload_config = true\n",
    )
    .expect("config");

    let loaded = veila_common::LoadedConfig {
        path: Some(config_path.clone()),
        config: AppConfig::load_from_file(&config_path).expect("load"),
    };
    let mut watcher = AutoReloadWatcher::new(Some(&config_path), &loaded);

    fs::write(&theme_path, b"[visuals.clock]\nfont_size = 94\n").expect("theme");
    assert_eq!(watcher.poll(Some(&config_path), &loaded), None);
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_eq!(
        watcher.poll(Some(&config_path), &loaded),
        Some(AutoReloadTrigger::Theme)
    );

    let _ = fs::remove_file(theme_path);
    let _ = fs::remove_dir(themes_dir);
    let _ = fs::remove_file(config_path);
    let _ = fs::remove_dir(root);
}

#[test]
fn triggers_on_include_change_after_debounce() {
    let unique = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("veila-auto-reload-include-{unique}"));
    fs::create_dir_all(&root).expect("dir");
    let config_path = root.join("config.toml");
    let include_path = root.join("matugen.toml");
    fs::write(&include_path, b"[visuals.clock]\nfont_size = 88\n").expect("include");
    fs::write(
        &config_path,
        b"include = [\"matugen.toml\"]\n\n[lock]\nauto_reload_config = true\n",
    )
    .expect("config");

    let loaded = veila_common::LoadedConfig {
        path: Some(config_path.clone()),
        config: AppConfig::load_from_file(&config_path).expect("load"),
    };
    let mut watcher = AutoReloadWatcher::new(Some(&config_path), &loaded);

    fs::write(&include_path, b"[visuals.clock]\nfont_size = 94\n").expect("include");
    assert_eq!(watcher.poll(Some(&config_path), &loaded), None);
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_eq!(
        watcher.poll(Some(&config_path), &loaded),
        Some(AutoReloadTrigger::Include)
    );

    let _ = fs::remove_file(include_path);
    let _ = fs::remove_file(config_path);
    let _ = fs::remove_dir(root);
}
