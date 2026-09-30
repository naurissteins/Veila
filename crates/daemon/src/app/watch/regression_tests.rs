use std::{
    fs::{self, File, FileTimes},
    path::PathBuf,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use veila_common::{AppConfig, LoadedConfig};

use super::{AutoReloadTrigger, AutoReloadWatcher};

struct Fixture {
    root: PathBuf,
    loaded: LoadedConfig,
    watcher: AutoReloadWatcher,
}

impl Fixture {
    fn new(config: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "veila-watch-regression-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("test root");
        let path = root.join("config.toml");
        fs::write(&path, config).expect("config");
        let loaded = LoadedConfig {
            config: AppConfig::load_from_file(&path).expect("valid config"),
            path: Some(path.clone()),
        };
        let watcher = AutoReloadWatcher::new(Some(&path), &loaded);
        Self {
            root,
            loaded,
            watcher,
        }
    }

    fn poll(&mut self) -> Option<AutoReloadTrigger> {
        self.watcher.poll(self.loaded.path.as_deref(), &self.loaded)
    }

    fn finish_debounce(&mut self) -> Option<AutoReloadTrigger> {
        self.watcher.debounce_until = Some(Instant::now());
        self.poll()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("remove fixture");
    }
}

#[test]
fn pending_reload_stays_urgent_until_the_edit_settles() {
    let mut fixture = Fixture::new("[lock]\nauto_reload_config = true\n");
    assert!(!fixture.watcher.is_pending());
    let path = fixture.loaded.path.as_ref().expect("path");
    fs::write(path, "[lock]\nauto_reload_config = false\n").expect("edit");
    assert_eq!(fixture.poll(), None);
    assert!(fixture.watcher.is_pending());
    assert_eq!(fixture.poll(), None);
    assert!(fixture.watcher.is_pending());
    assert_eq!(fixture.finish_debounce(), Some(AutoReloadTrigger::Config));
    assert!(!fixture.watcher.is_pending());
}

#[test]
fn detects_same_length_edits_with_restored_modification_time() {
    let mut fixture = Fixture::new("[visuals.clock]\nfont_size = 88\n");
    let path = fixture.loaded.path.as_ref().expect("path");
    let modified = fs::metadata(path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    fs::write(path, "[visuals.clock]\nfont_size = 94\n").expect("same length edit");
    File::open(path)
        .expect("file")
        .set_times(FileTimes::new().set_modified(modified))
        .expect("restore mtime");
    assert_eq!(fixture.poll(), None);
    assert_eq!(fixture.finish_debounce(), Some(AutoReloadTrigger::Config));
}

#[test]
fn detects_atomic_replacement_with_restored_modification_time() {
    let mut fixture = Fixture::new("[visuals.clock]\nfont_size = 88\n");
    let path = fixture.loaded.path.as_ref().expect("path");
    let modified = fs::metadata(path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    let replacement = fixture.root.join("replacement");
    fs::write(&replacement, "[visuals.clock]\nfont_size = 94\n").expect("replacement");
    File::open(&replacement)
        .expect("file")
        .set_times(FileTimes::new().set_modified(modified))
        .expect("restore mtime");
    fs::rename(replacement, path).expect("atomic replacement");
    assert_eq!(fixture.poll(), None);
    assert_eq!(fixture.finish_debounce(), Some(AutoReloadTrigger::Config));
}

#[test]
fn detects_a_previously_missing_include() {
    let mut fixture = Fixture::new("include = ['generated.toml']\n");
    fs::write(
        fixture.root.join("generated.toml"),
        "[visuals.clock]\nfont_size = 94\n",
    )
    .expect("new include");
    assert_eq!(fixture.poll(), None);
    assert_eq!(fixture.finish_debounce(), Some(AutoReloadTrigger::Include));
}

#[test]
fn disabled_auto_reload_still_detects_config_reenabling() {
    let mut fixture = Fixture::new("[lock]\nauto_reload_config = false\n");
    fs::write(
        fixture.loaded.path.as_ref().expect("path"),
        "[lock]\nauto_reload_config = true\n",
    )
    .expect("enable");
    assert_eq!(fixture.poll(), None);
    assert_eq!(fixture.finish_debounce(), Some(AutoReloadTrigger::Config));
}

#[test]
fn watches_a_new_user_theme_that_shadows_the_bundled_theme() {
    let mut fixture = Fixture::new("theme = 'default'\n");
    let themes = fixture.root.join("themes");
    fs::create_dir(&themes).expect("theme directory");
    let theme = themes.join("default.toml");
    fs::write(&theme, "[visuals.clock]\nfont_size = 88\n").expect("user theme");
    fixture.poll();
    fs::write(theme, "[visuals.clock]\nfont_size = 94\n").expect("edit user theme");
    assert_eq!(fixture.poll(), None);
    assert_eq!(fixture.finish_debounce(), Some(AutoReloadTrigger::Theme));
}
