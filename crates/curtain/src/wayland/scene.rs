use std::path::Path;

use anyhow::{Context, Result, anyhow};
use smithay_client_toolkit::{
    output::OutputInfo,
    reexports::client::QueueHandle,
    session_lock::{SessionLockSurface, SessionLockSurfaceConfigure},
};
use veila_renderer::{FrameSize, background::load_cached_render, shm};

use crate::state::CurtainApp;

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

        if let Err(error) = self.prepare_background(index, size) {
            self.failure_reason = Some(format!("failed to prepare curtain background: {error:#}"));
            self.exit_requested = true;
            return;
        }

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

    fn prepare_background(&mut self, index: usize, size: (u32, u32)) -> Result<()> {
        let frame_size = FrameSize::new(size.0, size.1);
        let needs_refresh = self.lock_surfaces[index]
            .background
            .as_ref()
            .map(|buffer| buffer.size() != frame_size)
            .unwrap_or(true);

        if !needs_refresh {
            return Ok(());
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
                    return Ok(());
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

        Ok(())
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

        self.prepare_background(index, size)?;

        let Some(background) = self.lock_surfaces[index].background.as_ref() else {
            return Err(anyhow!("background buffer is unavailable"));
        };

        let mut buffer = background.clone();
        self.ui_shell.render_overlay(&mut buffer);
        shm::commit_buffer(&self.shm, queue_handle, surface.wl_surface(), &buffer)
            .map_err(|error| anyhow!("failed to commit software buffer: {error}"))
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
