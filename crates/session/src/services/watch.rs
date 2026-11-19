use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use veila_common::LoadedConfig;
use veila_common::config::{ConfiguredTheme, config_watch_sources, default_config_path};

const MIN_AUTO_RELOAD_DEBOUNCE_MS: u64 = 250;
const MAX_AUTO_RELOAD_DEBOUNCE_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoReloadTrigger {
    Config,
    Include,
    Theme,
    Wallpaper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    exists: bool,
    modified: Option<SystemTime>,
    len: u64,
    content_hash: Option<u64>,
}

impl FileStamp {
    fn read(path: &Path) -> Self {
        Self::with_content(path, None)
    }

    fn read_with_content_hash(path: &Path) -> Self {
        Self::with_content(path, fs::read(path).ok().as_deref())
    }

    fn with_content(path: &Path, content: Option<&[u8]>) -> Self {
        match fs::metadata(path) {
            Ok(metadata) => Self {
                exists: true,
                modified: metadata.modified().ok(),
                len: metadata.len(),
                content_hash: content.map(file_content_hash),
            },
            Err(_) => Self {
                exists: false,
                modified: None,
                len: 0,
                content_hash: None,
            },
        }
    }
}

fn file_content_hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug)]
pub struct AutoReloadWatcher {
    config_path: Option<PathBuf>,
    config_stamp: Option<FileStamp>,
    wallpaper_path: Option<PathBuf>,
    wallpaper_stamp: Option<FileStamp>,
    theme: Option<ConfiguredTheme>,
    theme_path: Option<PathBuf>,
    theme_stamp: Option<FileStamp>,
    include_files: Vec<WatchedFile>,
    pending: Option<AutoReloadTrigger>,
    debounce_until: Option<std::time::Instant>,
}

impl AutoReloadWatcher {
    pub fn new(config_path_override: Option<&Path>, loaded_config: &LoadedConfig) -> Self {
        let mut watcher = Self {
            config_path: None,
            config_stamp: None,
            wallpaper_path: None,
            wallpaper_stamp: None,
            theme: None,
            theme_path: None,
            theme_stamp: None,
            include_files: Vec::new(),
            pending: None,
            debounce_until: None,
        };
        watcher.sync_targets(config_path_override, loaded_config);
        watcher
    }

    pub fn poll(
        &mut self,
        config_path_override: Option<&Path>,
        loaded_config: &LoadedConfig,
    ) -> Option<AutoReloadTrigger> {
        let mut changed = self
            .sync_targets(config_path_override, loaded_config)
            .then_some(AutoReloadTrigger::Config);

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
            let stamp = FileStamp::read_with_content_hash(path);
            if self.theme_stamp != Some(stamp) {
                self.theme_stamp = Some(stamp);
                changed = Some(AutoReloadTrigger::Theme);
            }
        }

        if loaded_config.config.lock.auto_reload_config {
            for include_file in &mut self.include_files {
                let stamp = FileStamp::read_with_content_hash(&include_file.path);
                if include_file.stamp != stamp {
                    include_file.stamp = stamp;
                    changed = Some(AutoReloadTrigger::Include);
                }
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

    fn sync_targets(
        &mut self,
        config_path_override: Option<&Path>,
        loaded_config: &LoadedConfig,
    ) -> bool {
        let next_config_path = loaded_config
            .path
            .clone()
            .or_else(|| config_path_override.map(Path::to_path_buf))
            .or_else(default_config_path);
        let config_changed = self.sync_config(next_config_path);

        let next_wallpaper_path = loaded_config.config.background.resolved_path();
        sync_path(
            &mut self.wallpaper_path,
            &mut self.wallpaper_stamp,
            next_wallpaper_path.as_deref(),
            FileStamp::read,
        );

        // a local theme can appear or disappear without a config edit
        let config_dir = self.config_path.as_deref().and_then(Path::parent);
        let next_theme_path = self
            .theme
            .as_ref()
            .and_then(|theme| theme.source_path(config_dir));
        sync_path(
            &mut self.theme_path,
            &mut self.theme_stamp,
            next_theme_path.as_deref(),
            FileStamp::read_with_content_hash,
        );

        config_changed
    }

    fn sync_config(&mut self, next_path: Option<PathBuf>) -> bool {
        let content = next_path.as_deref().and_then(|path| fs::read(path).ok());
        let stamp = next_path
            .as_deref()
            .map(|path| FileStamp::with_content(path, content.as_deref()));
        let same_path = self.config_path == next_path;
        if same_path && self.config_stamp == stamp {
            return false;
        }

        let config_dir = next_path.as_deref().and_then(Path::parent);
        let (theme, includes) = content
            .as_deref()
            .filter(|_| stamp.is_some_and(|stamp| stamp.exists))
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .map(|input| config_watch_sources(input, config_dir))
            .unwrap_or_default();
        self.theme = theme;
        sync_paths(&mut self.include_files, includes);
        self.config_path = next_path;
        self.config_stamp = stamp;
        same_path
    }
}

pub fn effective_auto_reload_debounce_ms(loaded_config: &LoadedConfig) -> u64 {
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
    read_stamp: fn(&Path) -> FileStamp,
) {
    let next = next_path.map(Path::to_path_buf);
    if *current_path == next {
        return;
    }

    *current_path = next.clone();
    *current_stamp = next.as_deref().map(read_stamp);
}

fn sync_paths(current_files: &mut Vec<WatchedFile>, next_paths: Vec<PathBuf>) {
    if current_files.len() == next_paths.len()
        && current_files
            .iter()
            .map(|file| file.path.as_path())
            .eq(next_paths.iter().map(PathBuf::as_path))
    {
        return;
    }

    *current_files = next_paths
        .into_iter()
        .map(|path| {
            let stamp = FileStamp::read_with_content_hash(&path);
            WatchedFile { path, stamp }
        })
        .collect();
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WatchedFile {
    path: PathBuf,
    stamp: FileStamp,
}

#[cfg(test)]
mod regression_tests;
#[cfg(test)]
mod tests;
