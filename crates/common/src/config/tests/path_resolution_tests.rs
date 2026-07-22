use std::fs;
use std::path::{Path, PathBuf};

use super::resolve_default_path;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("veila-path-{tag}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[test]
fn existing_user_config_wins_over_system_config() {
    let dir = temp_dir("user-wins");
    let user = dir.join("user.toml");
    let system = dir.join("system.toml");
    fs::write(&user, b"").expect("user config");
    fs::write(&system, b"").expect("system config");

    let resolved = resolve_default_path(Some(user.clone()), &system);
    assert_eq!(resolved, Some(user.clone()));

    fs::remove_file(user).ok();
    fs::remove_file(system).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn system_config_is_used_when_user_config_is_missing() {
    let dir = temp_dir("system-fallback");
    let user = dir.join("missing.toml");
    let system = dir.join("system.toml");
    fs::write(&system, b"").expect("system config");

    let resolved = resolve_default_path(Some(user), &system);
    assert_eq!(resolved, Some(system.clone()));

    fs::remove_file(system).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn system_config_is_used_when_user_path_cannot_be_resolved() {
    let dir = temp_dir("no-home");
    let system = dir.join("system.toml");
    fs::write(&system, b"").expect("system config");

    let resolved = resolve_default_path(None, &system);
    assert_eq!(resolved, Some(system.clone()));

    fs::remove_file(system).ok();
    fs::remove_dir(dir).ok();
}

#[test]
fn user_path_is_returned_when_neither_config_exists() {
    let dir = temp_dir("neither");
    let user = dir.join("missing.toml");
    let system = dir.join("also-missing.toml");

    let resolved = resolve_default_path(Some(user.clone()), &system);
    assert_eq!(resolved, Some(user));

    fs::remove_dir(dir).ok();
}

#[test]
fn nothing_resolves_when_user_path_is_absent_and_system_config_is_missing() {
    assert_eq!(
        resolve_default_path(None, Path::new("/nonexistent/veila/config.toml")),
        None
    );
}
