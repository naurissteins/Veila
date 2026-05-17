use std::path::Path;

use anyhow::{Context, Result};
use smithay_client_toolkit::{
    output::OutputInfo,
    reexports::client::QueueHandle,
    session_lock::{SessionLockSurface, SessionLockSurfaceConfigure},
};

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
        let was_unconfigured = self.lock_surfaces[index].size.is_none();
        self.lock_surfaces[index].size = Some(size);
        self.log_surface_size(index, configure.new_size, size);
        if was_unconfigured && !self.first_surface_configured_logged {
            self.first_surface_configured_logged = true;
            self.first_surface_configured_at = Some(std::time::Instant::now());
            self.latency_timings.first_surface_configured_ms =
                self.first_surface_configured_at.map(elapsed_ms);
            self.latency_timings.first_surface_configured_us =
                self.first_surface_configured_at.map(elapsed_us);
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
        self.maybe_start_background_render();

        if let Err(error) =
            self.render_surface_with_emergency_fallback(&surface, size, queue_handle)
        {
            self.failure_reason = Some(format!("failed to render curtain surface: {error:#}"));
            self.exit_requested = true;
            return;
        }

        self.maybe_notify_ready();
        self.flush_pending_pre_ready_redraw(queue_handle);
    }

    pub(crate) fn render_all_surfaces(&mut self, queue_handle: &QueueHandle<Self>) {
        if !self.ready_notified {
            if !self.pending_pre_ready_redraw {
                tracing::debug!("deferred non-critical redraw until curtain readiness");
            }
            self.pending_pre_ready_redraw = true;
            return;
        }

        self.refresh_power_status_text();
        let surfaces: Vec<_> = self
            .lock_surfaces
            .iter()
            .filter_map(|entry| entry.size.map(|size| (entry.surface.clone(), size)))
            .collect();

        for (surface, size) in surfaces {
            if let Err(error) =
                self.render_surface_with_emergency_fallback(&surface, size, queue_handle)
            {
                self.failure_reason = Some(format!("failed to rerender UI shell: {error:#}"));
                self.exit_requested = true;
                return;
            }
        }
    }

    pub(crate) fn maybe_notify_ready(&mut self) {
        if self.ready_notified || !self.session_locked || self.lock_surfaces.is_empty() {
            return;
        }

        if self.lock_surfaces.iter().any(|entry| entry.size.is_none()) {
            return;
        }

        self.ready_notified = true;
        self.refresh_scene_base_after_ready();
        self.latency_timings.ready_notified_ms = Some(elapsed_ms(self.startup_started_at));
        self.latency_timings.ready_notified_us = Some(elapsed_us(self.startup_started_at));
        self.latency_timings.surface_count = self.lock_surfaces.len();

        if let Some(path) = self.notify_socket.as_deref() {
            let report = self
                .latency_report
                .is_enabled()
                .then_some(&self.latency_timings);
            if let Err(error) = notify_ready(path, report) {
                tracing::warn!(?path, "failed to notify ready state: {error:#}");
            } else {
                let ready_notified_at = std::time::Instant::now();
                tracing::info!(
                    ?path,
                    startup_elapsed_ms = elapsed_ms(self.startup_started_at),
                    startup_elapsed_us = elapsed_us(self.startup_started_at),
                    session_locked_elapsed_ms = self.session_locked_at.map(elapsed_ms),
                    session_locked_elapsed_us = self.session_locked_at.map(elapsed_us),
                    first_surface_to_ready_ms =
                        duration_ms_between(self.first_surface_configured_at, ready_notified_at,),
                    all_surfaces_to_ready_ms =
                        duration_ms_between(self.all_surfaces_configured_at, ready_notified_at,),
                    "curtain reported readiness"
                );
            }
        }

        self.log_memory_snapshot("ready");
        self.maybe_start_avatar_load();
        self.maybe_power_off_secondary_outputs();
    }

    fn refresh_scene_base_after_ready(&mut self) {
        if !self.ui_shell.has_visual_layers()
            || self
                .lock_surfaces
                .iter()
                .all(|surface| surface.scene_base_has_layers)
        {
            return;
        }

        for surface in &mut self.lock_surfaces {
            surface.scene_base = None;
            surface.scene_base_revision = 0;
            surface.scene_base_has_layers = false;
        }
        self.pending_pre_ready_redraw = true;
    }

    pub(crate) fn flush_pending_pre_ready_redraw(&mut self, queue_handle: &QueueHandle<Self>) {
        if !self.ready_notified || !self.pending_pre_ready_redraw {
            return;
        }

        self.pending_pre_ready_redraw = false;
        self.render_all_surfaces(queue_handle);
    }

    pub(crate) fn resolve_surface_size(&self, index: usize, requested: (u32, u32)) -> SurfaceSize {
        let logical_size = if requested.0 > 0 && requested.1 > 0 {
            requested
        } else if let Some(info) = self.output_state.info(&self.lock_surfaces[index].output)
            && let Some((width, height)) = logical_size(&info)
        {
            tracing::warn!(
                output = info.name.as_deref().unwrap_or("unknown"),
                width,
                height,
                "lock surface configure had zero dimension; falling back to output logical size"
            );
            (width as u32, height as u32)
        } else {
            tracing::warn!("lock surface configure had zero dimension; falling back to 1920x1080");
            (1920, 1080)
        };

        SurfaceSize::new_with_fractional_scale(
            logical_size.0,
            logical_size.1,
            self.surface_scale(index),
            self.surface_fractional_scale(index),
        )
    }

    pub(super) fn surface_scale(&self, index: usize) -> i32 {
        let fractional_scale = self
            .surface_fractional_scale(index)
            .map(ceil_fractional_scale)
            .unwrap_or(1);
        let output_scale = self
            .output_state
            .info(&self.lock_surfaces[index].output)
            .map(|info| info.scale_factor)
            .unwrap_or(1)
            .max(1);
        self.lock_surfaces[index]
            .preferred_scale
            .max(output_scale)
            .max(fractional_scale)
            .max(1)
    }

    pub(super) fn surface_fractional_scale(&self, index: usize) -> Option<u32> {
        let surface = self.lock_surfaces.get(index)?;
        if surface.viewport.is_none() || surface.fractional_scale.is_none() {
            return None;
        }

        surface
            .preferred_fractional_scale
            .filter(|scale| *scale > 0)
    }

    fn log_surface_size(&self, index: usize, requested: (u32, u32), size: SurfaceSize) {
        let info = self.output_state.info(&self.lock_surfaces[index].output);
        let output = info
            .as_ref()
            .and_then(|info| info.name.as_deref())
            .unwrap_or("unknown");
        let output_logical_width = info
            .as_ref()
            .and_then(|info| info.logical_size.map(|size| size.0));
        let output_logical_height = info
            .as_ref()
            .and_then(|info| info.logical_size.map(|size| size.1));

        tracing::debug!(
            output,
            requested_width = requested.0,
            requested_height = requested.1,
            logical_width = size.logical_width,
            logical_height = size.logical_height,
            buffer_width = size.buffer.width,
            buffer_height = size.buffer.height,
            buffer_scale = size.scale,
            commit_buffer_scale = size.buffer_scale_for_commit(),
            fractional_scale = size.fractional_scale,
            preferred_buffer_scale = self.lock_surfaces[index].preferred_scale,
            output_logical_width,
            output_logical_height,
            "resolved session-lock surface size"
        );
    }
}

fn ceil_fractional_scale(scale: u32) -> i32 {
    scale.saturating_add(119).saturating_div(120).max(1) as i32
}

fn logical_size(info: &OutputInfo) -> Option<(i32, i32)> {
    let (width, height) = info.logical_size?;
    if width > 0 && height > 0 {
        Some((width, height))
    } else {
        None
    }
}

fn notify_ready(
    path: &Path,
    report: Option<&veila_common::ipc::CurtainLatencyReport>,
) -> Result<()> {
    use std::io::Write as _;
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(path)
        .with_context(|| format!("failed to connect to notify socket {}", path.display()))?;
    if let Some(report) = report {
        let mut payload =
            veila_common::ipc::encode_message(report).context("failed to encode latency report")?;
        payload.push('\n');
        stream
            .write_all(payload.as_bytes())
            .with_context(|| format!("failed to write readiness report to {}", path.display()))?;
    } else {
        stream
            .write_all(&[1u8])
            .with_context(|| format!("failed to write readiness byte to {}", path.display()))?;
    }

    Ok(())
}
