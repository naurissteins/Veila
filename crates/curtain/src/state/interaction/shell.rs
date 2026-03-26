use smithay_client_toolkit::reexports::client::{QueueHandle, protocol::wl_surface};
use veila_ui::{ShellAction, ShellKey};

use crate::ipc::auth::submit_password;

use super::super::CurtainApp;

impl CurtainApp {
    pub(crate) fn set_keyboard_focus(&mut self, focused: bool, queue_handle: &QueueHandle<Self>) {
        if self.has_keyboard_focus == focused {
            return;
        }

        self.has_keyboard_focus = focused;
        self.ui_shell.set_focus(focused);
        self.render_all_surfaces(queue_handle);
    }

    pub(crate) fn handle_shell_key(&mut self, key: ShellKey, queue_handle: &QueueHandle<Self>) {
        if self.auth_in_flight {
            return;
        }

        let action = self.ui_shell.handle_key(key);
        if let ShellAction::Submit(secret) = action {
            let Some(socket_path) = self.daemon_socket.clone() else {
                tracing::warn!("password submitted without a daemon auth socket");
                self.ui_shell.authentication_rejected(None);
                self.render_all_surfaces(queue_handle);
                return;
            };

            let attempt_id = self.next_auth_attempt_id;
            self.next_auth_attempt_id = self.next_auth_attempt_id.saturating_add(1);
            tracing::info!(
                attempt_id,
                secret_len = secret.chars().count(),
                "submitting password attempt"
            );
            self.auth_in_flight = true;
            submit_password(socket_path, attempt_id, secret, self.auth_sender.clone());
        }
        self.render_all_surfaces(queue_handle);
    }

    pub(crate) fn handle_shell_caps_lock(
        &mut self,
        active: bool,
        queue_handle: &QueueHandle<Self>,
    ) {
        if self.ui_shell.set_caps_lock_active(active) {
            self.render_all_surfaces(queue_handle);
        }
    }

    pub(crate) fn handle_shell_keyboard_layout(
        &mut self,
        label: Option<String>,
        queue_handle: &QueueHandle<Self>,
    ) {
        if self.ui_shell.set_keyboard_layout_label(label) {
            self.render_all_surfaces(queue_handle);
        }
    }

    pub(crate) fn handle_shell_pointer_press(
        &mut self,
        surface: &wl_surface::WlSurface,
        position: (f64, f64),
        queue_handle: &QueueHandle<Self>,
    ) {
        if self.auth_in_flight {
            return;
        }

        let Some((width, height)) = self.surface_size(surface) else {
            return;
        };

        if self
            .ui_shell
            .handle_pointer_press(width as i32, height as i32, position.0, position.1)
        {
            self.render_all_surfaces(queue_handle);
        }
    }

    pub(crate) fn handle_shell_pointer_motion(
        &mut self,
        surface: &wl_surface::WlSurface,
        position: (f64, f64),
        queue_handle: &QueueHandle<Self>,
    ) {
        let Some((width, height)) = self.surface_size(surface) else {
            return;
        };

        if self
            .ui_shell
            .handle_pointer_motion(width as i32, height as i32, position.0, position.1)
        {
            self.render_all_surfaces(queue_handle);
        }
    }

    pub(crate) fn handle_shell_pointer_release(
        &mut self,
        surface: &wl_surface::WlSurface,
        position: (f64, f64),
        queue_handle: &QueueHandle<Self>,
    ) {
        if self.auth_in_flight {
            return;
        }

        let Some((width, height)) = self.surface_size(surface) else {
            return;
        };

        if self
            .ui_shell
            .handle_pointer_release(width as i32, height as i32, position.0, position.1)
        {
            self.render_all_surfaces(queue_handle);
        }
    }

    pub(crate) fn handle_shell_pointer_leave(&mut self, queue_handle: &QueueHandle<Self>) {
        if self.ui_shell.handle_pointer_leave() {
            self.render_all_surfaces(queue_handle);
        }
    }

    pub(crate) fn advance_animated_scene(&mut self, queue_handle: &QueueHandle<Self>) {
        if self.ui_shell.advance_animated_state() {
            self.render_all_surfaces(queue_handle);
        }
    }

    fn surface_size(&self, surface: &wl_surface::WlSurface) -> Option<(u32, u32)> {
        self.lock_surfaces
            .iter()
            .find(|entry| entry.surface.wl_surface() == surface)
            .and_then(|entry| entry.size)
    }
}
