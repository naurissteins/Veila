use std::{path::Path, time::Instant};

use anyhow::{Context, Result, anyhow};
use smithay_client_toolkit::{
    output::OutputInfo,
    reexports::client::QueueHandle,
    session_lock::{SessionLockSurface, SessionLockSurfaceConfigure},
};
use veila_renderer::{
    ClearColor, FrameSize, SoftwareBuffer,
    background::{load_cached_render, load_cached_render_variant},
    shm,
};

use crate::state::{CurtainApp, RenderTimingSample};

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
        self.lock_surfaces[index].size = Some(size);
        self.maybe_start_background_render();

        if let Err(error) = self.render_surface(&surface, size, queue_handle) {
            self.failure_reason = Some(format!("failed to render curtain surface: {error:#}"));
            self.exit_requested = true;
            return;
        }

        self.maybe_notify_ready();
    }

    pub(crate) fn render_all_surfaces(&mut self, queue_handle: &QueueHandle<Self>) {
        let surfaces: Vec<_> = self
            .lock_surfaces
            .iter()
            .filter_map(|entry| entry.size.map(|size| (entry.surface.clone(), size)))
            .collect();

        for (surface, size) in surfaces {
            if let Err(error) = self.render_surface(&surface, size, queue_handle) {
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

        if let Some(path) = self.notify_socket.as_deref() {
            if let Err(error) = notify_ready(path) {
                tracing::warn!(?path, "failed to notify ready state: {error:#}");
            } else {
                tracing::info!(?path, "curtain reported readiness");
            }
        }
    }

    fn prepare_background(&mut self, index: usize, size: (u32, u32)) -> Result<bool> {
        let frame_size = FrameSize::new(size.0, size.1);
        let needs_refresh = self.lock_surfaces[index]
            .background
            .as_ref()
            .map(|buffer| buffer.size() != frame_size)
            .unwrap_or(true);

        if !needs_refresh {
            return Ok(false);
        }

        if let Some(path) = self.background_path.as_deref() {
            match load_cached_render(path, frame_size, self.background_treatment) {
                Ok(Some(buffer)) => {
                    tracing::debug!(
                        path = %path.display(),
                        width = frame_size.width,
                        height = frame_size.height,
                        "using cached rendered background for initial lock frame"
                    );
                    self.lock_surfaces[index].background = Some(buffer);
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
        }

        self.lock_surfaces[index].background = Some(
            self.background_asset
                .render(frame_size)
                .map_err(|error| anyhow!("failed to render background asset: {error}"))?,
        );

        Ok(true)
    }

    fn render_surface(
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

    fn prepare_static_overlay(&mut self, index: usize, size: (u32, u32)) -> Result<bool> {
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

    fn prepare_scene_base(
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

    fn try_prepare_scene_base_without_background(
        &mut self,
        index: usize,
        frame_size: FrameSize,
        revision: u64,
    ) -> Result<Option<bool>> {
        let needs_refresh = self.lock_surfaces[index]
            .scene_base
            .as_ref()
            .map(|buffer| buffer.size() != frame_size)
            .unwrap_or(true)
            || self.lock_surfaces[index].scene_base_revision != revision;

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
                    && surface
                        .scene_base
                        .as_ref()
                        .is_some_and(|buffer| buffer.size() == frame_size)
            })
            .and_then(|(_, surface)| surface.scene_base.clone())
        {
            self.lock_surfaces[index].scene_base = Some(buffer);
            self.lock_surfaces[index].scene_base_revision = revision;
            return Ok(Some(true));
        }

        if let (Some(path), Some(variant)) = (
            self.background_path.as_deref(),
            self.ui_shell.layer_cache_variant(),
        ) && let Ok(Some(buffer)) =
            load_cached_render_variant(path, frame_size, self.background_treatment, &variant)
        {
            self.lock_surfaces[index].scene_base = Some(buffer);
            self.lock_surfaces[index].scene_base_revision = revision;
            return Ok(Some(true));
        }

        Ok(None)
    }
    fn resolve_surface_size(&self, index: usize, requested: (u32, u32)) -> (u32, u32) {
        if requested.0 > 0 && requested.1 > 0 {
            return requested;
        }

        if let Some(info) = self.output_state.info(&self.lock_surfaces[index].output)
            && let Some((width, height)) = logical_size(&info)
        {
            tracing::warn!(
                output = info.name.as_deref().unwrap_or("unknown"),
                width,
                height,
                "lock surface configure had zero dimension; falling back to output logical size"
            );
            return (width as u32, height as u32);
        }

        tracing::warn!("lock surface configure had zero dimension; falling back to 1920x1080");
        (1920, 1080)
    }
}

fn logical_size(info: &OutputInfo) -> Option<(i32, i32)> {
    let (width, height) = info.logical_size?;
    if width > 0 && height > 0 {
        Some((width, height))
    } else {
        None
    }
}

fn notify_ready(path: &Path) -> Result<()> {
    use std::io::Write as _;
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(path)
        .with_context(|| format!("failed to connect to notify socket {}", path.display()))?;
    stream
        .write_all(&[1u8])
        .with_context(|| format!("failed to write readiness byte to {}", path.display()))?;

    Ok(())
}
