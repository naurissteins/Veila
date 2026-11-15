use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread,
};

use veila_renderer::{
    ClearColor, FrameSize, SoftwareBuffer,
    background::{BackgroundTreatment, load_cached_render, load_cached_render_variant},
};

use crate::state::CurtainApp;

#[derive(PartialEq, Eq)]
struct Request {
    path: PathBuf,
    size: FrameSize,
    backdrop_variant: Option<String>,
}

struct Hit {
    request: Request,
    variant: Option<String>,
    buffer: SoftwareBuffer,
    remaining_outputs: usize,
}

pub(crate) struct StartupWallpaperCache {
    receiver: Receiver<Hit>,
    hits: Vec<Hit>,
}

impl StartupWallpaperCache {
    fn start(
        requests: Vec<(Request, usize)>,
        color: ClearColor,
        treatment: BackgroundTreatment,
    ) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("veila-wallpaper-cache".into())
            .spawn(move || {
                for (request, remaining_outputs) in requests {
                    let Some((variant, buffer)) = request.read(color, treatment) else {
                        continue;
                    };
                    if sender
                        .send(Hit {
                            request,
                            variant,
                            buffer,
                            remaining_outputs,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })?;
        Ok(Self {
            receiver,
            hits: Vec::new(),
        })
    }

    fn take(
        &mut self,
        path: &Path,
        size: FrameSize,
        variant: Option<&str>,
    ) -> Option<SoftwareBuffer> {
        self.hits.extend(self.receiver.try_iter());
        let index = self.hits.iter().position(|hit| {
            hit.request.path == path
                && hit.request.size == size
                && hit.variant.as_deref() == variant
        })?;
        let hit = &mut self.hits[index];
        if hit.remaining_outputs == 1 {
            return Some(self.hits.swap_remove(index).buffer);
        }
        hit.remaining_outputs -= 1;
        Some(hit.buffer.clone())
    }
}

impl Request {
    fn read(
        &self,
        color: ClearColor,
        treatment: BackgroundTreatment,
    ) -> Option<(Option<String>, SoftwareBuffer)> {
        if let Some(variant) = self.backdrop_variant.as_deref()
            && let Ok(Some(buffer)) =
                load_cached_render_variant(&self.path, color, self.size, treatment, variant)
        {
            return Some((Some(variant.to_owned()), buffer));
        }
        load_cached_render(&self.path, color, self.size, treatment)
            .ok()
            .flatten()
            .map(|buffer| (None, buffer))
    }
}

impl CurtainApp {
    pub(crate) fn maybe_read_startup_wallpapers(&mut self) {
        if self.session_locked
            || self.startup_wallpaper_cache.is_some()
            || self.ui_shell.has_visual_layers()
            || (self.background_path.is_none() && self.background_outputs.is_empty())
            || !self.lock_surfaces.iter().all(|surface| {
                surface
                    .placeholder_pool
                    .as_ref()
                    .is_some_and(veila_renderer::shm::SurfaceBufferPool::has_committed_frame)
            })
        {
            return;
        }

        let mut requests = Vec::<(Request, usize)>::new();
        for (index, surface) in self.lock_surfaces.iter().enumerate() {
            let (Some(size), Some(path)) = (surface.size, self.background_path_for_surface(index))
            else {
                continue;
            };
            let backdrop_variant = self
                .ui_visible_on_surface(index)
                .then(|| {
                    self.ui_shell
                        .backdrop_cache_variant_at_scale(size.render_scale)
                })
                .flatten();
            let request = Request {
                path: path.to_owned(),
                size: size.buffer,
                backdrop_variant,
            };
            if let Some((_, outputs)) = requests.iter_mut().find(|(key, _)| *key == request) {
                *outputs += 1;
            } else {
                requests.push((request, 1));
            }
        }

        if !requests.is_empty() {
            match StartupWallpaperCache::start(
                requests,
                self.background_color,
                self.background_treatment,
            ) {
                Ok(cache) => {
                    self.startup_wallpaper_cache = Some(cache);
                    tracing::debug!(
                        startup_elapsed_us = crate::state::elapsed_us(self.startup_started_at),
                        "started wallpaper cache reads"
                    );
                }
                Err(error) => tracing::debug!(%error, "failed to start wallpaper cache reads"),
            }
        }
    }

    pub(crate) fn read_file_wallpaper_cache(
        &mut self,
        index: usize,
        path: &Path,
        size: FrameSize,
        variant: Option<&str>,
    ) -> veila_renderer::Result<Option<SoftwareBuffer>> {
        if let Some(cache) = self.startup_wallpaper_cache.as_mut() {
            let buffer = cache.take(path, size, variant);
            if buffer.is_some() {
                tracing::debug!(
                    startup_elapsed_us = crate::state::elapsed_us(self.startup_started_at),
                    output_index = index,
                    width = size.width,
                    height = size.height,
                    backdrop = variant.is_some(),
                    "using prefetched wallpaper cache"
                );
            }
            return Ok(buffer);
        }

        let surface = &self.lock_surfaces[index];
        if surface.wallpaper_fallback && surface.background_path.as_deref() == Some(path) {
            return Ok(None);
        }
        match variant {
            Some(variant) => load_cached_render_variant(
                path,
                self.background_color,
                size,
                self.background_treatment,
                variant,
            ),
            None => {
                load_cached_render(path, self.background_color, size, self.background_treatment)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_cache_only_yields_matching_results_that_have_arrived() {
        let (sender, receiver) = mpsc::channel();
        let mut cache = StartupWallpaperCache {
            receiver,
            hits: Vec::new(),
        };
        let path = Path::new("wallpaper.png");
        let size = FrameSize::new(4, 3);
        let color = ClearColor::opaque(195, 149, 130);
        assert!(cache.take(path, size, Some("backdrop")).is_none());

        sender
            .send(Hit {
                request: Request {
                    path: path.to_owned(),
                    size,
                    backdrop_variant: Some("backdrop".into()),
                },
                variant: Some("backdrop".into()),
                buffer: SoftwareBuffer::solid(size, color).expect("fixture buffer"),
                remaining_outputs: 2,
            })
            .expect("cache still active");

        assert!(
            cache
                .take(Path::new("other.png"), size, Some("backdrop"))
                .is_none()
        );
        assert!(
            cache
                .take(path, FrameSize::new(8, 6), Some("backdrop"))
                .is_none()
        );
        assert!(cache.take(path, size, None).is_none());
        for _ in 0..2 {
            assert!(cache.take(path, size, Some("backdrop")).is_some());
        }
        assert!(cache.take(path, size, Some("backdrop")).is_none());
    }
}
