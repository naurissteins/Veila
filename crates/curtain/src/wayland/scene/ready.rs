use std::path::Path;

use anyhow::{Context, Result};
use smithay_client_toolkit::{
    output::OutputInfo,
    reexports::client::QueueHandle,
    session_lock::{SessionLockSurface, SessionLockSurfaceConfigure},
};

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

    pub(super) fn resolve_surface_size(&self, index: usize, requested: (u32, u32)) -> (u32, u32) {
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
