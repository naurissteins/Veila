use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use veila_common::{
    LoadedConfig, active_include_source_paths, active_theme_source_path, default_config_path,
};

const MIN_AUTO_RELOAD_DEBOUNCE_MS: u64 = 250;
const MAX_AUTO_RELOAD_DEBOUNCE_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AutoReloadTrigger {
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
        Self::read_inner(path, false)
    }

    fn read_with_content_hash(path: &Path) -> Self {
        Self::read_inner(path, true)
    }

    fn read_inner(path: &Path, hash_content: bool) -> Self {
        let content_hash = hash_content.then(|| file_content_hash(path)).flatten();
        match fs::metadata(path) {
            Ok(metadata) => Self {
                exists: true,
                modified: metadata.modified().ok(),
                len: metadata.len(),
                content_hash,
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

fn file_content_hash(path: &Path) -> Option<u64> {
    let bytes = fs::read(path).ok()?;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Some(hasher.finish())
}

#[derive(Debug)]
pub(super) struct AutoReloadWatcher {
    config_path: Option<PathBuf>,
    config_stamp: Option<FileStamp>,
    wallpaper_path: Option<PathBuf>,
    wallpaper_stamp: Option<FileStamp>,
    theme_path: Option<PathBuf>,
    theme_stamp: Option<FileStamp>,
    include_files: Vec<WatchedFile>,
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
            include_files: Vec::new(),
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
            let stamp = FileStamp::read_with_content_hash(path);
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

    pub(super) fn is_pending(&self) -> bool {
        self.pending.is_some()
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
            FileStamp::read_with_content_hash,
        );

        let next_wallpaper_path = loaded_config.config.background.resolved_path();
        sync_path(
            &mut self.wallpaper_path,
            &mut self.wallpaper_stamp,
            next_wallpaper_path.as_deref(),
            FileStamp::read,
        );

        let next_theme_path = active_theme_source_path(next_config_path.as_deref()).unwrap_or(None);
        sync_path(
            &mut self.theme_path,
            &mut self.theme_stamp,
            next_theme_path.as_deref(),
            FileStamp::read_with_content_hash,
        );

        let next_include_paths =
            active_include_source_paths(next_config_path.as_deref()).unwrap_or_default();
        sync_paths(&mut self.include_files, next_include_paths);
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
