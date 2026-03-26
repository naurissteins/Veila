use smithay_client_toolkit::{
    reexports::client::{
        Connection, QueueHandle,
        protocol::{wl_keyboard, wl_pointer, wl_seat, wl_surface},
    },
    seat::{
        Capability, SeatHandler,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
};
use veila_ui::ShellKey;

use crate::state::CurtainApp;

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

        if capability == Capability::Pointer && self.pointer.is_none() {
            match self.seat_state.get_pointer(queue_handle, &seat) {
                Ok(pointer) => {
                    tracing::info!("pointer capability acquired");
                    self.pointer = Some(pointer);
                }
                Err(error) => {
                    self.failure_reason =
                        Some(format!("failed to acquire pointer capability: {error}"));
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

        if capability == Capability::Pointer
            && let Some(pointer) = self.pointer.take()
        {
            tracing::warn!("pointer capability removed");
            pointer.release();
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

impl PointerHandler for CurtainApp {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if !self.surface_has_focus_target(&event.surface) {
                continue;
            }

            match event.kind {
                PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                    self.handle_shell_pointer_motion(&event.surface, event.position, queue_handle);
                }
                PointerEventKind::Leave { .. } => {
                    self.handle_shell_pointer_leave(queue_handle);
                }
                PointerEventKind::Press { button, .. } if button == BTN_LEFT => {
                    self.handle_shell_pointer_press(&event.surface, event.position, queue_handle);
                }
                PointerEventKind::Release { button, .. } if button == BTN_LEFT => {
                    self.handle_shell_pointer_release(&event.surface, event.position, queue_handle);
                }
                _ => {}
            }
        }
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

const BTN_LEFT: u32 = 0x110;
