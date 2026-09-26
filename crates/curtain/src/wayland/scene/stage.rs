use anyhow::{Result, anyhow};
use smithay_client_toolkit::{
    reexports::client::QueueHandle,
    session_lock::{SessionLockSurface, SessionLockSurfaceConfigure},
};
use veila_renderer::{SoftwareBuffer, shm};

use crate::state::{CurtainApp, SurfaceSize, duration_ms_between, elapsed_ms, elapsed_us};

impl CurtainApp {
    pub(crate) fn configure_surface(
        &mut self,
        queue_handle: &QueueHandle<Self>,
        surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
    ) {
        let Some(index) = self
            .lock_surfaces
            .iter()
            .position(|entry| entry.surface.wl_surface() == surface.wl_surface())
        else {
            tracing::warn!("configure received for unknown session-lock surface");
            return;
        };

        let size = self.resolve_surface_size(index, configure.new_size);
        let previous_size = self.lock_surfaces[index].size;
        let was_unconfigured = previous_size.is_none();
        self.lock_surfaces[index].size = Some(size);
        if previous_size != Some(size) {
            self.background_render_started = false;
        }
        self.log_surface_size(index, configure.new_size, size);
        if was_unconfigured && !self.first_surface_configured_logged {
            self.first_surface_configured_logged = true;
            self.first_surface_configured_at = Some(std::time::Instant::now());
            self.latency_timings.first_surface_configured_ms =
                Some(elapsed_ms(self.startup_started_at));
            self.latency_timings.first_surface_configured_us =
                Some(elapsed_us(self.startup_started_at));
            tracing::info!(
                startup_elapsed_ms = elapsed_ms(self.startup_started_at),
                startup_elapsed_us = elapsed_us(self.startup_started_at),
                "first lock surface configured"
            );
        }
        if !self.all_surfaces_configured_logged
            && !self.lock_surfaces.is_empty()
            && self.lock_surfaces.iter().all(|entry| entry.size.is_some())
        {
            self.all_surfaces_configured_logged = true;
            let all_surfaces_configured_at = std::time::Instant::now();
            self.all_surfaces_configured_at = Some(all_surfaces_configured_at);
            self.latency_timings.all_surfaces_configured_ms =
                Some(elapsed_ms(self.startup_started_at));
            self.latency_timings.all_surfaces_configured_us =
                Some(elapsed_us(self.startup_started_at));
            tracing::info!(
                surfaces = self.lock_surfaces.len(),
                startup_elapsed_ms = elapsed_ms(self.startup_started_at),
                startup_elapsed_us = elapsed_us(self.startup_started_at),
                first_to_all_surfaces_ms = duration_ms_between(
                    self.first_surface_configured_at,
                    all_surfaces_configured_at,
                ),
                "all lock surfaces configured"
            );
        }
        if self.rich_scene_ready && !was_unconfigured {
            if let Err(error) =
                self.render_surface_with_emergency_fallback(&surface, size, queue_handle)
            {
                self.failure_reason = Some(format!("failed to render curtain surface: {error:#}"));
                self.exit_requested = true;
            }
            self.maybe_start_background_render();
            return;
        }

        if let Err(error) = self.commit_placeholder(index, size, queue_handle) {
            self.failure_reason = Some(format!("failed to commit curtain placeholder: {error:#}"));
            self.exit_requested = true;
            return;
        }

        if self.session_locked {
            if self.rich_scene_ready {
                if let Err(error) =
                    self.render_surface_with_emergency_fallback(&surface, size, queue_handle)
                {
                    self.failure_reason =
                        Some(format!("failed to render curtain surface: {error:#}"));
                    self.exit_requested = true;
                }
                self.maybe_start_background_render();
            } else {
                self.render_initial_scene(queue_handle);
            }
        }
    }

    pub(crate) fn redraw_scaled_surface(
        &mut self,
        index: usize,
        size: SurfaceSize,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        if !self.rich_scene_ready {
            self.commit_placeholder(index, size, queue_handle)?;
            if self.session_locked {
                self.render_initial_scene(queue_handle);
            }
            return Ok(());
        }

        let surface = self.lock_surfaces[index].surface.clone();
        self.render_surface_with_emergency_fallback(&surface, size, queue_handle)
    }

    pub(crate) fn render_initial_scene(&mut self, queue_handle: &QueueHandle<Self>) {
        if self.rich_scene_ready || self.failure_reason.is_some() {
            return;
        }

        let surfaces: Vec<_> = self
            .lock_surfaces
            .iter()
            .filter_map(|entry| entry.size.map(|size| (entry.surface.clone(), size)))
            .collect();
        if surfaces.is_empty()
            || self.lock_surfaces.iter().any(|entry| {
                entry.size.is_none()
                    || !entry
                        .placeholder_pool
                        .as_ref()
                        .is_some_and(veila_renderer::shm::SurfaceBufferPool::has_committed_frame)
            })
        {
            return;
        }

        for (surface, size) in surfaces {
            if let Err(error) =
                self.render_surface_with_emergency_fallback(&surface, size, queue_handle)
            {
                self.failure_reason =
                    Some(format!("failed to render initial curtain scene: {error:#}"));
                self.exit_requested = true;
                return;
            }
        }

        self.finish_initial_scene(queue_handle);
    }

    pub(crate) fn finish_initial_scene(&mut self, queue_handle: &QueueHandle<Self>) {
        if self.rich_scene_ready || !self.session_locked {
            return;
        }

        self.rich_scene_ready = self.lock_surfaces.iter().all(|entry| {
            entry
                .shm_pool
                .as_ref()
                .is_some_and(veila_renderer::shm::SurfaceBufferPool::has_committed_frame)
        });
        if !self.rich_scene_ready {
            return;
        }
        self.maybe_notify_ready();
        self.flush_pending_pre_ready_redraw(queue_handle);
        self.maybe_start_background_render();
        self.maybe_start_artwork_load();
    }

    pub(crate) fn commit_placeholder(
        &mut self,
        index: usize,
        size: SurfaceSize,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        let (buffer, buffer_scale) = self.prepare_placeholder(index, size)?;
        let placeholder_size = buffer.size();
        let surface = &mut self.lock_surfaces[index];
        if let Some(viewport) = surface.viewport.as_ref() {
            viewport.set_destination(size.logical_width as i32, size.logical_height as i32);
        }
        if surface.placeholder_pool.is_none() {
            surface.placeholder_pool =
                Some(shm::SurfaceBufferPool::new(&self.shm, placeholder_size)?);
        }
        let first_placeholder = !surface
            .placeholder_pool
            .as_ref()
            .is_some_and(shm::SurfaceBufferPool::has_committed_frame);
        let result = surface
            .placeholder_pool
            .as_mut()
            .ok_or_else(|| anyhow!("placeholder SHM pool is unavailable"))?
            .commit_buffer(
                queue_handle,
                surface.surface.wl_surface(),
                &buffer,
                buffer_scale,
            )?;
        if result == shm::FrameResult::Committed {
            self.note_first_frame_committed(first_placeholder);
            self.connection.flush()?;
            tracing::debug!(
                surface = index,
                width = placeholder_size.width,
                height = placeholder_size.height,
                startup_elapsed_us = crate::state::elapsed_us(self.startup_started_at),
                "committed curtain placeholder"
            );
            if self.latency_timings.placeholder_committed_us.is_none() {
                self.latency_timings.placeholder_committed_us =
                    Some(crate::state::elapsed_us(self.startup_started_at));
            }
        }
        Ok(())
    }

    fn prepare_placeholder(
        &self,
        index: usize,
        size: SurfaceSize,
    ) -> Result<(SoftwareBuffer, i32)> {
        let generated_preview = self.background_generated.is_some()
            && self.background_path_for_surface(index).is_none()
            && self.lock_surfaces[index].viewport.is_some();
        let (placeholder_size, buffer_scale) = if generated_preview {
            (size.generated_placeholder_size(), 1)
        } else {
            size.placeholder_buffer(self.lock_surfaces[index].viewport.is_some())
        };
        let solid_placeholder =
            || SoftwareBuffer::solid(placeholder_size, self.background_color.with_alpha(u8::MAX));
        let buffer = if generated_preview {
            match self.opaque_generated_preview(placeholder_size) {
                Ok(preview) => preview,
                Err(error) => {
                    tracing::warn!("failed to render generated lock placeholder: {error}");
                    solid_placeholder()?
                }
            }
        } else {
            solid_placeholder()?
        };
        Ok((buffer, buffer_scale))
    }
}
