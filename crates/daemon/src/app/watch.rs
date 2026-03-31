use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use veila_common::{LoadedConfig, active_theme_source_path, default_config_path};

const MIN_AUTO_RELOAD_DEBOUNCE_MS: u64 = 250;
const MAX_AUTO_RELOAD_DEBOUNCE_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AutoReloadTrigger {
    Config,
    Theme,
    Wallpaper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    exists: bool,
    modified: Option<SystemTime>,
    len: u64,
}

impl FileStamp {
    fn read(path: &Path) -> Self {
        match fs::metadata(path) {
            Ok(metadata) => Self {
                exists: true,
                modified: metadata.modified().ok(),
                len: metadata.len(),
            },
            Err(_) => Self {
                exists: false,
                modified: None,
                len: 0,
            },
        }
    }
}

#[derive(Debug)]
pub(super) struct AutoReloadWatcher {
    config_path: Option<PathBuf>,
    config_stamp: Option<FileStamp>,
    wallpaper_path: Option<PathBuf>,
    wallpaper_stamp: Option<FileStamp>,
    theme_path: Option<PathBuf>,
    theme_stamp: Option<FileStamp>,
    pending: Option<AutoReloadTrigger>,
    debounce_until: Option<std::time::Instant>,
}

impl AutoReloadWatcher {
    pub(super) fn new(config_path_override: Option<&Path>, loaded_config: &LoadedConfig) -> Self {
        let mut watcher = Self {
            config_path: None,
            config_stamp: None,
            wallpaper_path: None,
            wallpaper_stamp: None,
            theme_path: None,
            theme_stamp: None,
            pending: None,
            debounce_until: None,
        };
        watcher.sync_targets(config_path_override, loaded_config);
        watcher
    }

    pub(super) fn poll(
        &mut self,
        config_path_override: Option<&Path>,
        loaded_config: &LoadedConfig,
    ) -> Option<AutoReloadTrigger> {
        self.sync_targets(config_path_override, loaded_config);

        let mut changed = None;
        if let Some(path) = self.config_path.as_deref() {
            let stamp = FileStamp::read(path);
            if self.config_stamp != Some(stamp) {
                self.config_stamp = Some(stamp);
                changed = Some(AutoReloadTrigger::Config);
            }
        }

        if loaded_config.config.lock.auto_reload_config
            && let Some(path) = self.wallpaper_path.as_deref()
        {
            let stamp = FileStamp::read(path);
            if self.wallpaper_stamp != Some(stamp) {
                self.wallpaper_stamp = Some(stamp);
                changed = Some(AutoReloadTrigger::Wallpaper);
            }
        }

        if loaded_config.config.lock.auto_reload_config
            && let Some(path) = self.theme_path.as_deref()
        {
            let stamp = FileStamp::read(path);
            if self.theme_stamp != Some(stamp) {
                self.theme_stamp = Some(stamp);
                changed = Some(AutoReloadTrigger::Theme);
            }
        }

        if let Some(trigger) = changed {
            self.pending = Some(trigger);
            self.debounce_until = Some(
                std::time::Instant::now()
                    + Duration::from_millis(effective_auto_reload_debounce_ms(loaded_config)),
            );
            return None;
        }

        match (self.pending, self.debounce_until) {
            (Some(trigger), Some(deadline)) if std::time::Instant::now() >= deadline => {
                self.pending = None;
                self.debounce_until = None;
                Some(trigger)
            }
            _ => None,
        }
    }

    fn sync_targets(&mut self, config_path_override: Option<&Path>, loaded_config: &LoadedConfig) {
        let next_config_path = loaded_config
            .path
            .clone()
            .or_else(|| config_path_override.map(Path::to_path_buf))
            .or_else(default_config_path);
        sync_path(
            &mut self.config_path,
            &mut self.config_stamp,
            next_config_path.as_deref(),
        );

        let next_wallpaper_path = loaded_config.config.background.resolved_path();
        sync_path(
            &mut self.wallpaper_path,
            &mut self.wallpaper_stamp,
            next_wallpaper_path.as_deref(),
        );

        let next_theme_path = active_theme_source_path(next_config_path.as_deref()).unwrap_or(None);
        sync_path(
            &mut self.theme_path,
            &mut self.theme_stamp,
            next_theme_path.as_deref(),
        );
    }
}

pub(super) fn effective_auto_reload_debounce_ms(loaded_config: &LoadedConfig) -> u64 {
    loaded_config
        .config
        .lock
        .auto_reload_debounce_ms
        .clamp(MIN_AUTO_RELOAD_DEBOUNCE_MS, MAX_AUTO_RELOAD_DEBOUNCE_MS)
}

fn sync_path(
    current_path: &mut Option<PathBuf>,
    current_stamp: &mut Option<FileStamp>,
    next_path: Option<&Path>,
) {
    let next = next_path.map(Path::to_path_buf);
    if *current_path == next {
        return;
    }

    *current_path = next.clone();
    *current_stamp = next.as_deref().map(FileStamp::read);
}

#[cfg(test)]
mod tests {
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
        fs::write(&theme_path, b"[visuals.clock]\nsize = 14\n").expect("theme");
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

        fs::write(&theme_path, b"[visuals.clock]\nsize = 15\n").expect("theme");
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
}
