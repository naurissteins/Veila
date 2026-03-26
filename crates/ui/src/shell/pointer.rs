use super::ShellState;

impl ShellState {
    pub fn handle_pointer_motion(
        &mut self,
        frame_width: i32,
        frame_height: i32,
        x: f64,
        y: f64,
    ) -> bool {
        let x = x.floor() as i32;
        let y = y.floor() as i32;
        let toggle_rect = self.reveal_toggle_rect_for_frame(frame_width, frame_height);
        let hovered = toggle_rect.contains(x, y);
        let changed = self.reveal_toggle_hovered != hovered;
        self.reveal_toggle_hovered = hovered;
        if !hovered && self.reveal_toggle_pressed {
            self.reveal_toggle_pressed = false;
            return true;
        }

        changed
    }

    pub fn handle_pointer_leave(&mut self) -> bool {
        let changed = self.reveal_toggle_hovered || self.reveal_toggle_pressed;
        self.reveal_toggle_hovered = false;
        self.reveal_toggle_pressed = false;
        changed
    }

    pub fn handle_pointer_press(
        &mut self,
        frame_width: i32,
        frame_height: i32,
        x: f64,
        y: f64,
    ) -> bool {
        let x = x.floor() as i32;
        let y = y.floor() as i32;
        let toggle_rect = self.reveal_toggle_rect_for_frame(frame_width, frame_height);
        let pressed = toggle_rect.contains(x, y);
        let changed =
            self.reveal_toggle_pressed != pressed || self.reveal_toggle_hovered != pressed;
        self.reveal_toggle_hovered = pressed;
        self.reveal_toggle_pressed = pressed;
        changed
    }

    pub fn handle_pointer_release(
        &mut self,
        frame_width: i32,
        frame_height: i32,
        x: f64,
        y: f64,
    ) -> bool {
        let x = x.floor() as i32;
        let y = y.floor() as i32;
        let toggle_rect = self.reveal_toggle_rect_for_frame(frame_width, frame_height);
        let hovered = toggle_rect.contains(x, y);
        let toggled = self.reveal_toggle_pressed && hovered;
        let changed =
            self.reveal_toggle_pressed || self.reveal_toggle_hovered != hovered || toggled;
        self.reveal_toggle_pressed = false;
        self.reveal_toggle_hovered = hovered;
        if toggled {
            self.reveal_secret = !self.reveal_secret;
        }

        changed
    }
}
