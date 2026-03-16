use kwylock_renderer::{ClearColor, SoftwareBuffer};

use crate::ShellTheme;

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
    Pending,
    Rejected { retry_after_ms: Option<u64> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellState {
    secret: String,
    focused: bool,
    status: ShellStatus,
    theme: ShellTheme,
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new(ShellTheme::default())
    }
}

impl ShellState {
    pub fn new(theme: ShellTheme) -> Self {
        Self {
            secret: String::new(),
            focused: false,
            status: ShellStatus::Idle,
            theme,
        }
    }

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
                if self.secret.is_empty() {
                    ShellAction::None
                } else {
                    self.status = ShellStatus::Pending;
                    ShellAction::Submit(self.secret.clone())
                }
            }
        }
    }

    pub fn authentication_busy(&mut self) {
        self.status = ShellStatus::Idle;
    }

    pub fn authentication_rejected(&mut self, retry_after_ms: Option<u64>) {
        self.secret.clear();
        self.status = ShellStatus::Rejected { retry_after_ms };
    }

    pub fn render(&self, buffer: &mut SoftwareBuffer) {
        buffer.clear(self.theme.background);
        self.render_overlay(buffer);
    }

    pub fn render_overlay(&self, buffer: &mut SoftwareBuffer) {
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
                    self.theme.focus
                } else {
                    self.theme.input_border
                }
            }
            ShellStatus::Pending => self.theme.pending,
            ShellStatus::Rejected { .. } => self.theme.rejected,
        };

        fill_rect(
            buffer,
            panel_x,
            panel_y,
            panel_width,
            panel_height,
            self.theme.panel,
        );
        stroke_rect(
            buffer,
            panel_x,
            panel_y,
            panel_width,
            panel_height,
            2,
            self.theme.panel_border,
        );
        fill_rect(buffer, panel_x, panel_y, panel_width, 6, accent);

        let input_x = panel_x + 32;
        let input_y = panel_y + 82;
        let input_width = panel_width - 64;
        let input_height = 38;

        fill_rect(
            buffer,
            input_x,
            input_y,
            input_width,
            input_height,
            self.theme.input,
        );
        stroke_rect(
            buffer,
            input_x,
            input_y,
            input_width,
            input_height,
            2,
            if self.focused {
                accent
            } else {
                self.theme.input_border
            },
        );

        let indicator_y = input_y + input_height + 24;
        fill_rect(
            buffer,
            panel_x + 32,
            indicator_y,
            panel_width - 64,
            6,
            self.theme.muted,
        );
        fill_rect(
            buffer,
            panel_x + 32,
            indicator_y,
            indicator_width(panel_width, self.status),
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
                self.theme.muted,
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
            fill_rect(
                buffer,
                x,
                bullet_y,
                bullet_size,
                bullet_size,
                self.theme.foreground,
            );
        }

        if self.focused {
            let cursor_x = start_x + bullet_count as i32 * spacing + 4;
            fill_rect(buffer, cursor_x, input_y + 8, 3, input_height - 16, accent);
        }
    }
}

fn indicator_width(panel_width: i32, status: ShellStatus) -> i32 {
    match status {
        ShellStatus::Idle => (panel_width - 64) / 3,
        ShellStatus::Pending => (panel_width - 64) / 2,
        ShellStatus::Rejected { retry_after_ms } => {
            if retry_after_ms.unwrap_or_default() > 0 {
                panel_width - 64
            } else {
                ((panel_width - 64) * 2) / 3
            }
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
    fn rejection_clears_secret() {
        let mut shell = ShellState::default();
        shell.handle_key(ShellKey::Character('a'));
        shell.authentication_rejected(Some(1_000));

        assert_eq!(shell.handle_key(ShellKey::Enter), ShellAction::None);
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
