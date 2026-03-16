mod render;

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
    hint_text: String,
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new(ShellTheme::default(), None)
    }
}

impl ShellState {
    pub fn new(theme: ShellTheme, user_hint: Option<String>) -> Self {
        Self {
            secret: String::new(),
            focused: false,
            status: ShellStatus::Idle,
            theme,
            hint_text: user_hint
                .filter(|hint| !hint.trim().is_empty())
                .unwrap_or_else(|| String::from("Type your password to unlock")),
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
