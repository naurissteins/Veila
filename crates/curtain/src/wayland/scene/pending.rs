use smithay_client_toolkit::reexports::client::{QueueHandle, protocol::wl_surface};

use crate::state::{CurtainApp, RedrawKind};

impl CurtainApp {
    pub(crate) fn render_pending_surface(
        &mut self,
        wl_surface: &wl_surface::WlSurface,
        queue_handle: &QueueHandle<Self>,
    ) {
        let Some(index) = self
            .lock_surfaces
            .iter()
            .position(|entry| entry.surface.wl_surface() == wl_surface)
        else {
            return;
        };
        if let Some(pool) = self.lock_surfaces[index].shm_pool.as_mut() {
            pool.collect_released();
        }
        let Some(redraw) = self.lock_surfaces[index].pending_redraw.take() else {
            return;
        };
        self.lock_surfaces[index].pending_redraw.request(redraw);
        let Some(size) = self.lock_surfaces[index].size else {
            return;
        };
        let surface = self.lock_surfaces[index].surface.clone();
        let result = match redraw {
            RedrawKind::AuthDirty => self.render_auth_dirty_surface(&surface, size, queue_handle),
            RedrawKind::Full => {
                self.render_surface_with_emergency_fallback(&surface, size, queue_handle)
            }
        };
        if let Err(error) = result {
            self.failure_reason =
                Some(format!("failed to render pending curtain frame: {error:#}"));
            self.exit_requested = true;
        }
    }
}
