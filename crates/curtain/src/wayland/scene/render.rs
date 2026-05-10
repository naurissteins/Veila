use std::time::Instant;

use anyhow::{Result, anyhow};
use smithay_client_toolkit::{reexports::client::QueueHandle, session_lock::SessionLockSurface};
use veila_renderer::{FrameSize, shm};

use crate::state::{CurtainApp, FinalFrameKey, RenderTimingSample, ScratchBuffer, SurfaceSize};

impl CurtainApp {
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
        let ui_visible = self.ui_visible_on_surface(index);
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
            );
        }

        if self.lock_surfaces[index].scene_base.is_none() {
            return Err(anyhow!("scene base buffer is unavailable"));
        }

        let final_frame_key = self.final_frame_key(index, frame_size, revision, render_scale);
        if first_frame
            && !self.ready_notified
            && let Some(key) = final_frame_key
            && self.commit_cached_final_frame(
                index,
                surface,
                queue_handle,
                size,
                key,
                background_prepare_ms,
                total_started_at,
                timing_enabled,
                scene_base_refreshed,
            )?
        {
            return Ok(());
        }

        let background_restore_started_at = timing_enabled.then(Instant::now);
        let mut scratch_buffer = self.prepare_scratch_buffer(index, frame_size)?;
        let background_restore_ms = background_restore_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);
        let dynamic_overlay_started_at = timing_enabled.then(Instant::now);
        self.ui_shell
            .render_dynamic_overlay_scaled(&mut scratch_buffer, render_scale);
        let dynamic_overlay_ms = dynamic_overlay_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);
        let frame_size = scratch_buffer.size();
        let shm_pool_started_at = timing_enabled.then(Instant::now);
        if self.lock_surfaces[index].shm_pool.is_none() {
            self.lock_surfaces[index].shm_pool =
                Some(shm::SurfaceBufferPool::new(&self.shm, frame_size)?);
        }
        let shm_pool_prepare_ms = shm_pool_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);

        let commit_started_at = timing_enabled.then(Instant::now);
        let commit_result = self.lock_surfaces[index]
            .shm_pool
            .as_mut()
            .expect("surface SHM pool should be initialized")
            .commit_buffer(
                queue_handle,
                surface.wl_surface(),
                &scratch_buffer,
                size.scale,
            )
            .map_err(|error| anyhow!("failed to commit software buffer: {error}"));
        self.scratch_buffers.push(ScratchBuffer {
            buffer: scratch_buffer,
            final_frame_key: if first_frame && !self.ready_notified {
                final_frame_key
            } else {
                None
            },
        });
        commit_result?;

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

    #[allow(clippy::too_many_arguments)]
    fn commit_cached_final_frame(
        &mut self,
        index: usize,
        surface: &SessionLockSurface,
        queue_handle: &QueueHandle<Self>,
        size: SurfaceSize,
        key: FinalFrameKey,
        background_prepare_ms: u64,
        total_started_at: Option<Instant>,
        timing_enabled: bool,
        scene_base_refreshed: bool,
    ) -> Result<bool> {
        let Some(position) = self
            .scratch_buffers
            .iter()
            .position(|scratch| scratch.final_frame_key == Some(key))
        else {
            return Ok(false);
        };

        let scratch = self.scratch_buffers.swap_remove(position);
        let frame_size = scratch.buffer.size();
        let shm_pool_started_at = timing_enabled.then(Instant::now);
        if self.lock_surfaces[index].shm_pool.is_none() {
            self.lock_surfaces[index].shm_pool =
                Some(shm::SurfaceBufferPool::new(&self.shm, frame_size)?);
        }
        let shm_pool_prepare_ms = shm_pool_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);

        let commit_started_at = timing_enabled.then(Instant::now);
        let commit_result = self.lock_surfaces[index]
            .shm_pool
            .as_mut()
            .expect("surface SHM pool should be initialized")
            .commit_buffer(
                queue_handle,
                surface.wl_surface(),
                &scratch.buffer,
                size.scale,
            )
            .map_err(|error| anyhow!("failed to commit cached software buffer: {error}"));
        self.scratch_buffers.push(scratch);
        commit_result?;

        if let Some(started_at) = total_started_at {
            let sample = RenderTimingSample {
                first_frame: true,
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
                first_frame = sample.first_frame,
                background_refreshed = false,
                scene_base_refreshed,
                final_frame_cache_hit = true,
                background_prepare_ms = sample.background_prepare_ms,
                background_restore_ms = sample.background_restore_ms,
                dynamic_overlay_ms = sample.dynamic_overlay_ms,
                shm_pool_prepare_ms = sample.shm_pool_prepare_ms,
                commit_ms = sample.commit_ms,
                total_ms = sample.total_ms,
                "rendered curtain frame"
            );
        }

        self.note_memory_after_render(true);

        Ok(true)
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
        let commit_result = self.lock_surfaces[index]
            .shm_pool
            .as_mut()
            .expect("surface SHM pool should be initialized")
            .commit_buffer(queue_handle, surface.wl_surface(), &background, size.scale)
            .map_err(|error| anyhow!("failed to commit software buffer: {error}"));
        self.lock_surfaces[index].background = Some(background);
        commit_result?;

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

    fn prepare_scratch_buffer(
        &mut self,
        index: usize,
        frame_size: FrameSize,
    ) -> Result<veila_renderer::SoftwareBuffer> {
        let mut scratch_buffer = if let Some(position) = self
            .scratch_buffers
            .iter()
            .position(|scratch| scratch.buffer.size() == frame_size)
        {
            self.scratch_buffers.swap_remove(position).buffer
        } else {
            veila_renderer::SoftwareBuffer::new(frame_size)?
        };

        let scene_base = self.lock_surfaces[index]
            .scene_base
            .as_ref()
            .expect("scene base buffer should exist");
        scratch_buffer
            .pixels_mut()
            .copy_from_slice(scene_base.pixels());

        Ok(scratch_buffer)
    }

    fn final_frame_key(
        &self,
        index: usize,
        frame_size: FrameSize,
        scene_revision: u64,
        render_scale: u32,
    ) -> Option<FinalFrameKey> {
        let scene_base = self.lock_surfaces[index].scene_base.as_ref()?;
        Some(FinalFrameKey {
            scene_base_ptr: std::sync::Arc::as_ptr(scene_base) as usize,
            frame_size,
            scene_revision,
            render_scale,
        })
    }
}
