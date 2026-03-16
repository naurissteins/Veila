use kwylock_renderer::{ClearColor, SoftwareBuffer};

const BACKGROUND: ClearColor = ClearColor::opaque(8, 12, 20);
const PANEL: ClearColor = ClearColor::opaque(22, 28, 38);
const PANEL_BORDER: ClearColor = ClearColor::opaque(74, 86, 110);
const INPUT: ClearColor = ClearColor::opaque(13, 18, 28);
const INPUT_BORDER: ClearColor = ClearColor::opaque(92, 108, 146);
const BULLET: ClearColor = ClearColor::opaque(240, 244, 250);
const PLACEHOLDER: ClearColor = ClearColor::opaque(68, 78, 102);
const FOCUS: ClearColor = ClearColor::opaque(116, 161, 255);
const SUBMIT: ClearColor = ClearColor::opaque(255, 194, 92);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellAction {
    None,
    Submit(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKey {
    Character(char),
    Backspace,
    Enter,
    Escape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellStatus {
    Idle,
    Submitted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellState {
    secret: String,
    focused: bool,
    status: ShellStatus,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            secret: String::new(),
            focused: false,
            status: ShellStatus::Idle,
        }
    }
}

impl ShellState {
    pub fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn handle_key(&mut self, key: ShellKey) -> ShellAction {
        match key {
            ShellKey::Character(character) => {
                if !character.is_control() && self.secret.chars().count() < 128 {
                    self.secret.push(character);
                    self.status = ShellStatus::Idle;
                }
                ShellAction::None
            }
            ShellKey::Backspace => {
                self.secret.pop();
                self.status = ShellStatus::Idle;
                ShellAction::None
            }
            ShellKey::Escape => {
                self.secret.clear();
                self.status = ShellStatus::Idle;
                ShellAction::None
            }
            ShellKey::Enter => {
                self.status = ShellStatus::Submitted;
                if self.secret.is_empty() {
                    ShellAction::None
                } else {
                    ShellAction::Submit(self.secret.clone())
                }
            }
        }
    }

    pub fn render(&self, buffer: &mut SoftwareBuffer) {
        buffer.clear(BACKGROUND);

        let size = buffer.size();
        let width = size.width as i32;
        let height = size.height as i32;
        let panel_width = ((width * 3) / 5).clamp(320, 560);
        let panel_height = 176;
        let panel_x = (width - panel_width) / 2;
        let panel_y = (height - panel_height) / 2;
        let accent = match self.status {
            ShellStatus::Idle => {
                if self.focused {
                    FOCUS
                } else {
                    INPUT_BORDER
                }
            }
            ShellStatus::Submitted => SUBMIT,
        };

        fill_rect(buffer, panel_x, panel_y, panel_width, panel_height, PANEL);
        stroke_rect(
            buffer,
            panel_x,
            panel_y,
            panel_width,
            panel_height,
            2,
            PANEL_BORDER,
        );
        fill_rect(buffer, panel_x, panel_y, panel_width, 6, accent);

        let input_x = panel_x + 32;
        let input_y = panel_y + 82;
        let input_width = panel_width - 64;
        let input_height = 38;

        fill_rect(buffer, input_x, input_y, input_width, input_height, INPUT);
        stroke_rect(
            buffer,
            input_x,
            input_y,
            input_width,
            input_height,
            2,
            if self.focused { accent } else { INPUT_BORDER },
        );

        let indicator_y = input_y + input_height + 24;
        fill_rect(
            buffer,
            panel_x + 32,
            indicator_y,
            panel_width - 64,
            6,
            PLACEHOLDER,
        );
        fill_rect(
            buffer,
            panel_x + 32,
            indicator_y,
            (panel_width - 64) / 3,
            6,
            accent,
        );

        self.draw_secret(buffer, input_x, input_y, input_width, input_height, accent);
    }

    fn draw_secret(
        &self,
        buffer: &mut SoftwareBuffer,
        input_x: i32,
        input_y: i32,
        input_width: i32,
        input_height: i32,
        accent: ClearColor,
    ) {
        if self.secret.is_empty() {
            fill_rect(
                buffer,
                input_x + 20,
                input_y + (input_height / 2) - 2,
                input_width / 3,
                4,
                PLACEHOLDER,
            );

            if self.focused {
                fill_rect(
                    buffer,
                    input_x + 20,
                    input_y + 9,
                    3,
                    input_height - 18,
                    accent,
                );
            }

            return;
        }

        let bullet_size = 10;
        let spacing = 16;
        let visible = ((input_width - 40) / spacing).max(1) as usize;
        let bullet_count = self.secret.chars().count().min(visible);
        let row_width = (bullet_count as i32 * bullet_size)
            + ((bullet_count.saturating_sub(1)) as i32 * (spacing - bullet_size));
        let start_x = input_x + ((input_width - row_width) / 2).max(18);
        let bullet_y = input_y + (input_height - bullet_size) / 2;

        for index in 0..bullet_count {
            let x = start_x + index as i32 * spacing;
            fill_rect(buffer, x, bullet_y, bullet_size, bullet_size, BULLET);
        }

        if self.focused {
            let cursor_x = start_x + bullet_count as i32 * spacing + 4;
            fill_rect(buffer, cursor_x, input_y + 8, 3, input_height - 16, accent);
        }
    }
}

fn fill_rect(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: ClearColor,
) {
    let size = buffer.size();
    let right = (x + width).clamp(0, size.width as i32);
    let bottom = (y + height).clamp(0, size.height as i32);
    let left = x.clamp(0, size.width as i32);
    let top = y.clamp(0, size.height as i32);

    if left >= right || top >= bottom {
        return;
    }

    let stride = size.width as usize * 4;
    let pixel = color.to_argb8888_bytes();
    let pixels = buffer.pixels_mut();

    for row in top as usize..bottom as usize {
        let row_start = row * stride;
        for column in left as usize..right as usize {
            let offset = row_start + column * 4;
            pixels[offset..offset + 4].copy_from_slice(&pixel);
        }
    }
}

fn stroke_rect(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    thickness: i32,
    color: ClearColor,
) {
    fill_rect(buffer, x, y, width, thickness, color);
    fill_rect(buffer, x, y + height - thickness, width, thickness, color);
    fill_rect(buffer, x, y, thickness, height, color);
    fill_rect(buffer, x + width - thickness, y, thickness, height, color);
}

#[cfg(test)]
mod tests {
    use kwylock_renderer::{FrameSize, SoftwareBuffer};

    use super::{ShellAction, ShellKey, ShellState};

    #[test]
    fn edits_and_submits_password_text() {
        let mut shell = ShellState::default();

        assert_eq!(
            shell.handle_key(ShellKey::Character('a')),
            ShellAction::None
        );
        assert_eq!(
            shell.handle_key(ShellKey::Character('b')),
            ShellAction::None
        );
        assert_eq!(
            shell.handle_key(ShellKey::Enter),
            ShellAction::Submit(String::from("ab"))
        );
        assert_eq!(shell.handle_key(ShellKey::Backspace), ShellAction::None);
        assert_eq!(
            shell.handle_key(ShellKey::Enter),
            ShellAction::Submit(String::from("a"))
        );
    }

    #[test]
    fn renders_non_empty_scene() {
        let mut shell = ShellState::default();
        shell.set_focus(true);
        let mut buffer = SoftwareBuffer::new(FrameSize::new(480, 320)).expect("buffer");
        shell.render(&mut buffer);

        assert!(buffer.pixels().iter().any(|byte| *byte != 0));
    }
}
