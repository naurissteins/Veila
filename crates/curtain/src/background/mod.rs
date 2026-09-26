mod loader;
mod slideshow;

pub(crate) use loader::BackgroundEvent;
pub(crate) use slideshow::BackgroundSlideshow;

use loader::{
    spawn_artwork_loader, spawn_avatar_loader, spawn_generated_loader, spawn_loader,
    spawn_preloader,
};
use smithay_client_toolkit::reexports::client::QueueHandle;
use std::time::{Duration, Instant};
use veila_renderer::FrameSize;

use crate::state::CurtainApp;

const ARTWORK_RETRY_INTERVAL: Duration = Duration::from_secs(2);

impl CurtainApp {
    pub(crate) fn handle_background_event(
        &mut self,
        event: BackgroundEvent,
        queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            BackgroundEvent::BuffersReady {
                path,
                buffers,
                elapsed_ms,
                cache_hit,
            } => {
                tracing::info!(
                    elapsed_ms,
                    rendered_sizes = buffers.len(),
                    cache_hit,
                    "loaded deferred curtain background image"
                );
                let revision = self.ui_shell.static_scene_revision();
                let mut changed = false;
                for index in 0..self.lock_surfaces.len() {
                    if self
                        .background_path_for_surface(index)
                        .is_none_or(|selected_path| selected_path != path.as_path())
                    {
                        continue;
                    }

                    let surface = &mut self.lock_surfaces[index];
                    let Some(surface_size) = surface.size else {
                        surface.background = None;
                        continue;
                    };

                    let size = surface_size.buffer;
                    let Some(buffer) = buffers
                        .iter()
                        .find(|(candidate, _)| *candidate == size)
                        .map(|(_, buffer)| buffer.clone())
                    else {
                        continue;
                    };

                    if cache_hit
                        && surface.background_path.as_deref() == Some(path.as_path())
                        && surface.scene_base_revision == revision
                        && surface
                            .scene_base
                            .as_ref()
                            .is_some_and(|scene_base| scene_base.size() == size)
                    {
                        tracing::debug!(
                            path = %path.display(),
                            width = size.width,
                            height = size.height,
                            output_cached = true,
                            "skipping redundant deferred background rerender"
                        );
                        continue;
                    }

                    surface.background = Some(buffer);
                    surface.background_path = Some(path.clone());
                    surface.scene_base = None;
                    surface.scene_base_revision = 0;
                    surface.scene_base_has_layers = false;
                    changed = true;
                }
                if changed {
                    self.render_all_surfaces(queue_handle);
                }
            }
            BackgroundEvent::GeneratedBuffersReady {
                generated,
                treatment,
                buffers,
                elapsed_ms,
            } => {
                if self.background_generated != Some(generated)
                    || self.background_treatment != treatment
                {
                    return;
                }
                tracing::info!(
                    elapsed_ms,
                    rendered_sizes = buffers.len(),
                    "loaded generated curtain background"
                );
                let mut changed = false;
                for index in 0..self.lock_surfaces.len() {
                    if self.background_path_for_surface(index).is_some() {
                        continue;
                    }
                    let surface = &mut self.lock_surfaces[index];
                    let Some(size) = surface.size.map(|size| size.buffer) else {
                        continue;
                    };
                    let Some(buffer) = buffers
                        .iter()
                        .find(|(candidate, _)| *candidate == size)
                        .map(|(_, buffer)| buffer.clone())
                    else {
                        continue;
                    };
                    surface.background = Some(buffer);
                    surface.background_path = None;
                    surface.scene_base = None;
                    surface.scene_base_revision = 0;
                    surface.scene_base_has_layers = false;
                    changed = true;
                }
                if changed {
                    self.render_all_surfaces(queue_handle);
                }
            }
            BackgroundEvent::AvatarReady {
                path,
                asset,
                elapsed_ms,
            } => {
                if self.avatar_path != path {
                    return;
                }
                tracing::info!(elapsed_ms, "loaded deferred curtain avatar image");
                let changed = self.ui_shell.set_avatar(asset);
                self.avatar_load_started = false;
                self.avatar_load_needed = false;
                if changed {
                    self.render_all_surfaces(queue_handle);
                }
            }
            BackgroundEvent::ArtworkReady {
                path,
                snapshot,
                asset,
                elapsed_ms,
            } => {
                if self.artwork_in_flight.as_ref().is_none_or(
                    |(in_flight_path, in_flight_snapshot)| {
                        in_flight_path != &path || in_flight_snapshot != &snapshot
                    },
                ) {
                    return;
                }
                self.artwork_in_flight = None;
                if self.now_playing_snapshot == snapshot
                    && let Some(asset) = asset
                    && self.ui_shell.set_now_playing_artwork(&path, asset)
                {
                    tracing::debug!(elapsed_ms, "loaded deferred now playing artwork");
                    self.render_all_surfaces(queue_handle);
                }
                self.maybe_start_artwork_load();
            }
            BackgroundEvent::Failed { error, elapsed_ms } => {
                tracing::warn!(
                    elapsed_ms,
                    "failed to load deferred curtain background image: {error}"
                );
            }
        }
    }

    pub(crate) fn maybe_start_avatar_load(&mut self) {
        if !self.avatar_load_needed || self.avatar_load_started || !self.ready_notified {
            return;
        }

        self.avatar_load_started = true;
        spawn_avatar_loader(self.avatar_path.clone(), self.background_sender.clone());
    }

    pub(crate) fn maybe_start_artwork_load(&mut self) {
        if !self.session_locked || self.artwork_in_flight.is_some() {
            return;
        }
        let Some(path) = self
            .ui_shell
            .pending_now_playing_artwork_path()
            .map(std::path::Path::to_path_buf)
        else {
            return;
        };
        if self
            .artwork_last_attempt
            .as_ref()
            .is_some_and(|(previous, attempted_at)| {
                previous == &path && attempted_at.elapsed() < ARTWORK_RETRY_INTERVAL
            })
        {
            return;
        }
        let max_dimension = self
            .lock_surfaces
            .iter()
            .filter_map(|surface| surface.size)
            .map(|size| {
                self.ui_shell
                    .now_playing_artwork_decode_size(size.buffer, size.scale.max(1) as u32)
            })
            .max()
            .unwrap_or(160);
        let snapshot = self.now_playing_snapshot.clone();
        self.artwork_in_flight = Some((path.clone(), snapshot.clone()));
        self.artwork_last_attempt = Some((path.clone(), Instant::now()));
        spawn_artwork_loader(
            path,
            snapshot,
            max_dimension,
            self.background_sender.clone(),
        );
    }

    pub(crate) fn maybe_start_background_render(&mut self) {
        if self.background_render_started {
            return;
        }

        let Some(specs) = self.background_render_specs() else {
            return;
        };

        let generated_sizes: Vec<_> = self
            .generated_pending_sizes
            .iter()
            .copied()
            .filter(|size| {
                self.lock_surfaces
                    .iter()
                    .enumerate()
                    .any(|(index, surface)| {
                        self.background_path_for_surface(index).is_none()
                            && surface
                                .size
                                .is_some_and(|surface_size| surface_size.buffer == *size)
                    })
            })
            .collect();

        if specs.is_empty() && generated_sizes.is_empty() {
            return;
        }

        self.background_render_started = true;
        self.generated_pending_sizes.clear();
        for spec in specs {
            spawn_loader(
                spec.path,
                self.background_color,
                self.background_treatment,
                spec.sizes,
                self.background_sender.clone(),
            );
        }
        if let Some(generated) = self.background_generated
            && !generated_sizes.is_empty()
        {
            spawn_generated_loader(
                generated,
                self.background_color,
                self.background_treatment,
                generated_sizes,
                self.background_sender.clone(),
            );
        }
        self.preload_next_slideshow_background();
    }

    fn background_render_specs(&self) -> Option<Vec<BackgroundRenderSpec>> {
        let mut specs: Vec<BackgroundRenderSpec> = Vec::new();

        for (index, surface) in self.lock_surfaces.iter().enumerate() {
            let Some(path) = self
                .background_path_for_surface(index)
                .map(ToOwned::to_owned)
            else {
                continue;
            };
            let size = surface.size?.buffer;

            if let Some(spec) = specs.iter_mut().find(|spec| spec.path == path) {
                if !spec.sizes.contains(&size) {
                    spec.sizes.push(size);
                }
                continue;
            }

            specs.push(BackgroundRenderSpec {
                path,
                sizes: vec![size],
            });
        }

        Some(specs)
    }

    pub(crate) fn preload_next_slideshow_background(&self) {
        let Some(path) = self
            .slideshow
            .as_ref()
            .and_then(BackgroundSlideshow::next_preload_path)
        else {
            return;
        };

        let sizes: Vec<_> = self
            .lock_surfaces
            .iter()
            .filter_map(|surface| surface.size.map(|size| size.buffer))
            .collect();
        if sizes.is_empty() {
            return;
        }

        spawn_preloader(
            path,
            self.background_color,
            self.background_treatment,
            sizes,
        );
    }

    pub(crate) fn reset_background_source_state(&mut self) {
        self.background_render_started = false;
        self.generated_pending_sizes.clear();
        for surface in &mut self.lock_surfaces {
            surface.background_path = None;
            surface.background = None;
            surface.scene_base = None;
            surface.scene_base_revision = 0;
            surface.scene_base_has_layers = false;
        }
    }

    pub(crate) fn advance_background_slideshow(&mut self, queue_handle: &QueueHandle<Self>) {
        if self.outputs_powered_off() {
            return;
        }

        let Some(path) = self
            .slideshow
            .as_mut()
            .and_then(|slideshow| slideshow.advance(std::time::Instant::now()))
        else {
            return;
        };

        tracing::info!(path = %path.display(), "advanced lockscreen slideshow background");
        self.background_path = Some(path);
        self.reset_background_source_state();
        self.render_all_surfaces(queue_handle);
        self.maybe_start_background_render();
    }
}

struct BackgroundRenderSpec {
    path: std::path::PathBuf,
    sizes: Vec<FrameSize>,
}
