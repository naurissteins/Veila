mod loader;
mod slideshow;

pub(crate) use loader::BackgroundEvent;
pub(crate) use slideshow::BackgroundSlideshow;

use loader::{spawn_loader, spawn_preloader};
use smithay_client_toolkit::reexports::client::QueueHandle;
use veila_renderer::FrameSize;

use crate::state::CurtainApp;

impl CurtainApp {
    pub(crate) fn drain_background_events(&mut self, queue_handle: &QueueHandle<Self>) {
        while let Ok(event) = self.background_events.try_recv() {
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
                BackgroundEvent::AssetReady {
                    path,
                    asset,
                    elapsed_ms,
                } => {
                    tracing::debug!(elapsed_ms, "loaded deferred curtain background asset");
                    if self.background_path.as_deref() == Some(path.as_path()) {
                        self.background_asset = asset;
                    }
                }
                BackgroundEvent::Failed { error, elapsed_ms } => {
                    tracing::warn!(
                        elapsed_ms,
                        "failed to load deferred curtain background image: {error}"
                    );
                }
            }
        }
    }

    pub(crate) fn maybe_start_background_render(&mut self) {
        if self.background_render_started {
            return;
        }

        let Some(specs) = self.background_render_specs() else {
            return;
        };

        if specs.is_empty() {
            return;
        }

        self.background_render_started = true;
        for spec in specs {
            spawn_loader(
                spec.path,
                self.background_color,
                self.background_treatment,
                spec.sizes,
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
