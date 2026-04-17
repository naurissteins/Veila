use smithay_client_toolkit::{
    reexports::client::{
        Connection, QueueHandle,
        protocol::{wl_keyboard, wl_pointer, wl_seat, wl_surface},
    },
    seat::{
        Capability, SeatHandler,
        keyboard::{KeyEvent, KeyboardHandler, Keymap, Keysym, Modifiers, RawModifiers},
        pointer::{CursorIcon, PointerEvent, PointerEventKind, PointerHandler, ThemeSpec},
    },
};
use veila_ui::ShellKey;
use xkbcommon::xkb;

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
            let cursor_surface = self.compositor_state.create_surface(queue_handle);
            match self.seat_state.get_pointer_with_theme(
                queue_handle,
                &seat,
                self.shm.wl_shm(),
                cursor_surface,
                ThemeSpec::default(),
            ) {
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

        if capability == Capability::Pointer && self.pointer.take().is_some() {
            tracing::warn!("pointer capability removed");
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

    fn update_keymap(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        keymap: Keymap<'_>,
    ) {
        let labels = parse_keymap_layout_labels(keymap);
        self.keyboard_layout_labels = labels;
        self.handle_shell_keyboard_layout(active_layout_label(self), qh);
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        layout: u32,
    ) {
        self.active_keyboard_layout = layout;
        self.handle_shell_caps_lock(modifiers.caps_lock, qh);
        self.handle_shell_keyboard_layout(active_layout_label(self), qh);
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
                PointerEventKind::Enter { .. } => {
                    self.set_default_pointer_cursor(_conn);
                    self.handle_shell_pointer_motion(&event.surface, event.position, queue_handle);
                }
                PointerEventKind::Motion { .. } => {
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

impl CurtainApp {
    fn set_default_pointer_cursor(&mut self, connection: &Connection) {
        let Some(pointer) = self.pointer.as_ref() else {
            return;
        };

        if let Err(error) = pointer.set_cursor(connection, CursorIcon::Default) {
            tracing::debug!(%error, "failed to set default pointer cursor");
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

fn active_layout_label(app: &CurtainApp) -> Option<String> {
    app.keyboard_layout_labels
        .get(app.active_keyboard_layout as usize)
        .cloned()
        .or_else(|| app.keyboard_layout_labels.first().cloned())
}

fn parse_keymap_layout_labels(keymap: Keymap<'_>) -> Vec<String> {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let Some(keymap) = xkb::Keymap::new_from_string(
        &context,
        keymap.as_string(),
        xkb::KEYMAP_FORMAT_TEXT_V1,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    ) else {
        tracing::warn!("failed to parse keyboard keymap for layout indicator");
        return Vec::new();
    };

    keymap
        .layouts()
        .map(short_layout_label)
        .filter(|label| !label.is_empty())
        .collect()
}

fn short_layout_label(name: &str) -> String {
    let normalized = name.trim().to_ascii_lowercase();
    let token = normalized
        .split(|character: char| !character.is_ascii_alphanumeric())
        .find(|token| !token.is_empty())
        .unwrap_or("");

    if token.is_empty() {
        return String::new();
    }

    match token {
        "us" | "gb" | "uk" | "eng" | "english" => return String::from("EN"),
        "lv" | "latvian" | "latvia" => return String::from("LV"),
        "ru" | "russian" | "russia" => return String::from("RU"),
        _ => {}
    }

    if token.len() <= 3 {
        return token.to_ascii_uppercase();
    }

    token
        .chars()
        .filter(|character| character.is_ascii_alphabetic())
        .take(3)
        .collect::<String>()
        .to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::short_layout_label;

    #[test]
    fn normalizes_common_layout_codes() {
        assert_eq!(short_layout_label("us"), "EN");
        assert_eq!(short_layout_label("lv"), "LV");
        assert_eq!(short_layout_label("ru"), "RU");
    }

    #[test]
    fn normalizes_longer_layout_names() {
        assert_eq!(short_layout_label("English (US)"), "EN");
        assert_eq!(short_layout_label("latvian"), "LV");
        assert_eq!(short_layout_label("Portuguese-Brazil"), "POR");
    }
}
