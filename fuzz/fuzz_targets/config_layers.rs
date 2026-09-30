#![no_main]

use std::fs;

use libfuzzer_sys::fuzz_target;
use veila_common::{AppConfig, active_include_source_paths, active_theme_source_path};

const HEADER: &[u8] = b"theme = \"fuzz\"\ninclude = [\"include.toml\", \"missing.toml\"]\n";

fuzz_target!(|data: &[u8]| {
    if data.len() > 48 * 1024 {
        return;
    }
    let mut sections = data.splitn(3, |byte| *byte == 0);
    let (Some(theme), Some(include), Some(main)) =
        (sections.next(), sections.next(), sections.next())
    else {
        return;
    };

    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    let theme_dir = dir.path().join("themes");
    if fs::create_dir(&theme_dir).is_err() {
        return;
    }
    let theme_path = theme_dir.join("fuzz.toml");
    let include_path = dir.path().join("include.toml");
    let missing_path = dir.path().join("missing.toml");
    let config_path = dir.path().join("config.toml");
    let mut config = HEADER.to_vec();
    config.extend_from_slice(main);

    // Fixed metadata confines theme and include reads to this temporary directory.
    if fs::write(&theme_path, theme).is_err()
        || fs::write(&include_path, include).is_err()
        || fs::write(&config_path, config).is_err()
    {
        return;
    }

    let Ok(Some(resolved_theme)) = active_theme_source_path(Some(&config_path)) else {
        return;
    };
    let Ok(resolved_includes) = active_include_source_paths(Some(&config_path)) else {
        return;
    };
    if resolved_theme != theme_path || resolved_includes != [include_path, missing_path] {
        return;
    }

    let _ = AppConfig::load_from_file(&config_path);
});
