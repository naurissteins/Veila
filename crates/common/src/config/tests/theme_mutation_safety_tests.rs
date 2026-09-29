use super::*;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

#[test]
fn set_theme_preserves_comments_order_and_mode() {
    let dir = std::env::temp_dir().join(format!("veila-theme-layout-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    let original = "# header\ntheme = \"default\" # chosen theme\n\n[visuals.input]\n# width note\nwidth = 420\n";
    fs::write(&path, original).expect("config");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("mode");
    let original_inode = fs::metadata(&path).expect("metadata").ino();

    set_theme_in_config(Some(&path), "normandy").expect("set theme");

    let updated = fs::read_to_string(&path).expect("updated config");
    assert!(updated.contains("# header"));
    assert!(updated.contains("theme = \"normandy\" # chosen theme"));
    assert!(updated.contains("[visuals.input]\n# width note\nwidth = 420"));
    assert_eq!(
        fs::metadata(&path).expect("metadata").permissions().mode() & 0o777,
        0o640
    );
    assert_ne!(fs::metadata(&path).expect("metadata").ino(), original_inode);
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn unset_theme_preserves_unrelated_comments() {
    let dir = std::env::temp_dir().join(format!("veila-theme-unset-layout-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        "# header\ntheme = \"default\"\n\n[lock]\n# retry note\nauth_backoff_base_ms = 900\n",
    )
    .expect("config");

    unset_theme_in_config(Some(&path)).expect("unset theme");

    let updated = fs::read_to_string(&path).expect("updated config");
    assert!(updated.contains("# header"));
    assert!(updated.contains("[lock]\n# retry note\nauth_backoff_base_ms = 900"));
    assert!(!updated.contains("theme ="));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn setting_then_unsetting_theme_keeps_leading_comments() {
    let dir = std::env::temp_dir().join(format!("veila-theme-chain-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("config.toml");
    fs::write(
        &path,
        "# owner note\ntheme = \"default\" # preset\n\n[lock]\nauth_backoff_base_ms = 900\n",
    )
    .expect("config");

    set_theme_in_config(Some(&path), "normandy").expect("set theme");
    unset_theme_in_config(Some(&path)).expect("unset theme");

    assert_eq!(
        fs::read_to_string(&path).expect("updated config"),
        "# owner note\n\n[lock]\nauth_backoff_base_ms = 900\n"
    );
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn theme_edit_preserves_a_writable_config_symlink() {
    let dir = std::env::temp_dir().join(format!("veila-theme-symlink-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let target = dir.join("managed.toml");
    let path = dir.join("config.toml");
    fs::write(&target, "theme = \"default\"\n").expect("config");
    symlink(&target, &path).expect("symlink");

    set_theme_in_config(Some(&path), "normandy").expect("set theme");

    assert!(
        fs::symlink_metadata(&path)
            .expect("symlink metadata")
            .file_type()
            .is_symlink()
    );
    assert!(
        fs::read_to_string(&target)
            .expect("target")
            .contains("theme = \"normandy\"")
    );
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn theme_edit_rejects_read_only_config_symlink() {
    let dir = std::env::temp_dir().join(format!("veila-theme-readonly-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    let target = dir.join("managed.toml");
    let path = dir.join("config.toml");
    fs::write(&target, "theme = \"default\"\n").expect("config");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o444)).expect("read only");
    symlink(&target, &path).expect("symlink");

    let error = set_theme_in_config(Some(&path), "normandy").expect_err("read only");

    assert!(error.to_string().contains("managed declaratively"));
    assert!(
        fs::symlink_metadata(&path)
            .expect("symlink metadata")
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_to_string(&target).expect("target"),
        "theme = \"default\"\n"
    );
    fs::remove_dir_all(dir).expect("cleanup");
}
