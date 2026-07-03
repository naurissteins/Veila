use std::time::Duration;

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

use crate::{ipc::auth::notify_activity, state::CurtainApp};

const RESUME_INPUT_GRACE_PERIOD: Duration = Duration::from_millis(1000);

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
            self.note_surface_activity(surface, queue_handle);
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
            self.stop_backspace_repeat();
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
        self.drain_control_events(queue_handle);
        if self.wake_key_release_pending {
            return;
        }
        if self.resume_input.grace_period_active() {
            return;
        }
        if self.has_keyboard_focus
            && let Some(socket_path) = self.daemon_socket_path()
        {
            notify_activity(socket_path);
        }
        if self.handle_lock_activity(queue_handle) {
            self.wake_key_release_pending = true;
            self.resume_input.clear_swallow_input();
            self.stop_backspace_repeat();
            return;
        }
        self.record_visible_lock_activity();
        if self.resume_input.swallow_input_pending() {
            self.resume_input.clear_swallow_input();
            self.resume_input
                .begin_grace_period(RESUME_INPUT_GRACE_PERIOD);
            self.wake_key_release_pending = true;
            self.stop_backspace_repeat();
            return;
        }
        if event.keysym == Keysym::BackSpace {
            self.start_backspace_repeat();
        }
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
        self.drain_control_events(queue_handle);
        if self.wake_key_release_pending {
            return;
        }
        if self.resume_input.grace_period_active() {
            return;
        }
        self.record_visible_lock_activity();
        if event.keysym == Keysym::BackSpace {
            return;
        }
        handle_key_event(self, queue_handle, event);
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        queue_handle: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.drain_control_events(queue_handle);
        if self.wake_key_release_pending {
            self.wake_key_release_pending = false;
            self.stop_backspace_repeat();
            return;
        }
        if event.keysym == Keysym::BackSpace {
            self.stop_backspace_repeat();
        }
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
        self.ctrl_active = modifiers.ctrl;
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
        self.drain_control_events(queue_handle);
        for event in events {
            if !self.surface_has_focus_target(&event.surface) {
                continue;
            }
            self.note_surface_activity(&event.surface, queue_handle);

            let outputs_powered_off = self.outputs_powered_off();
            if outputs_powered_off && !matches!(event.kind, PointerEventKind::Press { .. }) {
                if matches!(event.kind, PointerEventKind::Release { .. })
                    && self.wake_pointer_release_pending
                {
                    self.wake_pointer_release_pending = false;
                }
                continue;
            }

            match event.kind {
                PointerEventKind::Enter { .. } => {
                    if self.resume_input.swallow_input_pending() {
                        continue;
                    }
                    self.record_visible_lock_activity();
                    self.set_configured_pointer_cursor(_conn);
                    self.handle_shell_pointer_motion(&event.surface, event.position, queue_handle);
                }
                PointerEventKind::Motion { .. } => {
                    if self.resume_input.swallow_input_pending() {
                        continue;
                    }
                    self.record_visible_lock_activity();
                    self.handle_shell_pointer_motion(&event.surface, event.position, queue_handle);
                }
                PointerEventKind::Leave { .. } => {
                    if self.resume_input.swallow_input_pending() {
                        continue;
                    }
                    self.handle_shell_pointer_leave(queue_handle);
                }
                PointerEventKind::Press { button, .. } => {
                    if self.wake_pointer_release_pending {
                        continue;
                    }
                    if self.resume_input.grace_period_active() {
                        continue;
                    }
                    if let Some(socket_path) = self.daemon_socket_path() {
                        notify_activity(socket_path);
                    }
                    if self.handle_lock_activity(queue_handle) {
                        self.wake_pointer_release_pending = true;
                        self.resume_input.clear_swallow_input();
                        continue;
                    }
                    self.record_visible_lock_activity();
                    if self.resume_input.swallow_input_pending() {
                        self.resume_input.clear_swallow_input();
                        self.resume_input
                            .begin_grace_period(RESUME_INPUT_GRACE_PERIOD);
                        self.wake_pointer_release_pending = true;
                        continue;
                    }
                    if button == BTN_LEFT {
                        self.handle_shell_pointer_press(
                            &event.surface,
                            event.position,
                            queue_handle,
                        );
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    self.record_visible_lock_activity();
                    if self.wake_pointer_release_pending {
                        self.wake_pointer_release_pending = false;
                        continue;
                    }
                    if button == BTN_LEFT {
                        self.handle_shell_pointer_release(
                            &event.surface,
                            event.position,
                            queue_handle,
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

impl CurtainApp {
    pub(crate) fn set_configured_pointer_cursor(&self, connection: &Connection) {
        let Some(pointer) = self.pointer.as_ref() else {
            return;
        };

        let result = if self.hide_cursor {
            pointer.hide_cursor()
        } else {
            pointer.set_cursor(connection, CursorIcon::Default)
        };

        if let Err(error) = result {
            tracing::debug!(hide_cursor = self.hide_cursor, %error, "failed to set pointer cursor");
        }
    }
}

fn handle_key_event(app: &mut CurtainApp, queue_handle: &QueueHandle<CurtainApp>, event: KeyEvent) {
    if !app.has_keyboard_focus {
        return;
    }

    if app.ctrl_active {
        match event.keysym {
            Keysym::a | Keysym::A => {
                app.handle_shell_key(ShellKey::SelectAll, queue_handle);
                return;
            }
            Keysym::u | Keysym::U => {
                app.handle_shell_key(ShellKey::Clear, queue_handle);
                return;
            }
            _ => {}
        }
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
    parse_keymap_layout_labels_from_str(&keymap.as_string())
}

fn parse_keymap_layout_labels_from_str(keymap: &str) -> Vec<String> {
    let mut labels_by_group = Vec::new();
    let mut fallback_labels = Vec::new();

    for line in keymap.lines().map(str::trim_start) {
        if !line.starts_with("name[") {
            continue;
        }

        let Some((group, value)) = line.split_once('=') else {
            continue;
        };
        let Some(name) = parse_xkb_quoted_string(value) else {
            continue;
        };
        let label = short_layout_label(&name);
        if label.is_empty() {
            continue;
        }

        if let Some(index) = parse_xkb_layout_group_index(group) {
            if labels_by_group.len() <= index {
                labels_by_group.resize(index + 1, None);
            }
            labels_by_group[index] = Some(label);
        } else {
            fallback_labels.push(label);
        }
    }

    let labels: Vec<String> = labels_by_group.into_iter().flatten().collect();
    if labels.is_empty() {
        fallback_labels
    } else {
        labels
    }
}

fn parse_xkb_layout_group_index(group: &str) -> Option<usize> {
    let start = group.find('[')? + 1;
    let end = group[start..].find(']')? + start;
    let group = group[start..end].trim().to_ascii_lowercase();
    let number = group.strip_prefix("group")?.parse::<usize>().ok()?;
    number.checked_sub(1)
}

fn parse_xkb_quoted_string(value: &str) -> Option<String> {
    let mut characters = value.trim_start().chars();
    if characters.next()? != '"' {
        return None;
    }

    let mut parsed = String::new();
    let mut escaped = false;
    for character in characters {
        if escaped {
            parsed.push(character);
            escaped = false;
            continue;
        }

        match character {
            '\\' => escaped = true,
            '"' => return Some(parsed),
            _ => parsed.push(character),
        }
    }

    None
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
    use super::{
        parse_keymap_layout_labels_from_str, parse_xkb_layout_group_index, parse_xkb_quoted_string,
        short_layout_label,
    };

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

    #[test]
    fn parses_xkb_layout_group_indices() {
        assert_eq!(parse_xkb_layout_group_index("name[group1]"), Some(0));
        assert_eq!(parse_xkb_layout_group_index("name[Group2]"), Some(1));
        assert_eq!(parse_xkb_layout_group_index("name[group0]"), None);
        assert_eq!(parse_xkb_layout_group_index("name[foo]"), None);
    }

    #[test]
    fn parses_xkb_quoted_strings() {
        assert_eq!(
            parse_xkb_quoted_string("\"English (US)\";"),
            Some(String::from("English (US)"))
        );
        assert_eq!(
            parse_xkb_quoted_string("\"Custom \\\"Graphre\\\"\";"),
            Some(String::from("Custom \"Graphre\""))
        );
        assert_eq!(parse_xkb_quoted_string("English;"), None);
    }

    #[test]
    fn extracts_layout_labels_without_recompiling_keymap() {
        let keymap = r#"
            xkb_keymap {
                xkb_symbols "(unnamed)" {
                    name[group1]="English (US)";
                    name[group2]="graphre";
                };
            };
        "#;

        assert_eq!(parse_keymap_layout_labels_from_str(keymap), ["EN", "GRA"]);
    }

    #[test]
    fn ignores_malformed_layout_names() {
        let keymap = r#"
            xkb_keymap {
                xkb_symbols "(unnamed)" {
                    name[group1]=English;
                    name[group2]="Portuguese-Brazil";
                };
            };
        "#;

        assert_eq!(parse_keymap_layout_labels_from_str(keymap), ["POR"]);
    }
}
