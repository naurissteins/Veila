use std::time::Instant;

use anyhow::{Result, anyhow};
use smithay_client_toolkit::{reexports::client::QueueHandle, session_lock::SessionLockSurface};
use veila_common::elapsed_ms;
use veila_renderer::copy_rect_from;
use veila_ui::WidgetDamage;

use crate::state::{
    CommittedWidgetFrame, CurtainApp, DirtyRenderTimingSample, RedrawKind, SurfaceSize,
};

impl CurtainApp {
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
        let render_scale = size.render_scale;
        let revision = self.ui_shell.static_scene_revision();
        let Some(previous) = self.lock_surfaces[index].widget_frame else {
            return self.render_surface(surface, size, queue_handle);
        };
        let Some(scene_base) = self.lock_surfaces[index].scene_base.as_ref().cloned() else {
            return self.render_surface(surface, size, queue_handle);
        };
        if previous.size != frame_size
            || previous.scale != render_scale
            || previous.revision != revision
            || scene_base.size() != frame_size
            || self.lock_surfaces[index].scene_base_revision != revision
            || self.lock_surfaces[index].scene_base_scale != render_scale
            || self.lock_surfaces[index].shm_pool.is_none()
            || self.lock_surfaces[index].pending_redraw.requires_full()
        {
            return self.render_surface(surface, size, queue_handle);
        }
        let current = self
            .ui_shell
            .widget_regions_at_scale(frame_size, render_scale);
        let WidgetDamage::Region(dirty_rect) = current.auth_damage_since(previous.regions) else {
            return self.render_surface(surface, size, queue_handle);
        };

        let timing_enabled = tracing::enabled!(tracing::Level::DEBUG);
        let total_started_at = timing_enabled.then(Instant::now);
        let dynamic_overlay_started_at = timing_enabled.then(Instant::now);
        let mut dynamic_overlay_ms = 0;
        let ui_shell = &self.ui_shell;
        self.configure_viewport_for_surface(index, size);
        let commit_started_at = timing_enabled.then(Instant::now);
        let (frame_result, damaged) = {
            let lock_surface = &mut self.lock_surfaces[index];
            let mut damaged = dirty_rect;
            lock_surface
                .shm_pool
                .as_mut()
                // The fallback above guarantees a pool is present.
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
                        ui_shell.render_auth_dirty_overlay_at_scale(buffer, render_scale);
                        if let Some(started_at) = dynamic_overlay_started_at {
                            dynamic_overlay_ms = elapsed_ms(started_at);
                        }
                        Ok(Some(damaged))
                    },
                )
                .map(|frame_result| (frame_result, damaged))
        }
        .map_err(|error| anyhow!("failed to render and commit auth dirty region: {error}"))?;
        if !self.lock_surfaces[index]
            .pending_redraw
            .record_result(RedrawKind::AuthDirty, frame_result)
        {
            return Ok(());
        }
        self.lock_surfaces[index].widget_frame = Some(CommittedWidgetFrame {
            regions: current,
            ..previous
        });

        if let Some(started_at) = total_started_at {
            let commit_ms = commit_started_at.map(elapsed_ms).unwrap_or(0);
            let dirty_pixels = u64::try_from(damaged.width.max(0))
                .unwrap_or(0)
                .saturating_mul(u64::try_from(damaged.height.max(0)).unwrap_or(0));
            let dirty_bytes = dirty_pixels.saturating_mul(4);
            let total_ms = elapsed_ms(started_at);
            self.render_profiler.record_dirty(DirtyRenderTimingSample {
                dynamic_overlay_ms,
                commit_ms,
                total_ms,
                dirty_pixels,
                dirty_bytes,
            });
            self.lock_surfaces[index].with_output_name(|name| {
                let fallback;
                let output = match name {
                    Some(name) => name,
                    None => {
                        fallback = format!("surface-{index}");
                        &fallback
                    }
                };
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
            });
        }

        Ok(())
    }
}
