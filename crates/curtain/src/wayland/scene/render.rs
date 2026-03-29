use std::time::Instant;

use anyhow::{Result, anyhow};
use smithay_client_toolkit::{reexports::client::QueueHandle, session_lock::SessionLockSurface};
use veila_renderer::{FrameSize, shm};

use crate::state::{CurtainApp, RenderTimingSample};

impl CurtainApp {
    pub(super) fn render_surface(
        &mut self,
        surface: &SessionLockSurface,
        size: (u32, u32),
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
        let frame_size = FrameSize::new(size.0, size.1);
        let revision = self.ui_shell.static_scene_revision();
        let background_started_at = timing_enabled.then(Instant::now);
        let scene_base_cache_ready =
            self.try_prepare_scene_base_without_background(index, frame_size, revision)?;
        let background_refreshed = if scene_base_cache_ready.is_some() {
            false
        } else {
            self.prepare_background(index, size)?
        };
        let scene_base_refreshed = match scene_base_cache_ready {
            Some(refreshed) => refreshed,
            None => self.prepare_scene_base(index, size, background_refreshed)?,
        };
        let background_prepare_ms = background_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);
        let static_started_at = timing_enabled.then(Instant::now);
        let static_overlay_refreshed = self.prepare_static_overlay(index, size)?;
        let static_overlay_prepare_ms = static_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);

        let Some(scene_base) = self.lock_surfaces[index].scene_base.as_ref() else {
            return Err(anyhow!("scene base buffer is unavailable"));
        };

        let background_restore_started_at = timing_enabled.then(Instant::now);
        let mut buffer = scene_base.clone();
        let background_restore_ms = background_restore_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);
        let static_blend_started_at = timing_enabled.then(Instant::now);
        if let Some(static_overlay) = self.lock_surfaces[index].static_overlay.as_ref() {
            buffer.blend_from(static_overlay)?;
        }
        let static_overlay_blend_ms = static_blend_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);
        let dynamic_overlay_started_at = timing_enabled.then(Instant::now);
        self.ui_shell.render_dynamic_overlay(&mut buffer);
        let dynamic_overlay_ms = dynamic_overlay_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);
        let frame_size = buffer.size();
        let shm_pool_started_at = timing_enabled.then(Instant::now);
        if self.lock_surfaces[index].shm_pool.is_none() {
            self.lock_surfaces[index].shm_pool =
                Some(shm::SurfaceBufferPool::new(&self.shm, frame_size)?);
        }
        let shm_pool_prepare_ms = shm_pool_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);

        let commit_started_at = timing_enabled.then(Instant::now);
        self.lock_surfaces[index]
            .shm_pool
            .as_mut()
            .expect("surface SHM pool should be initialized")
            .commit_buffer(queue_handle, surface.wl_surface(), &buffer)
            .map_err(|error| anyhow!("failed to commit software buffer: {error}"))?;

        if let Some(started_at) = total_started_at {
            let sample = RenderTimingSample {
                first_frame,
                background_prepare_ms,
                static_overlay_prepare_ms,
                background_restore_ms,
                static_overlay_blend_ms,
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
                width = frame_size.width,
                height = frame_size.height,
                first_frame = sample.first_frame,
                background_refreshed,
                scene_base_refreshed,
                static_overlay_refreshed,
                background_prepare_ms = sample.background_prepare_ms,
                static_overlay_prepare_ms = sample.static_overlay_prepare_ms,
                background_restore_ms = sample.background_restore_ms,
                static_overlay_blend_ms = sample.static_overlay_blend_ms,
                dynamic_overlay_ms = sample.dynamic_overlay_ms,
                shm_pool_prepare_ms = sample.shm_pool_prepare_ms,
                commit_ms = sample.commit_ms,
                total_ms = sample.total_ms,
                "rendered curtain frame"
            );
        }

        Ok(())
    }
}
