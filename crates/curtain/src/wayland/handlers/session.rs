use smithay_client_toolkit::{
    compositor::CompositorHandler,
    output::OutputHandler,
    reexports::client::{
        Connection, Proxy, QueueHandle,
        protocol::{wl_output, wl_surface},
    },
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockSurface, SessionLockSurfaceConfigure,
    },
};

use crate::state::{CurtainApp, duration_ms_between, elapsed_ms, elapsed_us};

impl SessionLockHandler for CurtainApp {
    fn locked(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, _session_lock: SessionLock) {
        let session_locked_at = std::time::Instant::now();
        self.session_locked_at = Some(session_locked_at);
        self.latency_timings.session_locked_ms = Some(elapsed_ms(self.startup_started_at));
        self.latency_timings.session_locked_us = Some(elapsed_us(self.startup_started_at));
        tracing::info!(
            startup_elapsed_ms = elapsed_ms(self.startup_started_at),
            startup_elapsed_us = elapsed_us(self.startup_started_at),
            first_surface_to_session_lock_ms =
                duration_ms_between(self.first_surface_configured_at, session_locked_at),
            all_surfaces_to_session_lock_ms =
                duration_ms_between(self.all_surfaces_configured_at, session_locked_at),
            "session lock confirmed by compositor"
        );
        self.session_locked = true;
        self.screen_off.arm(session_locked_at);
        self.maybe_notify_ready();
        self.flush_pending_pre_ready_redraw(qh);
    }

    fn finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session_lock: SessionLock,
    ) {
        tracing::warn!("compositor denied or revoked the session lock");
        self.session_finished = true;
        self.failure_reason = Some("compositor denied or revoked the session lock".to_string());
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
        surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        _serial: u32,
    ) {
        self.configure_surface(queue_handle, surface, configure);
    }
}

impl OutputHandler for CurtainApp {
    fn output_state(&mut self) -> &mut smithay_client_toolkit::output::OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Err(error) = self.create_surface_for_output(output.clone(), queue_handle) {
            self.failure_reason = Some(format!(
                "failed to create session-lock surface for new output: {error:#}"
            ));
            self.exit_requested = true;
            return;
        }

        tracing::info!(
            id = output.id().protocol_id(),
            "registered new output while locked"
        );
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _queue_handle: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _queue_handle: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        for surface in &mut self.lock_surfaces {
            if surface.output == output
                && let Some(output_power) = surface.output_power.take()
            {
                output_power.destroy();
            }
        }
        self.lock_surfaces.retain(|entry| entry.output != output);
    }
}

impl CompositorHandler for CurtainApp {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        new_factor: i32,
    ) {
        let Some(index) = self
            .lock_surfaces
            .iter()
            .position(|entry| entry.surface.wl_surface() == surface)
        else {
            return;
        };
        self.lock_surfaces[index].preferred_scale = new_factor.max(1);

        let Some(previous) = self.lock_surfaces[index].size else {
            tracing::debug!(
                buffer_scale = new_factor.max(1),
                "lock surface scale changed before configure"
            );
            return;
        };

        let size =
            self.resolve_surface_size(index, (previous.logical_width, previous.logical_height));
        if size == previous {
            return;
        }

        tracing::debug!(
            old_buffer_scale = previous.scale,
            new_buffer_scale = size.scale,
            logical_width = size.logical_width,
            logical_height = size.logical_height,
            buffer_width = size.buffer.width,
            buffer_height = size.buffer.height,
            "rerendering lock surface after scale change"
        );
        self.lock_surfaces[index].size = Some(size);
        let lock_surface = self.lock_surfaces[index].surface.clone();
        if let Err(error) = self.render_surface(&lock_surface, size, qh) {
            self.failure_reason = Some(format!(
                "failed to rerender scaled curtain surface: {error:#}"
            ));
            self.exit_requested = true;
        }
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}
