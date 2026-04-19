mod loader;

pub(crate) use loader::BackgroundEvent;

use loader::spawn_loader;
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
                    for index in 0..self.lock_surfaces.len() {
                        if self
                            .background_path_for_surface(index)
                            .is_none_or(|selected_path| selected_path != path.as_path())
                        {
                            continue;
                        }

                        let surface = &mut self.lock_surfaces[index];
                        let Some((width, height)) = surface.size else {
                            surface.background = None;
                            continue;
                        };

                        let size = FrameSize::new(width, height);
                        let Some(buffer) = buffers
                            .iter()
                            .find(|(candidate, _)| *candidate == size)
                            .map(|(_, buffer)| buffer.clone())
                        else {
                            continue;
                        };

                        surface.background = Some(buffer);
                        surface.background_path = Some(path.clone());
                        surface.scene_base = None;
                        surface.scene_base_revision = 0;
                    }
                    self.render_all_surfaces(queue_handle);
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
            let (width, height) = surface.size?;
            let size = FrameSize::new(width, height);

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
}

struct BackgroundRenderSpec {
    path: std::path::PathBuf,
    sizes: Vec<FrameSize>,
}
