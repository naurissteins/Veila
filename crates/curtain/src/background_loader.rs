use std::{path::PathBuf, sync::mpsc::Sender, thread};

use kwylock_renderer::{ClearColor, FrameSize, SoftwareBuffer, background::BackgroundAsset};

#[derive(Debug, Clone)]
pub(crate) enum BackgroundEvent {
    Prepared {
        asset: BackgroundAsset,
        buffers: Vec<(FrameSize, SoftwareBuffer)>,
    },
    Failed(String),
}

pub(crate) fn spawn_loader(
    path: PathBuf,
    fallback: ClearColor,
    sizes: Vec<FrameSize>,
    sender: Sender<BackgroundEvent>,
) {
    thread::spawn(move || {
        let event = match load_buffers(path, fallback, sizes) {
            Ok((asset, buffers)) => BackgroundEvent::Prepared { asset, buffers },
            Err(error) => BackgroundEvent::Failed(error.to_string()),
        };

        let _ = sender.send(event);
    });
}

fn load_buffers(
    path: PathBuf,
    fallback: ClearColor,
    sizes: Vec<FrameSize>,
) -> kwylock_renderer::Result<(BackgroundAsset, Vec<(FrameSize, SoftwareBuffer)>)> {
    let asset = BackgroundAsset::load(Some(path.as_path()), fallback)?;
    let mut buffers = Vec::with_capacity(sizes.len());

    for size in sizes {
        buffers.push((size, asset.render(size)?));
    }

    Ok((asset, buffers))
}
