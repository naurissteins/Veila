use std::time::Instant;

use anyhow::{Result, anyhow};
use smithay_client_toolkit::{reexports::client::QueueHandle, session_lock::SessionLockSurface};
use veila_renderer::{CrossfadeProgress, PixelBuffer, copy_rect_from, crossfade_buffers_into, shm};

use crate::state::{CurtainApp, DirtyRenderTimingSample, RenderTimingSample, SurfaceSize};

impl CurtainApp {
    pub(crate) fn render_surface_with_emergency_fallback(
        &mut self,
        surface: &SessionLockSurface,
        size: SurfaceSize,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        match self.render_surface(surface, size, queue_handle) {
            Ok(()) => Ok(()),
            Err(error) if !self.ui_shell.emergency_active() => {
                let reason = format!("{error:#}");
                self.activate_emergency_ui(&reason)?;
                self.render_surface(surface, size, queue_handle)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn render_surface(
        &mut self,
        surface: &SessionLockSurface,
        size: SurfaceSize,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        let Some(index) = self
            .lock_surfaces
            .iter()
            .position(|entry| entry.surface.wl_surface() == surface.wl_surface())
        else {
            return Err(anyhow!("session-lock surface is no longer tracked"));
        };

        let timing_enabled = tracing::enabled!(tracing::Level::DEBUG);
        let total_started_at = timing_enabled.then(Instant::now);
        let first_frame = self.lock_surfaces[index].shm_pool.is_none();
        let frame_size = size.buffer;
        let render_scale = size.scale.max(1) as u32;
        let revision = self.ui_shell.static_scene_revision();
        let output_role = self.output_role_for_surface(index);
        let ui_visible = output_role.renders_shell();

        if let Some((from, to, progress)) = self.slideshow_crossfade_for_surface(index) {
            return self.render_surface_slideshow_crossfade(
                index,
                surface,
                size,
                queue_handle,
                &from,
                &to,
                progress,
                ui_visible,
                output_role.as_str(),
            );
        }

        let background_started_at = timing_enabled.then(Instant::now);
        let scene_base_cache_ready = if ui_visible {
            self.try_prepare_scene_base_without_background(index, frame_size, revision, size.scale)?
        } else {
            None
        };
        let background_refreshed = if scene_base_cache_ready.is_some() {
            false
        } else {
            self.prepare_background(index, size, ui_visible.then_some(revision))?
        };
        let scene_base_refreshed = if ui_visible {
            match scene_base_cache_ready {
                Some(refreshed) => refreshed,
                None => self.prepare_scene_base(index, size, background_refreshed)?,
            }
        } else {
            false
        };
        let background_prepare_ms = background_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);

        if !ui_visible && !first_frame && !background_refreshed {
            return Ok(());
        }

        if !ui_visible {
            return self.commit_background_only(
                index,
                surface,
                queue_handle,
                first_frame,
                background_refreshed,
                background_prepare_ms,
                total_started_at,
                timing_enabled,
                size,
                output_role.as_str(),
            );
        }

        if self.lock_surfaces[index].scene_base.is_none() {
            return Err(anyhow!("scene base buffer is unavailable"));
        }

        let background_restore_started_at = timing_enabled.then(Instant::now);
        let scene_base = self.lock_surfaces[index]
            .scene_base
            .as_ref()
            .expect("scene base buffer should exist")
            .clone();
        let background_restore_ms = background_restore_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);
        let shm_pool_started_at = timing_enabled.then(Instant::now);
        if self.lock_surfaces[index].shm_pool.is_none() {
            self.lock_surfaces[index].shm_pool =
                Some(shm::SurfaceBufferPool::new(&self.shm, frame_size)?);
        }
        let shm_pool_prepare_ms = shm_pool_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);

        let commit_started_at = timing_enabled.then(Instant::now);
        let dynamic_overlay_started_at = timing_enabled.then(Instant::now);
        let mut dynamic_overlay_ms = 0;
        let ui_shell = &self.ui_shell;
        self.configure_viewport_for_surface(index, size);
        let commit_result = {
            let lock_surface = &mut self.lock_surfaces[index];
            lock_surface
                .shm_pool
                .as_mut()
                .expect("surface SHM pool should be initialized")
                .render_buffer(
                    queue_handle,
                    surface.wl_surface(),
                    frame_size,
                    size.buffer_scale_for_commit(),
                    |buffer| {
                        buffer.pixels_mut().copy_from_slice(scene_base.pixels());
                        ui_shell.render_dynamic_overlay_scaled(buffer, render_scale);
                        if let Some(started_at) = dynamic_overlay_started_at {
                            dynamic_overlay_ms =
                                started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                        }
                        Ok(())
                    },
                )
        }
        .map_err(|error| anyhow!("failed to render and commit software buffer: {error}"));
        commit_result?;
        self.note_first_frame_committed(first_frame);

        if let Some(started_at) = total_started_at {
            let sample = RenderTimingSample {
                first_frame,
                background_prepare_ms,
                background_restore_ms,
                dynamic_overlay_ms,
                shm_pool_prepare_ms,
                commit_ms: commit_started_at
                    .map(|commit_started_at| {
                        commit_started_at
                            .elapsed()
                            .as_millis()
                            .min(u128::from(u64::MAX)) as u64
                    })
                    .unwrap_or(0),
                total_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            };
            self.render_profiler.record(sample);
            let output = self
                .output_state
                .info(&self.lock_surfaces[index].output)
                .and_then(|info| info.name.clone())
                .unwrap_or_else(|| format!("surface-{index}"));
            tracing::debug!(
                output,
                logical_width = size.logical_width,
                logical_height = size.logical_height,
                width = frame_size.width,
                height = frame_size.height,
                buffer_scale = size.scale,
                commit_buffer_scale = size.buffer_scale_for_commit(),
                fractional_scale = size.fractional_scale,
                output_role = output_role.as_str(),
                first_frame = sample.first_frame,
                background_refreshed,
                scene_base_refreshed,
                background_prepare_ms = sample.background_prepare_ms,
                background_restore_ms = sample.background_restore_ms,
                dynamic_overlay_ms = sample.dynamic_overlay_ms,
                shm_pool_prepare_ms = sample.shm_pool_prepare_ms,
                commit_ms = sample.commit_ms,
                total_ms = sample.total_ms,
                "rendered curtain frame"
            );
        }

        self.note_memory_after_render(first_frame);

        Ok(())
    }

    pub(crate) fn render_auth_dirty_surface(
        &mut self,
        surface: &SessionLockSurface,
        size: SurfaceSize,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        let Some(index) = self
            .lock_surfaces
            .iter()
            .position(|entry| entry.surface.wl_surface() == surface.wl_surface())
        else {
            return Err(anyhow!("session-lock surface is no longer tracked"));
        };

        let frame_size = size.buffer;
        let render_scale = size.scale.max(1) as u32;
        let revision = self.ui_shell.static_scene_revision();
        let Some(scene_base) = self.lock_surfaces[index].scene_base.as_ref().cloned() else {
            return self.render_surface(surface, size, queue_handle);
        };
        if scene_base.size() != frame_size
            || self.lock_surfaces[index].scene_base_revision != revision
            || self.lock_surfaces[index].shm_pool.is_none()
        {
            return self.render_surface(surface, size, queue_handle);
        }
        let Some(dirty_rect) = self
            .ui_shell
            .auth_dirty_rect_scaled(frame_size, render_scale)
        else {
            return self.render_surface(surface, size, queue_handle);
        };

        let timing_enabled = tracing::enabled!(tracing::Level::DEBUG);
        let total_started_at = timing_enabled.then(Instant::now);
        let dynamic_overlay_started_at = timing_enabled.then(Instant::now);
        let mut dynamic_overlay_ms = 0;
        let ui_shell = &self.ui_shell;
        self.configure_viewport_for_surface(index, size);
        let commit_started_at = timing_enabled.then(Instant::now);
        let damaged = {
            let lock_surface = &mut self.lock_surfaces[index];
            let mut damaged = dirty_rect;
            lock_surface
                .shm_pool
                .as_mut()
                .expect("surface SHM pool should be initialized")
                .render_buffer_region(
                    queue_handle,
                    surface.wl_surface(),
                    frame_size,
                    size.buffer_scale_for_commit(),
                    dirty_rect,
                    |buffer| {
                        if let Some(copied) =
                            copy_rect_from(scene_base.as_ref(), buffer, dirty_rect)?
                        {
                            damaged = copied;
                        }
                        ui_shell.render_auth_dirty_overlay_scaled(buffer, render_scale);
                        if let Some(started_at) = dynamic_overlay_started_at {
                            dynamic_overlay_ms =
                                started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                        }
                        Ok(Some(damaged))
                    },
                )
                .map(|_| damaged)
        }
        .map_err(|error| anyhow!("failed to render and commit auth dirty region: {error}"))?;

        if let Some(started_at) = total_started_at {
            let commit_ms = commit_started_at
                .map(|commit_started_at| {
                    commit_started_at
                        .elapsed()
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64
                })
                .unwrap_or(0);
            let dirty_pixels = u64::try_from(damaged.width.max(0))
                .unwrap_or(0)
                .saturating_mul(u64::try_from(damaged.height.max(0)).unwrap_or(0));
            let dirty_bytes = dirty_pixels.saturating_mul(4);
            let total_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
            self.render_profiler.record_dirty(DirtyRenderTimingSample {
                dynamic_overlay_ms,
                commit_ms,
                total_ms,
                dirty_pixels,
                dirty_bytes,
            });
            let output = self
                .output_state
                .info(&self.lock_surfaces[index].output)
                .and_then(|info| info.name.clone())
                .unwrap_or_else(|| format!("surface-{index}"));
            tracing::debug!(
                output,
                logical_width = size.logical_width,
                logical_height = size.logical_height,
                width = frame_size.width,
                height = frame_size.height,
                dirty_x = damaged.x,
                dirty_y = damaged.y,
                dirty_width = damaged.width,
                dirty_height = damaged.height,
                dirty_pixels,
                dirty_bytes,
                buffer_scale = size.scale,
                commit_buffer_scale = size.buffer_scale_for_commit(),
                fractional_scale = size.fractional_scale,
                dynamic_overlay_ms,
                commit_ms,
                total_ms,
                "rendered auth dirty region"
            );
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn commit_background_only(
        &mut self,
        index: usize,
        surface: &SessionLockSurface,
        queue_handle: &QueueHandle<Self>,
        first_frame: bool,
        background_refreshed: bool,
        background_prepare_ms: u64,
        total_started_at: Option<Instant>,
        timing_enabled: bool,
        size: SurfaceSize,
        output_role: &'static str,
    ) -> Result<()> {
        let Some(background) = self.lock_surfaces[index].background.take() else {
            return Err(anyhow!("background buffer is unavailable"));
        };

        let frame_size = background.size();
        let shm_pool_started_at = timing_enabled.then(Instant::now);
        if self.lock_surfaces[index].shm_pool.is_none() {
            self.lock_surfaces[index].shm_pool =
                Some(shm::SurfaceBufferPool::new(&self.shm, frame_size)?);
        }
        let shm_pool_prepare_ms = shm_pool_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);

        let commit_started_at = timing_enabled.then(Instant::now);
        self.configure_viewport_for_surface(index, size);
        let commit_result = self.lock_surfaces[index]
            .shm_pool
            .as_mut()
            .expect("surface SHM pool should be initialized")
            .commit_buffer(
                queue_handle,
                surface.wl_surface(),
                &background,
                size.buffer_scale_for_commit(),
            )
            .map_err(|error| anyhow!("failed to commit software buffer: {error}"));
        self.lock_surfaces[index].background = Some(background);
        commit_result?;
        self.note_first_frame_committed(first_frame);

        if let Some(started_at) = total_started_at {
            let sample = RenderTimingSample {
                first_frame,
                background_prepare_ms,
                background_restore_ms: 0,
                dynamic_overlay_ms: 0,
                shm_pool_prepare_ms,
                commit_ms: commit_started_at
                    .map(|commit_started_at| {
                        commit_started_at
                            .elapsed()
                            .as_millis()
                            .min(u128::from(u64::MAX)) as u64
                    })
                    .unwrap_or(0),
                total_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            };
            self.render_profiler.record(sample);
            let output = self
                .output_state
                .info(&self.lock_surfaces[index].output)
                .and_then(|info| info.name.clone())
                .unwrap_or_else(|| format!("surface-{index}"));
            tracing::debug!(
                output,
                logical_width = size.logical_width,
                logical_height = size.logical_height,
                width = frame_size.width,
                height = frame_size.height,
                buffer_scale = size.scale,
                commit_buffer_scale = size.buffer_scale_for_commit(),
                fractional_scale = size.fractional_scale,
                output_role,
                first_frame = sample.first_frame,
                background_refreshed,
                scene_base_refreshed = false,
                background_prepare_ms = sample.background_prepare_ms,
                background_restore_ms = 0,
                dynamic_overlay_ms = 0,
                shm_pool_prepare_ms = sample.shm_pool_prepare_ms,
                commit_ms = sample.commit_ms,
                total_ms = sample.total_ms,
                "rendered curtain frame"
            );
        }

        self.note_memory_after_render(first_frame);

        Ok(())
    }

    fn configure_viewport_for_surface(&self, index: usize, size: SurfaceSize) {
        let Some(viewport) = self.lock_surfaces[index].viewport.as_ref() else {
            return;
        };

        if size.fractional_scale.is_some() {
            viewport.set_destination(size.logical_width as i32, size.logical_height as i32);
        }
    }

    fn note_first_frame_committed(&mut self, first_frame: bool) {
        if !first_frame || self.first_frame_committed_at.is_some() {
            return;
        }

        let committed_at = Instant::now();
        self.first_frame_committed_at = Some(committed_at);
        let elapsed = committed_at.saturating_duration_since(self.startup_started_at);
        self.latency_timings.first_frame_ms =
            Some(elapsed.as_millis().min(u128::from(u64::MAX)) as u64);
        self.latency_timings.first_frame_us =
            Some(elapsed.as_micros().min(u128::from(u64::MAX)) as u64);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_surface_slideshow_crossfade(
        &mut self,
        index: usize,
        surface: &SessionLockSurface,
        size: SurfaceSize,
        queue_handle: &QueueHandle<Self>,
        from: &veila_renderer::SoftwareBuffer,
        to: &veila_renderer::SoftwareBuffer,
        progress: CrossfadeProgress,
        ui_visible: bool,
        output_role: &'static str,
    ) -> Result<()> {
        let frame_size = size.buffer;
        let render_scale = size.scale.max(1) as u32;

        if self.lock_surfaces[index].shm_pool.is_none() {
            self.lock_surfaces[index].shm_pool =
                Some(shm::SurfaceBufferPool::new(&self.shm, frame_size)?);
        }
        self.configure_viewport_for_surface(index, size);

        // Request the next vsync-aligned frame callback so the crossfade advances
        // in step with the compositor's refresh instead of a wall-clock timer.
        self.request_surface_frame_callback(index, queue_handle);

        if !ui_visible {
            self.lock_surfaces[index]
                .shm_pool
                .as_mut()
                .expect("surface SHM pool should be initialized")
                .render_buffer(
                    queue_handle,
                    surface.wl_surface(),
                    frame_size,
                    size.buffer_scale_for_commit(),
                    |buffer| crossfade_buffers_into(from, to, progress, buffer),
                )
                .map_err(|error| anyhow!("failed to commit crossfaded background: {error}"))?;
            return Ok(());
        }

        let ui_shell = &self.ui_shell;
        self.lock_surfaces[index]
            .shm_pool
            .as_mut()
            .expect("surface SHM pool should be initialized")
            .render_buffer(
                queue_handle,
                surface.wl_surface(),
                frame_size,
                size.buffer_scale_for_commit(),
                |buffer| {
                    crossfade_buffers_into(from, to, progress, buffer)?;
                    ui_shell.render_dynamic_overlay_scaled(buffer, render_scale);
                    Ok(())
                },
            )
            .map_err(|error| anyhow!("failed to render crossfaded slideshow frame: {error}"))?;

        tracing::trace!(
            output_role,
            progress,
            width = frame_size.width,
            height = frame_size.height,
            "rendered slideshow crossfade frame"
        );

        Ok(())
    }
}
