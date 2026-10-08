use std::{fs, io::ErrorKind, path::PathBuf};

use super::{AppConfig, VeilaError};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("veila-explicit-{tag}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[test]
fn missing_explicit_config_preserves_the_filesystem_error() {
    let dir = temp_dir("missing");
    for path in [dir.join("missing.toml"), dir.join("absent/config.toml")] {
        let expected = fs::read_to_string(&path).expect_err("missing file");
        let VeilaError::ConfigIo(error) = AppConfig::load(Some(&path)).expect_err("explicit path")
        else {
            panic!("expected config I/O error");
        };
        assert_eq!(error.kind(), ErrorKind::NotFound);
        assert_eq!(error.raw_os_error(), expected.raw_os_error());
        assert_eq!(error.to_string(), expected.to_string());
    }
    fs::remove_dir(dir).expect("cleanup");
}

#[test]
fn directory_explicit_config_preserves_the_read_error() {
    let dir = temp_dir("directory");
    let expected = fs::read_to_string(&dir).expect_err("directory");
    let VeilaError::ConfigIo(error) = AppConfig::load(Some(&dir)).expect_err("explicit directory")
    else {
        panic!("expected config I/O error");
    };
    assert_eq!(error.kind(), expected.kind());
    assert_eq!(error.raw_os_error(), expected.raw_os_error());
    fs::remove_dir(dir).expect("cleanup");
}

#[test]
fn invalid_utf8_explicit_config_reports_io_error() {
    let dir = temp_dir("utf8");
    let path = dir.join("config.toml");
    fs::write(&path, [0xff]).expect("invalid UTF-8 file");
    assert!(matches!(
        AppConfig::load(Some(&path)),
        Err(VeilaError::ConfigIo(error)) if error.kind() == ErrorKind::InvalidData
    ));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn malformed_explicit_config_reports_parse_error() {
    let dir = temp_dir("parse");
    let path = dir.join("config.toml");
    fs::write(&path, "[background").expect("malformed config");
    assert!(matches!(
        AppConfig::load(Some(&path)),
        Err(VeilaError::Config(_))
    ));
    fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn explicit_config_keeps_its_path_and_layered_contents() {
    let dir = temp_dir("success");
    let path = dir.join("config.toml");
    fs::write(&path, "[lock]\nacquire_timeout_seconds = 17\n").expect("config");
    let loaded = AppConfig::load(Some(&path)).expect("explicit config");
    assert_eq!(loaded.path.as_deref(), Some(path.as_path()));
    assert_eq!(loaded.config.lock.acquire_timeout_seconds, 17);
    assert_eq!(
        loaded.config,
        AppConfig::load_from_file(&path).expect("layers")
    );
    fs::remove_dir_all(dir).expect("cleanup");
}
