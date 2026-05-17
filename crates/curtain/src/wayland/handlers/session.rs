use smithay_client_toolkit::{
    compositor::CompositorHandler,
    output::OutputHandler,
    reexports::{
        client::{
            Connection, Dispatch, Proxy, QueueHandle,
            protocol::{wl_output, wl_surface},
        },
        protocols::wp::fractional_scale::v1::client::{
            wp_fractional_scale_manager_v1, wp_fractional_scale_v1,
        },
    },
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockSurface, SessionLockSurfaceConfigure,
    },
};
use wayland_protocols_wlr::output_power_management::v1::client::zwlr_output_power_v1;

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

impl Dispatch<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1, ()> for CurtainApp {
    fn event(
        _: &mut Self,
        _: &wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        _: <wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_fractional_scale_v1::WpFractionalScaleV1, wl_surface::WlSurface> for CurtainApp {
    fn event(
        state: &mut Self,
        _: &wp_fractional_scale_v1::WpFractionalScaleV1,
        event: <wp_fractional_scale_v1::WpFractionalScaleV1 as Proxy>::Event,
        surface: &wl_surface::WlSurface,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let wp_fractional_scale_v1::Event::PreferredScale { scale } = event else {
            return;
        };

        let Some(index) = state
            .lock_surfaces
            .iter()
            .position(|entry| entry.surface.wl_surface() == surface)
        else {
            return;
        };

        let scale = scale.max(1);
        if state.lock_surfaces[index].preferred_fractional_scale == Some(scale) {
            return;
        }
        state.lock_surfaces[index].preferred_fractional_scale = Some(scale);

        let Some(previous) = state.lock_surfaces[index].size else {
            tracing::debug!(
                fractional_scale = scale,
                "lock surface fractional scale changed before configure"
            );
            return;
        };

        let size =
            state.resolve_surface_size(index, (previous.logical_width, previous.logical_height));
        if size == previous {
            return;
        }

        tracing::debug!(
            old_buffer_scale = previous.scale,
            new_buffer_scale = size.scale,
            fractional_scale = scale,
            logical_width = size.logical_width,
            logical_height = size.logical_height,
            buffer_width = size.buffer.width,
            buffer_height = size.buffer.height,
            "rerendering lock surface after fractional scale change"
        );
        state.lock_surfaces[index].size = Some(size);
        let lock_surface = state.lock_surfaces[index].surface.clone();
        if let Err(error) = state.render_surface(&lock_surface, size, qh) {
            state.failure_reason = Some(format!(
                "failed to rerender fractionally scaled curtain surface: {error:#}"
            ));
            state.exit_requested = true;
        }
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
            if surface.output == output {
                if let Some(output_power) = surface.output_power.take() {
                    output_power.destroy();
                }
                if let Some(fractional_scale) = surface.fractional_scale.take() {
                    fractional_scale.destroy();
                }
                if let Some(viewport) = surface.viewport.take() {
                    viewport.destroy();
                }
            }
        }
        let removed_index = self
            .lock_surfaces
            .iter()
            .position(|entry| entry.output == output);
        self.lock_surfaces.retain(|entry| entry.output != output);
        if let Some(removed_index) = removed_index {
            self.focused_surface_index = self.focused_surface_index.and_then(|focused| {
                if focused == removed_index {
                    None
                } else if focused > removed_index {
                    Some(focused - 1)
                } else {
                    Some(focused)
                }
            });
        }
        if self.secondary_outputs_powered_off {
            if self.set_outputs_power_mode(zwlr_output_power_v1::Mode::On) {
                tracing::info!("woke remaining locked outputs after output topology changed");
            }
            self.maybe_power_off_secondary_outputs();
        }
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
