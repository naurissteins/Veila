use std::{
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    thread,
    time::Instant,
};

use veila_renderer::{
    ClearColor, FrameSize, SoftwareBuffer,
    background::{BackgroundAsset, BackgroundTreatment, load_cached_render, store_cached_render},
};

#[derive(Debug, Clone)]
pub(crate) enum BackgroundEvent {
    BuffersReady {
        path: PathBuf,
        buffers: Vec<(FrameSize, SoftwareBuffer)>,
        elapsed_ms: u128,
        cache_hit: bool,
    },
    AssetReady {
        path: PathBuf,
        asset: BackgroundAsset,
        elapsed_ms: u128,
    },
    Failed {
        error: String,
        elapsed_ms: u128,
    },
}

pub(crate) fn spawn_loader(
    path: PathBuf,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    sizes: Vec<FrameSize>,
    sender: Sender<BackgroundEvent>,
) {
    thread::spawn(move || {
        let unique_sizes = unique_sizes(sizes);
        let cached_started_at = Instant::now();
        let cached_buffers = load_cached_buffers(&path, treatment, &unique_sizes);
        let cached_sizes: Vec<_> = cached_buffers.iter().map(|(size, _)| *size).collect();

        if !cached_buffers.is_empty() {
            let _ = sender.send(BackgroundEvent::BuffersReady {
                path: path.clone(),
                buffers: cached_buffers,
                elapsed_ms: cached_started_at.elapsed().as_millis(),
                cache_hit: true,
            });
        }

        let render_started_at = Instant::now();
        match load_buffers(&path, fallback, treatment, unique_sizes, &cached_sizes) {
            Ok((asset, rendered_buffers)) => {
                let asset_elapsed_ms = render_started_at.elapsed().as_millis();
                let _ = sender.send(BackgroundEvent::AssetReady {
                    path: path.clone(),
                    asset,
                    elapsed_ms: asset_elapsed_ms,
                });
                if rendered_buffers.is_empty() {
                    return;
                }

                let _ = sender.send(BackgroundEvent::BuffersReady {
                    path: path.clone(),
                    buffers: rendered_buffers.clone(),
                    elapsed_ms: asset_elapsed_ms,
                    cache_hit: false,
                });
                store_cached_buffers(&path, treatment, &rendered_buffers);
            }
            Err(error) => {
                let _ = sender.send(BackgroundEvent::Failed {
                    error: error.to_string(),
                    elapsed_ms: render_started_at.elapsed().as_millis(),
                });
            }
        }
    });
}

fn load_buffers(
    path: &Path,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    sizes: Vec<FrameSize>,
    cached_sizes: &[FrameSize],
) -> veila_renderer::Result<(BackgroundAsset, Vec<(FrameSize, SoftwareBuffer)>)> {
    let asset = BackgroundAsset::load(Some(path), fallback, None, treatment)?;
    let mut buffers = Vec::with_capacity(sizes.len());

    for size in sizes {
        if cached_sizes.contains(&size) {
            continue;
        }
        buffers.push((size, asset.render(size)?));
    }

    Ok((asset, buffers))
}

fn load_cached_buffers(
    path: &Path,
    treatment: BackgroundTreatment,
    sizes: &[FrameSize],
) -> Vec<(FrameSize, SoftwareBuffer)> {
    let mut buffers = Vec::with_capacity(sizes.len());

    for size in sizes {
        match load_cached_render(path, *size, treatment) {
            Ok(Some(buffer)) => buffers.push((*size, buffer)),
            Ok(None) => {}
            Err(error) => {
                tracing::debug!("failed to read cached wallpaper buffer: {error:#}");
            }
        }
    }

    buffers
}

fn store_cached_buffers(
    path: &Path,
    treatment: BackgroundTreatment,
    buffers: &[(FrameSize, SoftwareBuffer)],
) {
    for (size, buffer) in buffers {
        if let Err(error) = store_cached_render(path, *size, treatment, buffer) {
            tracing::debug!("failed to store cached wallpaper buffer: {error:#}");
        }
    }
}

fn unique_sizes(sizes: Vec<FrameSize>) -> Vec<FrameSize> {
    let mut unique = Vec::with_capacity(sizes.len());

    for size in sizes {
        if !unique.contains(&size) {
            unique.push(size);
        }
    }

    unique
}

#[cfg(test)]
mod tests {
    use veila_renderer::FrameSize;

    use super::unique_sizes;

    #[test]
    fn deduplicates_matching_output_sizes() {
        let sizes = unique_sizes(vec![
            FrameSize::new(1920, 1080),
            FrameSize::new(2560, 1440),
            FrameSize::new(1920, 1080),
        ]);

        assert_eq!(
            sizes,
            vec![FrameSize::new(1920, 1080), FrameSize::new(2560, 1440)]
        );
    }
}
