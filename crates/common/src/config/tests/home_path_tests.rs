use std::path::{Path, PathBuf};

use super::super::include;
use super::AppConfig;

#[test]
fn include_and_background_expand_repeated_separators_consistently() {
    let config = AppConfig::from_toml_str(
        r#"
            include = "~//fragment.toml"
            [background]
            path = "~//fragment.toml"
            [[background.outputs]]
            name = "DP-1"
            path = "~//fragment.toml"
        "#,
    )
    .expect("config should parse");
    let expected = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("fragment.toml"))
        .unwrap_or_else(|| PathBuf::from("~//fragment.toml"));
    let value = toml::from_str("include = '~//fragment.toml'").expect("include should parse");

    assert_eq!(
        include::extract_paths(&value, None).unwrap(),
        vec![expected.clone()]
    );
    assert_eq!(config.background.resolved_path(), Some(expected.clone()));
    assert_eq!(
        config.background.resolved_path_for_output(Some("DP-1")),
        Some(expected)
    );
}

#[test]
fn include_resolution_keeps_its_config_directory_base() {
    let value = toml::from_str("include = ['relative.toml', '/absolute.toml', '~user/file.toml']")
        .expect("include should parse");
    assert_eq!(
        include::extract_paths(&value, Some(Path::new("/config"))).unwrap(),
        vec![
            PathBuf::from("/config/relative.toml"),
            PathBuf::from("/absolute.toml"),
            PathBuf::from("/config/~user/file.toml"),
        ]
    );
}
