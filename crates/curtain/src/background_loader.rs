use std::{path::PathBuf, sync::mpsc::Sender, thread};

use kwylock_renderer::{ClearColor, background::BackgroundAsset};

#[derive(Debug, Clone)]
pub(crate) enum BackgroundEvent {
    Loaded(BackgroundAsset),
    Failed(String),
}

pub(crate) fn spawn_loader(path: PathBuf, fallback: ClearColor, sender: Sender<BackgroundEvent>) {
    thread::spawn(move || {
        let event = match BackgroundAsset::load(Some(path.as_path()), fallback) {
            Ok(asset) => BackgroundEvent::Loaded(asset),
            Err(error) => BackgroundEvent::Failed(error.to_string()),
        };

        let _ = sender.send(event);
    });
}
