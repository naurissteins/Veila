mod schema;

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::Result;

use super::{
    AppConfig, MAX_IDLE_LOCK_AFTER_SECONDS, MIN_IDLE_LOCK_AFTER_SECONDS,
    active_include_source_paths, active_theme_source_path, default_path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidationReport {
    pub config_path: Option<PathBuf>,
    pub sources: Vec<ConfigValidationSource>,
    pub issues: Vec<ConfigValidationIssue>,
}

impl ConfigValidationReport {
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidationSource {
    pub kind: ConfigValidationSourceKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigValidationSourceKind {
    Config,
    Include,
    Theme,
}

impl ConfigValidationSourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Include => "include",
            Self::Theme => "theme",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidationIssue {
    pub source: ConfigValidationSourceKind,
    pub path: PathBuf,
    pub key_path: String,
    pub message: String,
}

pub(super) fn validate_config(explicit_path: Option<&Path>) -> Result<ConfigValidationReport> {
    let loaded = AppConfig::load(explicit_path)?;
    let mut report = ConfigValidationReport {
        config_path: loaded.path.clone(),
        sources: Vec::new(),
        issues: Vec::new(),
    };

    let Some(config_path) = resolved_existing_config_path(explicit_path, loaded.path.as_deref())
    else {
        return Ok(report);
    };

    validate_source(
        &mut report,
        ConfigValidationSourceKind::Config,
        &config_path,
    )?;
    validate_active_theme(&mut report, &config_path)?;
    validate_includes(&mut report, &config_path)?;
    validate_idle_range(&mut report, &loaded.config, &config_path);

    Ok(report)
}

fn validate_idle_range(report: &mut ConfigValidationReport, config: &AppConfig, path: &Path) {
    if config.idle.lock_after_in_range() {
        return;
    }

    report.issues.push(ConfigValidationIssue {
        source: ConfigValidationSourceKind::Config,
        path: path.to_path_buf(),
        key_path: "idle.lock_after_seconds".to_string(),
        message: format!(
            "`lock_after_seconds = {}` is outside {MIN_IDLE_LOCK_AFTER_SECONDS}..={MAX_IDLE_LOCK_AFTER_SECONDS}; the daemon clamps it to the nearest limit",
            config.idle.lock_after_seconds
        ),
    });
}

fn resolved_existing_config_path(
    explicit_path: Option<&Path>,
    loaded_path: Option<&Path>,
) -> Option<PathBuf> {
    loaded_path
        .map(Path::to_path_buf)
        .or_else(|| explicit_path.map(Path::to_path_buf))
        .or_else(default_path)
        .filter(|path| path.exists())
}

fn validate_active_theme(report: &mut ConfigValidationReport, config_path: &Path) -> Result<()> {
    let Some(path) = active_theme_source_path(Some(config_path))? else {
        return Ok(());
    };
    validate_source(report, ConfigValidationSourceKind::Theme, &path)
}

fn validate_includes(report: &mut ConfigValidationReport, config_path: &Path) -> Result<()> {
    for path in active_include_source_paths(Some(config_path))? {
        if path.exists() {
            validate_source(report, ConfigValidationSourceKind::Include, &path)?;
        }
    }
    Ok(())
}

fn validate_source(
    report: &mut ConfigValidationReport,
    kind: ConfigValidationSourceKind,
    path: &Path,
) -> Result<()> {
    if report
        .sources
        .iter()
        .any(|source| source.kind == kind && source.path == path)
    {
        return Ok(());
    }

    let raw = fs::read_to_string(path)?;
    let value = super::parse_toml_value(&raw)?;
    let issues = schema::unknown_key_issues(kind, path, &value);

    report.sources.push(ConfigValidationSource {
        kind,
        path: path.to_path_buf(),
    });
    report.issues.extend(issues);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::AppConfig;

    #[test]
    fn reports_unknown_nested_keys() {
        let root = temp_dir("unknown-nested");
        fs::create_dir_all(&root).expect("temp dir");
        let config_path = root.join("config.toml");
        fs::write(
            &config_path,
            r#"
[visuals.clock]
font_size = 42
bogus = true
"#,
        )
        .expect("config file");

        let report = AppConfig::validate(Some(&config_path)).expect("config should validate");

        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.issues[0].key_path, "visuals.clock.bogus");

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn reports_unknown_include_keys() {
        let root = temp_dir("unknown-include");
        fs::create_dir_all(&root).expect("temp dir");
        let config_path = root.join("config.toml");
        let include_path = root.join("colors.toml");
        fs::write(&config_path, "include = [\"colors.toml\"]\n").expect("config file");
        fs::write(
            &include_path,
            r##"
[visuals.palette]
foreground = "#ffffff"
accent = "#ff00ff"
"##,
        )
        .expect("include file");

        let report = AppConfig::validate(Some(&config_path)).expect("config should validate");

        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.issues[0].source.as_str(), "include");
        assert_eq!(report.issues[0].key_path, "visuals.palette.accent");

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn bundled_themes_have_known_keys() {
        let root = temp_dir("bundled-themes");
        fs::create_dir_all(&root).expect("temp dir");
        let config_path = root.join("config.toml");

        for theme in super::super::bundled_theme_names().expect("themes should load") {
            fs::write(&config_path, format!("theme = \"{theme}\"\n")).expect("config file");
            let report = AppConfig::validate(Some(&config_path)).expect("config should validate");

            assert!(
                report.is_valid(),
                "theme {theme} has unknown keys: {:?}",
                report.issues
            );
        }

        fs::remove_dir_all(root).ok();
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "veila-config-validation-{name}-{}",
            std::process::id()
        ))
    }
}
