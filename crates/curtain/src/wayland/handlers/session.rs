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

use crate::state::{CurtainApp, elapsed_ms, elapsed_us};

impl SessionLockHandler for CurtainApp {
    fn locked(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _session_lock: SessionLock) {
        self.session_locked_at = Some(std::time::Instant::now());
        tracing::info!(
            startup_elapsed_ms = elapsed_ms(self.startup_started_at),
            startup_elapsed_us = elapsed_us(self.startup_started_at),
            "session lock confirmed by compositor"
        );
        self.session_locked = true;
        self.maybe_notify_ready();
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
        self.lock_surfaces.retain(|entry| entry.output != output);
    }
}

impl CompositorHandler for CurtainApp {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
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
