use std::time::Instant;

use anyhow::{Result, anyhow};
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::{reexports::client::QueueHandle, session_lock::SessionLockSurface};
use veila_renderer::{
    FrameSize,
    backend::{FrameBackend, FrameBackendContext, FrameBackendPreference},
};

use crate::state::{CurtainApp, RenderTimingSample, SurfaceSize};

impl CurtainApp {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn commit_background_only(
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
        let frame_backend_started_at = timing_enabled.then(Instant::now);
        let frame_backend_context = self.frame_backend_context();
        prepare_frame_backend(
            &self.shm,
            frame_backend_context,
            self.frame_backend_preference,
            &mut self.lock_surfaces[index].frame_backend,
            frame_size,
        )?;
        let frame_backend_prepare_ms = frame_backend_started_at
            .map(|started_at| started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);

        let commit_started_at = timing_enabled.then(Instant::now);
        self.configure_viewport_for_surface(index, size);
        let commit_result = self.lock_surfaces[index]
            .frame_backend
            .as_mut()
            .expect("surface frame backend should be initialized")
            .commit_buffer(
                queue_handle,
                surface.wl_surface(),
                &background,
                size.buffer_scale_for_commit(),
            )
            .map_err(|error| anyhow!("failed to commit frame buffer: {error}"));
        self.lock_surfaces[index].background = Some(background);
        commit_result?;
        self.note_first_frame_committed(first_frame);

        if let Some(started_at) = total_started_at {
            let sample = RenderTimingSample {
                first_frame,
                background_prepare_ms,
                background_restore_ms: 0,
                dynamic_overlay_ms: 0,
                frame_backend_prepare_ms,
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
                frame_backend_prepare_ms = sample.frame_backend_prepare_ms,
                commit_ms = sample.commit_ms,
                total_ms = sample.total_ms,
                "rendered curtain frame"
            );
        }

        self.note_memory_after_render(first_frame);

        Ok(())
    }

    pub(super) fn frame_backend_context(&self) -> FrameBackendContext {
        #[cfg(feature = "gpu")]
        {
            use std::ptr::NonNull;

            NonNull::new(self.connection.backend().display_ptr().cast())
                .map(FrameBackendContext::wayland_display)
                .unwrap_or_else(FrameBackendContext::software_only)
        }

        #[cfg(not(feature = "gpu"))]
        {
            FrameBackendContext::software_only()
        }
    }
}

pub(super) fn prepare_frame_backend(
    shm: &Shm,
    context: FrameBackendContext,
    preference: FrameBackendPreference,
    frame_backend: &mut Option<FrameBackend>,
    frame_size: FrameSize,
) -> Result<()> {
    if frame_backend.is_some() {
        return Ok(());
    }

    let selection = FrameBackend::new_preferred(shm, context, frame_size, preference)?;
    let requested = selection.requested.as_str();
    let active = selection.backend.kind().as_str();
    if let Some(reason) = selection.fallback_reason {
        tracing::warn!(
            requested_frame_backend = requested,
            active_frame_backend = active,
            reason,
            "falling back to software frame backend"
        );
    } else {
        tracing::debug!(
            requested_frame_backend = requested,
            active_frame_backend = active,
            "created frame backend"
        );
    }

    *frame_backend = Some(selection.backend);
    Ok(())
}
