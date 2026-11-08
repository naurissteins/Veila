use crate::services::watch::{AutoReloadTrigger, AutoReloadWatcher};
use std::{path::PathBuf, time::Duration};
use tokio::sync::{mpsc, watch};
use veila_common::{AppConfig, IdleConfig, LoadedConfig};

pub(super) async fn run(
    path: Option<PathBuf>,
    settings: watch::Sender<IdleConfig>,
    mut reload: mpsc::Receiver<()>,
) {
    // Filesystem work stays off the idle/sleep event loop.
    let initial_path = path.clone();
    let Ok((mut loaded, mut watcher)) = tokio::task::spawn_blocking(move || {
        let loaded = initial(initial_path.clone());
        let watcher = AutoReloadWatcher::new(initial_path.as_deref(), &loaded);
        (loaded, watcher)
    })
    .await
    else {
        return;
    };
    settings.send_replace(loaded.config.idle.clone());
    loop {
        let manual = tokio::select! {
            () = settings.closed() => return,
            message = reload.recv() => { if message.is_none() { return; } true },
            () = tokio::time::sleep(Duration::from_secs(1)) => false,
        };
        let path = path.clone();
        let result = tokio::task::spawn_blocking(move || {
            let trigger = watcher.poll(path.as_deref(), &loaded);
            if manual || trigger.is_some() {
                match AppConfig::load(path.as_deref()) {
                    Ok(next) if manual || loaded.config.lock.auto_reload_config ||
                        (trigger == Some(AutoReloadTrigger::Config) && next.config.lock.auto_reload_config) => {
                        loaded = next;
                        tracing::info!(manual, "idle watcher config reloaded");
                    }
                    Ok(_) => {},
                    Err(error) => tracing::warn!(%error, "idle watcher reload rejected; retaining current config"),
                }
            }
            (watcher, loaded)
        }).await;
        match result {
            Ok((next_watcher, next_loaded)) => {
                watcher = next_watcher;
                loaded = next_loaded;
            }
            Err(error) => {
                tracing::error!(%error, "idle config worker stopped");
                return;
            }
        }
        settings.send_if_modified(|current| {
            if *current == loaded.config.idle {
                false
            } else {
                *current = loaded.config.idle.clone();
                true
            }
        });
    }
}

fn initial(path: Option<PathBuf>) -> LoadedConfig {
    AppConfig::load(path.as_deref()).unwrap_or_else(|error| {
        tracing::warn!(%error, "idle watcher using safe initial defaults");
        LoadedConfig {
            path,
            config: AppConfig::default(),
        }
    })
}
