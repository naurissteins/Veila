use std::time::Instant;

use anyhow::{Result, anyhow};
use smithay_client_toolkit::{reexports::client::QueueHandle, session_lock::SessionLockSurface};
use veila_renderer::{copy_rect_from, shm::FrameResult};
use veila_ui::{WidgetDamage, WidgetKind};

use crate::state::{
    CommittedWidgetFrame, CurtainApp, DirtyRenderTimingSample, RedrawKind, SurfaceSize,
};

impl CurtainApp {
    pub(crate) fn render_widget_dirty_surface(
        &mut self,
        surface: &SessionLockSurface,
        size: SurfaceSize,
        widget: WidgetKind,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        let Some(index) = self
            .lock_surfaces
            .iter()
            .position(|entry| entry.surface.wl_surface() == surface.wl_surface())
        else {
            return Err(anyhow!("session-lock surface is no longer tracked"));
        };
        let revision = self.ui_shell.static_scene_revision();
        let frame_size = size.buffer;
        let scale = size.render_scale;
        let Some(previous) = self.lock_surfaces[index].widget_frame else {
            return self.render_surface_with_emergency_fallback(surface, size, queue_handle);
        };
        let Some(scene_base) = self.lock_surfaces[index].scene_base.as_ref().cloned() else {
            return self.render_surface_with_emergency_fallback(surface, size, queue_handle);
        };
        if previous.size != frame_size
            || previous.scale != scale
            || previous.revision != revision
            || scene_base.size() != frame_size
            || self.lock_surfaces[index].scene_base_revision != revision
            || self.lock_surfaces[index].scene_base_scale != scale
            || self.lock_surfaces[index].shm_pool.is_none()
            || self.lock_surfaces[index].pending_redraw.requires_full()
        {
            return self.render_surface_with_emergency_fallback(surface, size, queue_handle);
        }

        let current = self.ui_shell.widget_regions_at_scale(frame_size, scale);
        let damage = match current.damage_since(previous.regions, widget) {
            WidgetDamage::Full => {
                return self.render_surface_with_emergency_fallback(surface, size, queue_handle);
            }
            WidgetDamage::Skip => {
                self.lock_surfaces[index]
                    .pending_redraw
                    .satisfy(RedrawKind::Widget(widget));
                self.lock_surfaces[index].widget_frame = Some(CommittedWidgetFrame {
                    regions: current,
                    ..previous
                });
                return Ok(());
            }
            WidgetDamage::Region(rect) => rect,
        };
        let damage_pixels = u64::try_from(damage.width.max(0)).unwrap_or(0)
            * u64::try_from(damage.height.max(0)).unwrap_or(0);
        let frame_pixels = u64::from(frame_size.width) * u64::from(frame_size.height);
        if damage_pixels > frame_pixels / 2 {
            return self.render_surface_with_emergency_fallback(surface, size, queue_handle);
        }

        let timing_enabled = tracing::enabled!(tracing::Level::DEBUG);
        let started_at = timing_enabled.then(Instant::now);
        let overlay_started_at = timing_enabled.then(Instant::now);
        let mut overlay_ms = 0;
        let ui_shell = &self.ui_shell;
        self.configure_viewport_for_surface(index, size);
        let commit_started_at = timing_enabled.then(Instant::now);
        let result = self.lock_surfaces[index]
            .shm_pool
            .as_mut()
            .ok_or_else(|| anyhow!("surface SHM pool is unavailable"))?
            .render_buffer_region(
                queue_handle,
                surface.wl_surface(),
                frame_size,
                size.buffer_scale_for_commit(),
                damage,
                |buffer| {
                    copy_rect_from(scene_base.as_ref(), buffer, damage)?;
                    ui_shell.render_widget_at_scale(buffer, scale, widget);
                    if let Some(started_at) = overlay_started_at {
                        overlay_ms =
                            started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                    }
                    Ok(Some(damage))
                },
            )
            .map_err(|error| anyhow!("failed to render and commit widget dirty region: {error}"))?;
        if !self.lock_surfaces[index]
            .pending_redraw
            .record_result(RedrawKind::Widget(widget), result)
        {
            return Ok(());
        }
        self.lock_surfaces[index].widget_frame = Some(CommittedWidgetFrame {
            regions: current,
            ..previous
        });
        if result == FrameResult::Committed
            && let Some(started_at) = started_at
        {
            let total_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
            let commit_ms = commit_started_at
                .map(|instant| instant.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
                .unwrap_or(0);
            self.render_profiler.record_dirty(DirtyRenderTimingSample {
                dynamic_overlay_ms: overlay_ms,
                commit_ms,
                total_ms,
                dirty_pixels: damage_pixels,
                dirty_bytes: damage_pixels.saturating_mul(4),
            });
            tracing::debug!(
                ?widget,
                dirty_x = damage.x,
                dirty_y = damage.y,
                dirty_width = damage.width,
                dirty_height = damage.height,
                dirty_pixels = damage_pixels,
                overlay_ms,
                commit_ms,
                total_ms,
                "rendered widget dirty region"
            );
        }
        Ok(())
    }
}
