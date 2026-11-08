use super::{
    BackgroundSlideshow, cache,
    loader::{spawn_generated_loader, spawn_loader, spawn_preloader},
};
use crate::state::CurtainApp;
use smithay_client_toolkit::reexports::client::QueueHandle;
use veila_renderer::FrameSize;

impl CurtainApp {
    pub(crate) fn maybe_start_background_render(&mut self) {
        if !self.ready_notified || self.background_render_started {
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
                            && surface.size.is_some_and(|s| s.buffer == *size)
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
                self.backdrop_job(),
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
                self.backdrop_job(),
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
            self.backdrop_job(),
        );
    }

    fn backdrop_job(&self) -> cache::BackdropJob {
        cache::BackdropJob::new(
            self.ui_shell.static_backdrops(),
            self.lock_surfaces
                .iter()
                .filter(|surface| !surface.backdrop_cache_hit)
                .filter_map(|surface| surface.size.map(|size| (size.buffer, size.render_scale)))
                .collect(),
        )
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
            surface.backdrop_cache_hit = false;
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
