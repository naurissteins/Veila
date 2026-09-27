use std::{
    path::{Path, PathBuf},
    thread,
    time::Instant,
};

use calloop::channel::Sender;
use veila_common::NowPlayingSnapshot;
use veila_renderer::{
    ClearColor, FrameSize, SoftwareBuffer,
    avatar::AvatarAsset,
    background::{
        BackgroundAsset, BackgroundTreatment, GeneratedBackground, load_cached_generated_render,
        load_cached_render, store_cached_generated_render, store_cached_render,
    },
    cover::CoverArtAsset,
};

#[derive(Debug, Clone)]
pub(crate) enum BackgroundEvent {
    BuffersReady {
        path: PathBuf,
        buffers: Vec<(FrameSize, SoftwareBuffer)>,
        elapsed_ms: u128,
        cache_hit: bool,
    },
    GeneratedBuffersReady {
        generated: GeneratedBackground,
        treatment: BackgroundTreatment,
        buffers: Vec<(FrameSize, SoftwareBuffer)>,
        elapsed_ms: u128,
    },
    AvatarReady {
        path: Option<PathBuf>,
        asset: AvatarAsset,
        elapsed_ms: u128,
    },
    ArtworkReady {
        path: PathBuf,
        snapshot: Option<NowPlayingSnapshot>,
        asset: Option<CoverArtAsset>,
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
            Ok(rendered_buffers) => {
                let asset_elapsed_ms = render_started_at.elapsed().as_millis();
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

pub(crate) fn spawn_generated_loader(
    generated: GeneratedBackground,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    sizes: Vec<FrameSize>,
    sender: Sender<BackgroundEvent>,
) {
    thread::spawn(move || {
        let started_at = Instant::now();
        let asset = match BackgroundAsset::load(None, fallback, Some(generated), treatment) {
            Ok(asset) => asset,
            Err(error) => {
                let _ = sender.send(BackgroundEvent::Failed {
                    error: error.to_string(),
                    elapsed_ms: started_at.elapsed().as_millis(),
                });
                return;
            }
        };
        let mut buffers = Vec::new();
        for size in unique_sizes(sizes) {
            let buffer = match load_cached_generated_render(generated, size, treatment) {
                Ok(Some(buffer)) => Ok(buffer),
                _ => asset.render(size).inspect(|buffer| {
                    if let Err(error) =
                        store_cached_generated_render(generated, size, treatment, buffer)
                    {
                        tracing::debug!("failed to cache generated background: {error:#}");
                    }
                }),
            };
            match buffer {
                Ok(buffer) => buffers.push((size, buffer)),
                Err(error) => {
                    let _ = sender.send(BackgroundEvent::Failed {
                        error: error.to_string(),
                        elapsed_ms: started_at.elapsed().as_millis(),
                    });
                }
            }
        }
        if !buffers.is_empty() {
            let _ = sender.send(BackgroundEvent::GeneratedBuffersReady {
                generated,
                treatment,
                buffers,
                elapsed_ms: started_at.elapsed().as_millis(),
            });
        }
    });
}

pub(crate) fn spawn_avatar_loader(path: Option<PathBuf>, sender: Sender<BackgroundEvent>) {
    thread::spawn(move || {
        let started_at = Instant::now();
        let asset = veila_ui::load_avatar(path.clone());
        let _ = sender.send(BackgroundEvent::AvatarReady {
            path,
            asset,
            elapsed_ms: started_at.elapsed().as_millis(),
        });
    });
}

pub(crate) fn spawn_artwork_loader(
    path: PathBuf,
    snapshot: Option<NowPlayingSnapshot>,
    max_dimension: u32,
    sender: Sender<BackgroundEvent>,
) {
    thread::spawn(move || {
        let started_at = Instant::now();
        let asset = match CoverArtAsset::load(&path, max_dimension) {
            Ok(asset) => Some(asset),
            Err(error) => {
                tracing::debug!(path = %path.display(), "failed to load now playing artwork: {error}");
                None
            }
        };
        let _ = sender.send(BackgroundEvent::ArtworkReady {
            path,
            snapshot,
            asset,
            elapsed_ms: started_at.elapsed().as_millis(),
        });
    });
}

pub(crate) fn spawn_preloader(
    path: PathBuf,
    fallback: ClearColor,
    treatment: BackgroundTreatment,
    sizes: Vec<FrameSize>,
) {
    thread::spawn(move || {
        let unique_sizes = unique_sizes(sizes);
        let cached_sizes: Vec<_> = load_cached_buffers(&path, treatment, &unique_sizes)
            .iter()
            .map(|(size, _)| *size)
            .collect();

        if cached_sizes.len() == unique_sizes.len() {
            return;
        }

        match load_buffers(&path, fallback, treatment, unique_sizes, &cached_sizes) {
            Ok(rendered_buffers) => {
                if !rendered_buffers.is_empty() {
                    store_cached_buffers(&path, treatment, &rendered_buffers);
                }
            }
            Err(error) => {
                tracing::debug!(
                    path = %path.display(),
                    "failed to preload slideshow wallpaper buffers: {error:#}"
                );
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
) -> veila_renderer::Result<Vec<(FrameSize, SoftwareBuffer)>> {
    let sizes: Vec<_> = sizes
        .into_iter()
        .filter(|size| !cached_sizes.contains(size))
        .collect();
    if sizes.is_empty() {
        return Ok(Vec::new());
    }
    let asset = BackgroundAsset::load(Some(path), fallback, None, treatment)?;
    let mut buffers = Vec::with_capacity(sizes.len());

    for size in sizes {
        buffers.push((size, asset.render(size)?));
    }

    Ok(buffers)
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
    fn cached_sizes_do_not_open_the_source() {
        let size = FrameSize::new(16, 9);
        let buffers = super::load_buffers(
            std::path::Path::new("/nonexistent/veila-wallpaper.png"),
            veila_renderer::ClearColor::opaque(0, 0, 0),
            veila_renderer::background::BackgroundTreatment::default(),
            vec![size],
            &[size],
        )
        .expect("complete render cache needs no source");
        assert!(buffers.is_empty());
    }

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
