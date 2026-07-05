mod loader;
mod slideshow;
mod transition;

pub(crate) use loader::BackgroundEvent;
pub(crate) use slideshow::BackgroundSlideshow;
pub(crate) use transition::SlideshowTransition;

use std::{sync::Arc, time::Instant};

use loader::{spawn_avatar_loader, spawn_loader, spawn_preloader};
use smithay_client_toolkit::reexports::client::QueueHandle;
use veila_renderer::{FrameSize, SoftwareBuffer};

use veila_common::config::wallpaper_paths_equal;

use crate::state::CurtainApp;

const SLIDESHOW_TRANSITION_POLL_INTERVAL_MS: u64 = 80;

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
                    if self.try_apply_slideshow_transition_buffers(
                        &path,
                        &buffers,
                        queue_handle,
                    ) {
                        continue;
                    }

                    tracing::info!(
                        elapsed_ms,
                        rendered_sizes = buffers.len(),
                        cache_hit,
                        "loaded deferred curtain background image"
                    );
                    let mut changed = false;
                    for index in 0..self.lock_surfaces.len() {
                        if self
                            .background_path_for_surface(index)
                            .is_none_or(|selected_path| {
                                !wallpaper_paths_equal(selected_path, path.as_path())
                            })
                        {
                            continue;
                        }

                        let Some(surface_size) = self.lock_surfaces[index].size else {
                            self.lock_surfaces[index].background = None;
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

                        let skip_update = self.surface_wallpaper_is_current(index, path.as_path(), size);
                        if skip_update {
                            continue;
                        }

                        let surface = &mut self.lock_surfaces[index];
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
                BackgroundEvent::AvatarReady {
                    path,
                    asset,
                    elapsed_ms,
                } => {
                    if self.avatar_path != path {
                        continue;
                    }
                    tracing::info!(elapsed_ms, "loaded deferred curtain avatar image");
                    self.ui_shell.set_avatar(asset);
                    self.avatar_load_started = false;
                    self.render_all_surfaces(queue_handle);
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

    pub(crate) fn maybe_start_avatar_load(&mut self) {
        if self.avatar_load_started || !self.ready_notified {
            return;
        }

        self.avatar_load_started = true;
        spawn_avatar_loader(self.avatar_path.clone(), self.background_sender.clone());
    }

    pub(crate) fn maybe_start_background_render(&mut self) {
        if self.background_render_started {
            return;
        }

        let specs = self.background_render_specs();
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

    fn background_render_specs(&self) -> Vec<BackgroundRenderSpec> {
        let mut specs: Vec<BackgroundRenderSpec> = Vec::new();

        for (index, surface) in self.lock_surfaces.iter().enumerate() {
            let Some(path) = self
                .background_path_for_surface(index)
                .map(ToOwned::to_owned)
            else {
                continue;
            };
            let Some(size) = surface
                .size
                .map(|size| size.buffer)
                .or_else(|| self.estimated_surface_size(index).map(|size| size.buffer))
            else {
                continue;
            };

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

        specs
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

        let transition_duration = self
            .slideshow
            .as_ref()
            .map(BackgroundSlideshow::transition_duration)
            .unwrap_or_default();

        if transition_duration.is_zero() {
            self.background_path = Some(path);
            self.reset_background_source_state();
            self.render_all_surfaces(queue_handle);
            self.maybe_start_background_render();
            return;
        }

        self.begin_slideshow_transition(path, transition_duration, queue_handle);
    }

    pub(crate) fn advance_slideshow_transition(&mut self, queue_handle: &QueueHandle<Self>) {
        let now = Instant::now();
        let Some(transition) = self.slideshow_transition.as_ref() else {
            return;
        };

        if transition.is_loading() {
            return;
        }

        if transition.is_complete(now) {
            self.finalize_slideshow_transition(queue_handle);
            return;
        }

        self.render_all_surfaces(queue_handle);
    }

    pub(crate) fn slideshow_transition_poll_interval(&self) -> Option<std::time::Duration> {
        self.slideshow_transition
            .as_ref()
            .filter(|transition| transition.is_animating())
            .map(|_| std::time::Duration::from_millis(SLIDESHOW_TRANSITION_POLL_INTERVAL_MS))
    }

    pub(crate) fn slideshow_crossfade_for_surface(
        &self,
        index: usize,
    ) -> Option<(Arc<SoftwareBuffer>, Arc<SoftwareBuffer>, u8)> {
        let transition = self.slideshow_transition.as_ref()?;
        let progress = transition.fade_progress(Instant::now())?;
        let from = transition.from_buffers.get(index)?.clone()?;
        let to = transition.to_buffers.get(index)?.clone()?;
        Some((from, to, progress))
    }

    fn begin_slideshow_transition(
        &mut self,
        path: std::path::PathBuf,
        duration: std::time::Duration,
        queue_handle: &QueueHandle<Self>,
    ) {
        let from_buffers: Vec<_> = (0..self.lock_surfaces.len())
            .map(|index| self.snapshot_slideshow_surface_buffer(index))
            .collect();

        tracing::debug!(
            path = %path.display(),
            duration_ms = duration.as_millis(),
            "beginning slideshow crossfade"
        );

        self.slideshow_transition = Some(SlideshowTransition::new(
            from_buffers,
            path.clone(),
            duration,
        ));
        self.background_path = Some(path);
        self.background_render_started = false;
        self.maybe_start_background_render();
        self.render_all_surfaces(queue_handle);
    }

    fn snapshot_slideshow_surface_buffer(&self, index: usize) -> Option<Arc<SoftwareBuffer>> {
        let surface = &self.lock_surfaces[index];
        if self.ui_visible_on_surface(index) {
            return surface.scene_base.clone().or_else(|| {
                surface
                    .background
                    .as_ref()
                    .map(|buffer| Arc::new(buffer.clone()))
            });
        }

        surface
            .background
            .as_ref()
            .map(|buffer| Arc::new(buffer.clone()))
            .or_else(|| surface.scene_base.clone())
    }

    fn surface_wallpaper_is_current(
        &self,
        index: usize,
        path: &std::path::Path,
        size: FrameSize,
    ) -> bool {
        let surface = &self.lock_surfaces[index];
        let wallpaper_path = surface
            .background_path
            .as_deref()
            .or(self.background_path.as_deref());
        let path_matches =
            wallpaper_path.is_some_and(|current| wallpaper_paths_equal(current, path));

        if !path_matches {
            return false;
        }

        if let Some(scene_base) = &surface.scene_base {
            return scene_base.size() == size;
        }

        surface
            .background
            .as_ref()
            .is_some_and(|background| background.size() == size)
    }

    fn try_apply_slideshow_transition_buffers(
        &mut self,
        path: &std::path::Path,
        buffers: &[(FrameSize, SoftwareBuffer)],
        queue_handle: &QueueHandle<Self>,
    ) -> bool {
        let transition_active = self
            .slideshow_transition
            .as_ref()
            .is_some_and(|transition| wallpaper_paths_equal(&transition.to_path, path));
        if !transition_active {
            return false;
        }

        let revision = self.ui_shell.static_scene_revision();
        let mut built_targets = Vec::new();

        for index in 0..self.lock_surfaces.len() {
            let Some(surface_size) = self.lock_surfaces[index].size else {
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

            if self
                .slideshow_transition
                .as_ref()
                .and_then(|transition| transition.to_buffers.get(index))
                .is_some_and(Option::is_some)
            {
                continue;
            }

            let target = self.build_slideshow_scene_base(index, surface_size, buffer, revision);
            built_targets.push((index, target));
        }

        let Some(transition) = self.slideshow_transition.as_mut() else {
            return true;
        };

        let mut changed = false;
        for (index, target) in built_targets {
            if transition.to_buffers[index].is_none() {
                transition.to_buffers[index] = Some(target);
                changed = true;
            }
        }

        if transition.all_targets_ready() && transition.is_loading() {
            transition.mark_animating(Instant::now());
            changed = true;
        }

        if changed {
            self.render_all_surfaces(queue_handle);
        }

        true
    }

    fn finalize_slideshow_transition(&mut self, queue_handle: &QueueHandle<Self>) {
        let Some(transition) = self.slideshow_transition.take() else {
            return;
        };

        tracing::debug!(
            path = %transition.to_path.display(),
            "finalized slideshow crossfade"
        );

        let revision = self.ui_shell.static_scene_revision();
        for (index, target) in transition.to_buffers.into_iter().enumerate() {
            let Some(target) = target else {
                continue;
            };

            let has_layers = self.ui_visible_on_surface(index) && self.ui_shell.has_visual_layers();
            let surface = &mut self.lock_surfaces[index];
            surface.scene_base = Some(target);
            surface.background = None;
            surface.background_path = Some(transition.to_path.clone());
            surface.scene_base_revision = revision;
            surface.scene_base_has_layers = has_layers;
        }

        self.background_render_started = true;
        self.render_all_surfaces(queue_handle);
    }
}

struct BackgroundRenderSpec {
    path: std::path::PathBuf,
    sizes: Vec<FrameSize>,
}
