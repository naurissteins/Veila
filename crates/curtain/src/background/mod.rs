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
                    for surface in &mut self.lock_surfaces {
                        let Some((width, height)) = surface.size else {
                            surface.background = None;
                            continue;
                        };

                        let size = FrameSize::new(width, height);
                        surface.background = buffers
                            .iter()
                            .find(|(candidate, _)| *candidate == size)
                            .map(|(_, buffer)| buffer.clone());
                    }
                    self.render_all_surfaces(queue_handle);
                }
                BackgroundEvent::AssetReady { asset, elapsed_ms } => {
                    tracing::debug!(elapsed_ms, "loaded deferred curtain background asset");
                    self.background_asset = asset;
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

        let Some(path) = self.background_path.clone() else {
            return;
        };

        let Some(sizes) = self.background_sizes() else {
            return;
        };

        self.background_render_started = true;
        spawn_loader(
            path,
            self.background_color,
            sizes,
            self.background_sender.clone(),
        );
    }

    fn background_sizes(&self) -> Option<Vec<FrameSize>> {
        let mut sizes = Vec::with_capacity(self.lock_surfaces.len());

        for surface in &self.lock_surfaces {
            let (width, height) = surface.size?;
            sizes.push(FrameSize::new(width, height));
        }

        Some(sizes)
    }
}
