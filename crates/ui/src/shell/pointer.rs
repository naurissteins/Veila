use std::time::{Duration, Instant};

use veila_common::PowerAction;
use veila_renderer::FrameSize;

use super::{PowerConfirmation, ShellAction, ShellState};

const POWER_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(5);

impl ShellState {
    pub fn handle_pointer_motion(
        &mut self,
        frame_width: i32,
        frame_height: i32,
        x: f64,
        y: f64,
    ) -> bool {
        if matches!(self.status, super::ShellStatus::Pending { .. }) {
            let changed = self.reveal_toggle_hovered
                || self.reveal_toggle_pressed
                || self.power_button_hovered.is_some()
                || self.power_button_pressed.is_some();
            self.reveal_toggle_hovered = false;
            self.reveal_toggle_pressed = false;
            self.power_button_hovered = None;
            self.power_button_pressed = None;
            return changed;
        }

        let x = x.floor() as i32;
        let y = y.floor() as i32;
        let power_hovered = self.power_action_at(frame_width, frame_height, x, y);
        let toggle_rect = self.reveal_toggle_rect_for_frame(frame_width, frame_height);
        let hovered = self.input_visible() && power_hovered.is_none() && toggle_rect.contains(x, y);
        let changed =
            self.reveal_toggle_hovered != hovered || self.power_button_hovered != power_hovered;
        self.reveal_toggle_hovered = hovered;
        self.power_button_hovered = power_hovered;
        if !hovered && self.reveal_toggle_pressed {
            self.reveal_toggle_pressed = false;
            return true;
        }
        if self.power_button_pressed.is_some() && self.power_button_pressed != power_hovered {
            self.power_button_pressed = None;
            return true;
        }

        changed
    }

    pub fn handle_pointer_leave(&mut self) -> bool {
        let changed = self.reveal_toggle_hovered
            || self.reveal_toggle_pressed
            || self.power_button_hovered.is_some()
            || self.power_button_pressed.is_some();
        self.reveal_toggle_hovered = false;
        self.reveal_toggle_pressed = false;
        self.power_button_hovered = None;
        self.power_button_pressed = None;
        changed
    }

    pub fn handle_pointer_press(
        &mut self,
        frame_width: i32,
        frame_height: i32,
        x: f64,
        y: f64,
    ) -> bool {
        if matches!(self.status, super::ShellStatus::Pending { .. }) {
            let changed = self.reveal_toggle_hovered
                || self.reveal_toggle_pressed
                || self.power_button_hovered.is_some()
                || self.power_button_pressed.is_some();
            self.reveal_toggle_hovered = false;
            self.reveal_toggle_pressed = false;
            self.power_button_hovered = None;
            self.power_button_pressed = None;
            return changed;
        }

        let x = x.floor() as i32;
        let y = y.floor() as i32;
        let _ = self.clear_expired_power_confirmation(Instant::now());
        let power_pressed = self.power_action_at(frame_width, frame_height, x, y);
        if power_pressed.is_some() {
            let changed = self.power_button_pressed != power_pressed
                || self.power_button_hovered != power_pressed;
            self.power_button_hovered = power_pressed;
            self.power_button_pressed = power_pressed;
            self.reveal_toggle_hovered = false;
            self.reveal_toggle_pressed = false;
            return changed;
        }

        if self.reveal_auth() {
            return true;
        }

        let selection_changed = self.set_secret_selected(false);
        let toggle_rect = self.reveal_toggle_rect_for_frame(frame_width, frame_height);
        let pressed = toggle_rect.contains(x, y);
        let changed = selection_changed
            || self.reveal_toggle_pressed != pressed
            || self.reveal_toggle_hovered != pressed
            || self.power_button_hovered.is_some()
            || self.power_button_pressed.is_some();
        self.reveal_toggle_hovered = pressed;
        self.reveal_toggle_pressed = pressed;
        self.power_button_hovered = None;
        self.power_button_pressed = None;
        changed
    }

    pub fn handle_pointer_release(
        &mut self,
        frame_width: i32,
        frame_height: i32,
        x: f64,
        y: f64,
    ) -> bool {
        if matches!(self.status, super::ShellStatus::Pending { .. }) {
            let changed = self.reveal_toggle_hovered
                || self.reveal_toggle_pressed
                || self.power_button_hovered.is_some()
                || self.power_button_pressed.is_some();
            self.reveal_toggle_hovered = false;
            self.reveal_toggle_pressed = false;
            self.power_button_hovered = None;
            self.power_button_pressed = None;
            return changed;
        }

        let x = x.floor() as i32;
        let y = y.floor() as i32;
        let _ = self.clear_expired_power_confirmation(Instant::now());
        let power_hovered = self.power_action_at(frame_width, frame_height, x, y);
        let power_clicked = self
            .power_button_pressed
            .and_then(|pressed| (Some(pressed) == power_hovered).then_some(pressed));
        if let Some(action) = power_clicked {
            self.activate_power_button(action);
            self.power_button_pressed = None;
            self.power_button_hovered = power_hovered;
            self.reveal_toggle_hovered = false;
            self.reveal_toggle_pressed = false;
            return true;
        }

        if self.reveal_auth() {
            return true;
        }

        let toggle_rect = self.reveal_toggle_rect_for_frame(frame_width, frame_height);
        let hovered = self.input_visible() && power_hovered.is_none() && toggle_rect.contains(x, y);
        let toggled = self.reveal_toggle_pressed && hovered;
        let changed = self.reveal_toggle_pressed
            || self.reveal_toggle_hovered != hovered
            || self.power_button_hovered != power_hovered
            || self.power_button_pressed.is_some()
            || toggled;
        self.reveal_toggle_pressed = false;
        self.reveal_toggle_hovered = hovered;
        self.power_button_hovered = power_hovered;
        self.power_button_pressed = None;
        if toggled {
            self.reveal_secret = !self.reveal_secret;
        }

        changed
    }

    pub fn take_pointer_action(&mut self) -> ShellAction {
        self.requested_power_action
            .take()
            .map_or(ShellAction::None, ShellAction::Power)
    }

    pub fn power_button_interaction_state(
        &self,
    ) -> (
        Option<PowerAction>,
        Option<PowerAction>,
        Option<PowerAction>,
    ) {
        (
            self.power_button_hovered,
            self.power_button_pressed,
            self.power_confirmation_action(),
        )
    }

    pub(super) fn power_confirmation_action(&self) -> Option<PowerAction> {
        self.power_confirmation
            .filter(|confirmation| Instant::now() <= confirmation.expires_at)
            .map(|confirmation| confirmation.action)
    }

    fn power_action_at(
        &self,
        frame_width: i32,
        frame_height: i32,
        x: i32,
        y: i32,
    ) -> Option<PowerAction> {
        let size = FrameSize::new(frame_width.max(0) as u32, frame_height.max(0) as u32);
        self.theme.power_buttons.iter().find_map(|button| {
            let rect = self.power_button_rect(size, button.action)?;
            rect.contains(x, y).then_some(button.action)
        })
    }

    fn activate_power_button(&mut self, action: PowerAction) {
        let now = Instant::now();
        let Some(button) = self
            .theme
            .power_buttons
            .iter()
            .find(|button| button.action == action && button.enabled)
        else {
            return;
        };

        if !button.confirm || self.power_confirmation_action() == Some(action) {
            self.power_confirmation = None;
            self.requested_power_action = Some(action);
            return;
        }

        self.power_confirmation = Some(PowerConfirmation {
            action,
            expires_at: now + POWER_CONFIRMATION_TIMEOUT,
        });
    }

    pub(super) fn clear_expired_power_confirmation(&mut self, now: Instant) -> bool {
        if self
            .power_confirmation
            .is_some_and(|confirmation| now > confirmation.expires_at)
        {
            self.power_confirmation = None;
            return true;
        }
        false
    }
}
