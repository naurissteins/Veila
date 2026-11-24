mod artwork;
mod cache;
mod loader;
mod scheduling;
mod slideshow;
mod startup;
pub(crate) mod video;

pub(crate) use artwork::ArtworkLoad;
pub(crate) use loader::BackgroundEvent;
pub(crate) use slideshow::BackgroundSlideshow;
pub(crate) use startup::StartupWallpaperCache;

use loader::spawn_avatar_loader;
use smithay_client_toolkit::reexports::client::QueueHandle;

use crate::state::CurtainApp;

impl CurtainApp {
    pub(crate) fn handle_background_event(
        &mut self,
        event: BackgroundEvent,
        queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            BackgroundEvent::Video(update) => self.accept_video_frame(update, queue_handle),
            BackgroundEvent::FontsReady => self.accept_fonts(queue_handle),
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
                        && !surface.wallpaper_fallback
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
                    surface.wallpaper_fallback = false;
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
            BackgroundEvent::ArtworkReady => self.finish_artwork_load(queue_handle),
            BackgroundEvent::Failed { error, elapsed_ms } => {
                tracing::warn!(
                    elapsed_ms,
                    "failed to load deferred curtain background image: {error}"
                );
            }
        }
    }

    pub(crate) fn maybe_start_avatar_load(&mut self) {
        if self.ui_shell.emergency_active()
            || !self.avatar_load_needed
            || self.avatar_load_started
            || !self.ready_notified
        {
            return;
        }

        match spawn_avatar_loader(self.avatar_path.clone(), self.background_sender.clone()) {
            Ok(()) => self.avatar_load_started = true,
            Err(error) => {
                self.avatar_load_needed = false;
                tracing::warn!(%error, "failed to start avatar worker");
            }
        }
    }
}
