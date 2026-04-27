use anyhow::{Result, anyhow};
use veila_renderer::{
    ClearColor, FrameSize, SoftwareBuffer,
    background::{
        load_cached_generated_render, load_cached_generated_render_variant, load_cached_render,
        load_cached_render_variant,
    },
};

use crate::state::CurtainApp;

impl CurtainApp {
    pub(super) fn prepare_background(&mut self, index: usize, size: (u32, u32)) -> Result<bool> {
        let frame_size = FrameSize::new(size.0, size.1);
        let selected_path = self
            .background_path_for_surface(index)
            .map(ToOwned::to_owned);
        let needs_refresh = self.lock_surfaces[index]
            .background
            .as_ref()
            .map(|buffer| buffer.size() != frame_size)
            .unwrap_or(true);
        let source_changed = self.lock_surfaces[index].background_path != selected_path;

        if !needs_refresh && !source_changed {
            return Ok(false);
        }

        if let Some(path) = selected_path.as_deref() {
            match load_cached_render(path, frame_size, self.background_treatment) {
                Ok(Some(buffer)) => {
                    tracing::debug!(
                        path = %path.display(),
                        width = frame_size.width,
                        height = frame_size.height,
                        "using cached rendered background for initial lock frame"
                    );
                    self.lock_surfaces[index].background = Some(buffer);
                    self.lock_surfaces[index].background_path = selected_path;
                    return Ok(true);
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::debug!(
                        path = %path.display(),
                        width = frame_size.width,
                        height = frame_size.height,
                        "failed to load cached rendered background for initial frame: {error:#}"
                    );
                }
            }
        } else if let Some(generated) = self.background_generated {
            match load_cached_generated_render(generated, frame_size, self.background_treatment) {
                Ok(Some(buffer)) => {
                    tracing::debug!(
                        width = frame_size.width,
                        height = frame_size.height,
                        "using cached rendered generated background for initial lock frame"
                    );
                    self.lock_surfaces[index].background = Some(buffer);
                    self.lock_surfaces[index].background_path = None;
                    return Ok(true);
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::debug!(
                        width = frame_size.width,
                        height = frame_size.height,
                        "failed to load cached rendered generated background for initial frame: {error:#}"
                    );
                }
            }
        }

        self.lock_surfaces[index].background = Some(
            self.background_asset
                .render(frame_size)
                .map_err(|error| anyhow!("failed to render background asset: {error}"))?,
        );
        self.lock_surfaces[index].background_path = selected_path;

        Ok(true)
    }

    pub(super) fn prepare_static_overlay(
        &mut self,
        index: usize,
        size: (u32, u32),
    ) -> Result<bool> {
        let frame_size = FrameSize::new(size.0, size.1);
        let revision = self.ui_shell.static_scene_revision();
        let needs_refresh = self.lock_surfaces[index]
            .static_overlay
            .as_ref()
            .map(|buffer| buffer.size() != frame_size)
            .unwrap_or(true)
            || self.lock_surfaces[index].static_overlay_revision != revision;

        if !needs_refresh {
            return Ok(false);
        }

        let mut overlay = SoftwareBuffer::new(frame_size)?;
        overlay.clear(ClearColor::rgba(0, 0, 0, 0));
        self.ui_shell.render_static_overlay(&mut overlay);
        self.lock_surfaces[index].static_overlay = Some(overlay);
        self.lock_surfaces[index].static_overlay_revision = revision;

        Ok(true)
    }

    pub(super) fn prepare_scene_base(
        &mut self,
        index: usize,
        size: (u32, u32),
        background_refreshed: bool,
    ) -> Result<bool> {
        let frame_size = FrameSize::new(size.0, size.1);
        let revision = self.ui_shell.static_scene_revision();
        let needs_refresh = background_refreshed
            || self.lock_surfaces[index]
                .scene_base
                .as_ref()
                .map(|buffer| buffer.size() != frame_size)
                .unwrap_or(true)
            || self.lock_surfaces[index].scene_base_revision != revision;

        if !needs_refresh {
            return Ok(false);
        }

        if let Some(refreshed) =
            self.try_prepare_scene_base_without_background(index, frame_size, revision)?
        {
            return Ok(refreshed);
        }

        let Some(background) = self.lock_surfaces[index].background.as_ref() else {
            return Err(anyhow!("background buffer is unavailable"));
        };

        let mut buffer = background.clone();
        self.ui_shell.render_backdrop_layer(&mut buffer);
        self.lock_surfaces[index].scene_base = Some(buffer);
        self.lock_surfaces[index].scene_base_revision = revision;

        Ok(true)
    }

    pub(super) fn try_prepare_scene_base_without_background(
        &mut self,
        index: usize,
        frame_size: FrameSize,
        revision: u64,
    ) -> Result<Option<bool>> {
        let selected_path = self
            .background_path_for_surface(index)
            .map(ToOwned::to_owned);
        let needs_refresh = self.lock_surfaces[index]
            .scene_base
            .as_ref()
            .map(|buffer| buffer.size() != frame_size)
            .unwrap_or(true)
            || self.lock_surfaces[index].scene_base_revision != revision
            || self.lock_surfaces[index].background_path != selected_path;

        if !needs_refresh {
            return Ok(Some(false));
        }

        if let Some(buffer) = self
            .lock_surfaces
            .iter()
            .enumerate()
            .find(|(candidate_index, surface)| {
                *candidate_index != index
                    && surface.scene_base_revision == revision
                    && surface.background_path == selected_path
                    && surface
                        .scene_base
                        .as_ref()
                        .is_some_and(|buffer| buffer.size() == frame_size)
            })
            .and_then(|(_, surface)| surface.scene_base.clone())
        {
            self.lock_surfaces[index].scene_base = Some(buffer);
            self.lock_surfaces[index].scene_base_revision = revision;
            self.lock_surfaces[index].background = None;
            self.lock_surfaces[index].background_path = selected_path;
            return Ok(Some(true));
        }

        if let Some(variant) = self.ui_shell.layer_cache_variant() {
            if let Some(path) = selected_path.as_deref() {
                if let Ok(Some(buffer)) = load_cached_render_variant(
                    path,
                    frame_size,
                    self.background_treatment,
                    &variant,
                ) {
                    self.lock_surfaces[index].scene_base = Some(buffer);
                    self.lock_surfaces[index].scene_base_revision = revision;
                    self.lock_surfaces[index].background = None;
                    self.lock_surfaces[index].background_path = selected_path;
                    return Ok(Some(true));
                }
            } else if let Some(generated) = self.background_generated
                && let Ok(Some(buffer)) = load_cached_generated_render_variant(
                    generated,
                    frame_size,
                    self.background_treatment,
                    &variant,
                )
            {
                self.lock_surfaces[index].scene_base = Some(buffer);
                self.lock_surfaces[index].scene_base_revision = revision;
                self.lock_surfaces[index].background = None;
                self.lock_surfaces[index].background_path = None;
                return Ok(Some(true));
            }
        }

        Ok(None)
    }
}
