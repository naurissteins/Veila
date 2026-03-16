use kwylock_ui::ShellKey;
use smithay_client_toolkit::{
    compositor::CompositorHandler,
    output::{OutputHandler, OutputState},
    reexports::client::{
        Connection, Proxy, QueueHandle,
        protocol::{wl_buffer, wl_keyboard, wl_output, wl_seat, wl_surface},
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
    },
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockSurface, SessionLockSurfaceConfigure,
    },
    shm::{Shm, ShmHandler},
};

use crate::state::CurtainApp;

impl SessionLockHandler for CurtainApp {
    fn locked(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _session_lock: SessionLock) {
        tracing::info!("session lock confirmed by compositor");
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

impl SeatHandler for CurtainApp {
    fn seat_state(&mut self) -> &mut smithay_client_toolkit::seat::SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            match self.seat_state.get_keyboard(queue_handle, &seat, None) {
                Ok(keyboard) => {
                    tracing::info!("keyboard capability acquired");
                    self.keyboard = Some(keyboard);
                }
                Err(error) => {
                    self.failure_reason =
                        Some(format!("failed to acquire keyboard capability: {error}"));
                    self.exit_requested = true;
                }
            }
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _queue_handle: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard
            && let Some(keyboard) = self.keyboard.take()
        {
            tracing::warn!("keyboard capability removed");
            keyboard.release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for CurtainApp {
    fn enter(
        &mut self,
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        if self.surface_has_focus_target(surface) {
            self.set_keyboard_focus(true, queue_handle);
        }
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
        if self.surface_has_focus_target(surface) {
            self.set_keyboard_focus(false, queue_handle);
        }
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        handle_key_event(self, queue_handle, event);
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        handle_key_event(self, queue_handle, event);
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _queue_handle: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
    }
}

impl ProvidesRegistryState for CurtainApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState, SeatState];
}

impl ShmHandler for CurtainApp {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

fn handle_key_event(app: &mut CurtainApp, queue_handle: &QueueHandle<CurtainApp>, event: KeyEvent) {
    if !app.has_keyboard_focus {
        return;
    }

    match event.keysym {
        Keysym::BackSpace => app.handle_shell_key(ShellKey::Backspace, queue_handle),
        Keysym::Return | Keysym::KP_Enter => app.handle_shell_key(ShellKey::Enter, queue_handle),
        Keysym::Escape => app.handle_shell_key(ShellKey::Escape, queue_handle),
        _ => {
            if let Some(text) = event.utf8 {
                for character in text.chars().filter(|character| !character.is_control()) {
                    app.handle_shell_key(ShellKey::Character(character), queue_handle);
                }
            }
        }
    }
}

smithay_client_toolkit::delegate_compositor!(CurtainApp);
smithay_client_toolkit::delegate_keyboard!(CurtainApp);
smithay_client_toolkit::delegate_output!(CurtainApp);
smithay_client_toolkit::delegate_registry!(CurtainApp);
smithay_client_toolkit::delegate_seat!(CurtainApp);
smithay_client_toolkit::delegate_session_lock!(CurtainApp);
smithay_client_toolkit::delegate_shm!(CurtainApp);
smithay_client_toolkit::reexports::client::delegate_noop!(CurtainApp: ignore wl_buffer::WlBuffer);
